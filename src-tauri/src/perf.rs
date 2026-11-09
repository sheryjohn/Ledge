//! Ledge Performance panel: samples system vitals with `sysinfo` and pushes them to the
//! frontend as `perf-snapshot` events.
//!
//! While a notch window reports the panel open (`set_perf_active`) it takes a full sample,
//! process list included, every `nectar-perf-interval-ms`. Otherwise it takes a light sample
//! (no process scan) every few seconds, so the sparklines already have history on reveal.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};

pub const EVENT: &str = "perf-snapshot";

const DEFAULT_INTERVAL_MS: u64 = 1000;
/// Light sampling cadence while no panel is open.
const IDLE_INTERVAL: Duration = Duration::from_secs(5);
const DEFAULT_TOP_N: usize = 8;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    /// Percent of the whole machine, 0 to 100, like Task Manager.
    pub cpu: f32,
    /// Private working set in bytes (Task Manager's Memory column), or the full working set
    /// when the private figure could not be read.
    pub mem: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub free: u64,
    pub read_bps: u64,
    pub write_bps: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PerfSnapshot {
    pub ts_ms: u64,
    pub cpu_total: f32,
    pub cpu_cores: Vec<f32>,
    pub mem_used: u64,
    pub mem_total: u64,
    /// Committed memory and the commit limit, as on Task Manager's Memory page.
    pub commit_used: u64,
    pub commit_total: u64,
    pub disks: Vec<DiskInfo>,
    pub disk_read_bps: u64,
    pub disk_write_bps: u64,
    /// Bytes per second over physical adapters only. The panel shows bits per second.
    pub net_rx_bps: u64,
    pub net_tx_bps: u64,
    /// False for light samples; the process fields are then empty and should be ignored.
    pub has_processes: bool,
    pub process_count: usize,
    pub top_cpu: Vec<ProcInfo>,
    pub top_mem: Vec<ProcInfo>,
}

// ---- Pure helpers (unit tested) ----

/// Bytes counted over `elapsed`, as bytes per second.
pub fn per_second(bytes: u64, elapsed: Duration) -> u64 {
    let ms = elapsed.as_millis() as u64;
    if ms == 0 {
        return 0;
    }
    bytes.saturating_mul(1000) / ms
}

/// sysinfo reports process CPU as percent of one core (up to 100 x cores on Windows).
/// Task Manager shows percent of the whole machine.
pub fn normalize_cpu(per_core_percent: f32, cores: usize) -> f32 {
    if cores == 0 {
        return 0.0;
    }
    (per_core_percent / cores as f32).clamp(0.0, 100.0)
}

/// The `n` biggest entries by `key`, biggest first. Ties keep a stable order by PID.
pub fn top_n_by<F: Fn(&ProcInfo) -> f64>(procs: &[ProcInfo], n: usize, key: F) -> Vec<ProcInfo> {
    let mut sorted: Vec<&ProcInfo> = procs.iter().collect();
    sorted.sort_by(|a, b| {
        key(b)
            .partial_cmp(&key(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pid.cmp(&b.pid))
    });
    sorted.into_iter().take(n).cloned().collect()
}

pub fn parse_interval_ms(v: Option<&str>) -> u64 {
    v.and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_MS)
        .clamp(500, 5000)
}

pub fn parse_top_n(v: Option<&str>) -> usize {
    v.and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(DEFAULT_TOP_N)
        .clamp(3, 20)
}

// ---- Activation ----

fn active_windows() -> &'static Mutex<HashSet<String>> {
    static S: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
static WORKER: OnceLock<std::thread::Thread> = OnceLock::new();

/// A notch window says whether its Performance panel is on screen.
#[tauri::command]
pub fn set_perf_active(window: tauri::Window, active: bool) {
    let any = if let Ok(mut set) = active_windows().lock() {
        if active {
            set.insert(window.label().to_string());
        } else {
            set.remove(window.label());
        }
        !set.is_empty()
    } else {
        active
    };
    let was = ACTIVE.swap(any, Ordering::Relaxed);
    if any && !was {
        if let Some(t) = WORKER.get() {
            t.unpark();
        }
    }
}

// ---- Sampler ----

struct Sampler {
    sys: System,
    disks: Disks,
    networks: Networks,
    last: Instant,
    cpu_util: Option<crate::winperf::CpuUtility>,
    last_net: Option<(u64, u64)>,
}

impl Sampler {
    fn new() -> Self {
        let mut sys = System::new();
        // CPU usage needs two refreshes with a gap; prime it here.
        sys.refresh_cpu_usage();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        Sampler {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last: Instant::now(),
            cpu_util: crate::winperf::CpuUtility::open(),
            last_net: crate::winperf::physical_net_octets(),
        }
    }

    fn sample(&mut self, top_n: usize, with_processes: bool) -> PerfSnapshot {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        if with_processes {
            self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        }
        self.disks.refresh(true);
        self.networks.refresh(true);

        let cores = self.sys.cpus().len();
        let cpu_cores: Vec<f32> = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let cpu_time = self.sys.global_cpu_usage();
        let cpu_util = self.cpu_util.as_mut().and_then(|c| c.read());
        let cpu_total = cpu_util.map(|u| u as f32).unwrap_or(cpu_time);
        let proc_scale = cpu_util.map(|u| crate::winperf::utility_scale(u, cpu_time)).unwrap_or(1.0);
        let private_ws = if with_processes { crate::winperf::private_working_sets() } else { None };

        let procs: Vec<ProcInfo> = if !with_processes { Vec::new() } else { self
            .sys
            .processes()
            .values()
            .filter(|p| p.pid().as_u32() != 0)
            .map(|p| ProcInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                cpu: (normalize_cpu(p.cpu_usage(), cores) * proc_scale).clamp(0.0, 100.0),
                mem: private_ws
                    .as_ref()
                    .and_then(|m| m.get(&p.pid().as_u32()).copied())
                    .unwrap_or_else(|| p.memory()),
            })
            .collect() };

        let mut disk_read = 0u64;
        let mut disk_write = 0u64;
        let disks: Vec<DiskInfo> = self
            .disks
            .iter()
            // Fixed volumes only: USB sticks and card readers come and go and would clutter the panel.
            .filter(|d| !d.is_removable() && d.total_space() > 0)
            .map(|d| {
                let usage = d.usage();
                let read_bps = per_second(usage.read_bytes, elapsed);
                let write_bps = per_second(usage.written_bytes, elapsed);
                disk_read += read_bps;
                disk_write += write_bps;
                DiskInfo {
                    name: d.name().to_string_lossy().into_owned(),
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    total: d.total_space(),
                    free: d.available_space(),
                    read_bps,
                    write_bps,
                }
            })
            .collect();

        let (rx, tx) = match (crate::winperf::physical_net_octets(), self.last_net) {
            (Some(now_net), prev) => {
                self.last_net = Some(now_net);
                match prev {
                    Some((prx, ptx)) => (crate::winperf::counter_delta(prx, now_net.0), crate::winperf::counter_delta(ptx, now_net.1)),
                    None => (0, 0),
                }
            }
            (None, _) => {
                let (mut rx, mut tx) = (0u64, 0u64);
                for (_, data) in &self.networks {
                    rx = rx.saturating_add(data.received());
                    tx = tx.saturating_add(data.transmitted());
                }
                (rx, tx)
            }
        };
        let (commit_used, commit_total) =
            crate::winperf::commit_charge().unwrap_or((self.sys.used_swap(), self.sys.total_swap()));

        PerfSnapshot {
            ts_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            cpu_total,
            cpu_cores,
            mem_used: self.sys.used_memory(),
            mem_total: self.sys.total_memory(),
            commit_used,
            commit_total,
            disks,
            disk_read_bps: disk_read,
            disk_write_bps: disk_write,
            net_rx_bps: per_second(rx, elapsed),
            net_tx_bps: per_second(tx, elapsed),
            has_processes: with_processes,
            process_count: procs.len(),
            top_cpu: top_n_by(&procs, top_n, |p| p.cpu as f64),
            top_mem: top_n_by(&procs, top_n, |p| p.mem as f64),
        }
    }
}

/// Starts the sampler thread once. `set_perf_active(true)` wakes it for an immediate full sample.
pub fn start(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("perf-sampler".into()).spawn(move || {
        let mut sampler = Sampler::new();
        // First CPU readings need a gap after the priming refresh.
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(Duration::from_millis(250)));
        loop {
            let active = ACTIVE.load(Ordering::Relaxed);
            let top_n = parse_top_n(crate::utils::get_setting_str(&app, "nectar-perf-top-n").as_deref());
            let snap = sampler.sample(top_n, active);
            let _ = app.emit(EVENT, &snap);
            let wait = if active {
                Duration::from_millis(parse_interval_ms(crate::utils::get_setting_str(&app, "nectar-perf-interval-ms").as_deref()))
            } else {
                IDLE_INTERVAL
            };
            std::thread::park_timeout(wait);
        }
    });
    if let Ok(handle) = spawned {
        let _ = WORKER.set(handle.thread().clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, cpu: f32, mem: u64) -> ProcInfo {
        ProcInfo { pid, name: format!("p{pid}"), cpu, mem }
    }

    #[test]
    fn per_second_scales_by_elapsed() {
        assert_eq!(per_second(1000, Duration::from_millis(1000)), 1000);
        assert_eq!(per_second(1000, Duration::from_millis(500)), 2000);
        assert_eq!(per_second(1000, Duration::from_millis(0)), 0);
    }

    #[test]
    fn normalize_cpu_divides_by_cores_and_clamps() {
        assert_eq!(normalize_cpu(400.0, 8), 50.0);
        assert_eq!(normalize_cpu(900.0, 8), 100.0);
        assert_eq!(normalize_cpu(50.0, 0), 0.0);
    }

    #[test]
    fn top_n_sorts_descending_and_truncates() {
        let procs = vec![p(1, 5.0, 10), p(2, 50.0, 5), p(3, 20.0, 99), p(4, 50.0, 1)];
        let by_cpu: Vec<u32> = top_n_by(&procs, 3, |x| x.cpu as f64).iter().map(|x| x.pid).collect();
        assert_eq!(by_cpu, vec![2, 4, 3]);
        let by_mem: Vec<u32> = top_n_by(&procs, 2, |x| x.mem as f64).iter().map(|x| x.pid).collect();
        assert_eq!(by_mem, vec![3, 1]);
    }

    #[test]
    fn settings_are_clamped() {
        assert_eq!(parse_interval_ms(None), 1000);
        assert_eq!(parse_interval_ms(Some("100")), 500);
        assert_eq!(parse_interval_ms(Some("abc")), 1000);
        assert_eq!(parse_top_n(Some("50")), 20);
        assert_eq!(parse_top_n(Some("1")), 3);
    }
}
