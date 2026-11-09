//! Ledge: the Windows counters Task Manager reads, so the Performance panel shows the same
//! numbers. Every reader returns `None` on failure; the sampler then falls back to `sysinfo`.

use std::collections::HashMap;

// ---- Pure helpers (unit tested) ----

/// Task Manager's per-process CPU adds up to its header figure (processor utility). sysinfo's
/// per-process values add up to processor time, so rows are scaled by utility / time.
pub fn utility_scale(utility: f64, time: f32) -> f32 {
    if !utility.is_finite() || time < 1.0 {
        return 1.0;
    }
    ((utility / time as f64) as f32).clamp(0.0, 2.0)
}

/// Counter growth since the last reading. A counter that went backwards (adapter reset,
/// adapter removed) counts as no traffic rather than a huge spike.
pub fn counter_delta(prev: u64, now: u64) -> u64 {
    now.saturating_sub(prev)
}

const IF_HARDWARE: u8 = 1 << 0;
const IF_FILTER: u8 = 1 << 1;
const IF_OPER_UP: i32 = 1;

/// Task Manager lists real adapters only: hardware, not a filter driver layer, and up.
/// Loopback, Hyper-V and WSL switches, and per-filter duplicate rows are left out.
pub fn is_counted_adapter(flags: u8, oper_status: i32) -> bool {
    flags & IF_HARDWARE != 0 && flags & IF_FILTER == 0 && oper_status == IF_OPER_UP
}

// ---- CPU: % Processor Utility through PDH ----

#[repr(C)]
struct PdhFmtCounterValue {
    cstatus: u32,
    value: f64,
}

#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(source: *const u16, user: usize, query: *mut isize) -> u32;
    fn PdhAddEnglishCounterW(query: isize, path: *const u16, user: usize, counter: *mut isize) -> u32;
    fn PdhCollectQueryData(query: isize) -> u32;
    fn PdhGetFormattedCounterValue(counter: isize, format: u32, kind: *mut u32, value: *mut PdhFmtCounterValue) -> u32;
    fn PdhCloseQuery(query: isize) -> u32;
}

const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
const PDH_CSTATUS_VALID_DATA: u32 = 0;
const PDH_CSTATUS_NEW_DATA: u32 = 1;

pub struct CpuUtility {
    query: isize,
    counter: isize,
}

impl CpuUtility {
    /// Opens the counter and takes the first reading. Rate counters need two readings,
    /// so the first `read` after this returns a value.
    pub fn open() -> Option<Self> {
        let path: Vec<u16> = "\\Processor Information(_Total)\\% Processor Utility\0".encode_utf16().collect();
        let mut query = 0isize;
        let mut counter = 0isize;
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
                return None;
            }
            if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) != 0 {
                PdhCloseQuery(query);
                return None;
            }
            PdhCollectQueryData(query);
        }
        Some(CpuUtility { query, counter })
    }

    /// Percent of the whole machine since the previous read, 0 to 100.
    pub fn read(&mut self) -> Option<f64> {
        let mut v = PdhFmtCounterValue { cstatus: u32::MAX, value: 0.0 };
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }
            if PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v) != 0 {
                return None;
            }
        }
        if v.cstatus != PDH_CSTATUS_VALID_DATA && v.cstatus != PDH_CSTATUS_NEW_DATA {
            return None;
        }
        v.value.is_finite().then(|| v.value.clamp(0.0, 100.0))
    }
}

impl Drop for CpuUtility {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}

// ---- Process memory: private working set ----

#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(class: u32, info: *mut std::ffi::c_void, len: u32, ret_len: *mut u32) -> i32;
}

const SYSTEM_PROCESS_INFORMATION: u32 = 5;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;
// x64 layout of SYSTEM_PROCESS_INFORMATION. WorkingSetPrivateSize is the first field
// of the documented Reserved1 block.
const OFF_NEXT: usize = 0x00;
const OFF_PRIVATE_WS: usize = 0x08;
const OFF_PID: usize = 0x50;

/// Private working set of every process by PID, the figure on Task Manager's Processes tab.
pub fn private_working_sets() -> Option<HashMap<u32, u64>> {
    let mut buf: Vec<u64> = vec![0; 128 * 1024];
    let mut status = STATUS_INFO_LENGTH_MISMATCH;
    for _ in 0..5 {
        let mut needed = 0u32;
        let bytes = (buf.len() * 8) as u32;
        status = unsafe { NtQuerySystemInformation(SYSTEM_PROCESS_INFORMATION, buf.as_mut_ptr().cast(), bytes, &mut needed) };
        if status != STATUS_INFO_LENGTH_MISMATCH {
            break;
        }
        // Processes appear between calls; leave headroom.
        let want = (needed as usize).max(bytes as usize) + 64 * 1024;
        buf = vec![0; want / 8 + 1];
    }
    if status < 0 {
        return None;
    }
    let base = buf.as_ptr() as *const u8;
    let len = buf.len() * 8;
    let mut map = HashMap::new();
    let mut off = 0usize;
    loop {
        if off + OFF_PID + 8 > len {
            break;
        }
        let (next, private_ws, pid) = unsafe {
            let p = base.add(off);
            (
                std::ptr::read_unaligned(p.add(OFF_NEXT) as *const u32),
                std::ptr::read_unaligned(p.add(OFF_PRIVATE_WS) as *const i64),
                std::ptr::read_unaligned(p.add(OFF_PID) as *const usize),
            )
        };
        map.insert(pid as u32, private_ws.max(0) as u64);
        if next == 0 {
            break;
        }
        off += next as usize;
    }
    Some(map)
}

// ---- Committed memory ----

/// (committed bytes, commit limit), Task Manager's "Committed" pair.
pub fn commit_charge() -> Option<(u64, u64)> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
    Some((m.ullTotalPageFile.saturating_sub(m.ullAvailPageFile), m.ullTotalPageFile))
}

// ---- Network: physical adapters ----

/// Total (received, sent) bytes over real adapters since boot.
pub fn physical_net_octets() -> Option<(u64, u64)> {
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return None;
        }
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let (mut rx, mut tx) = (0u64, 0u64);
        for r in rows {
            if is_counted_adapter(r.InterfaceAndOperStatusFlags._bitfield, r.OperStatus.0) {
                rx = rx.saturating_add(r.InOctets);
                tx = tx.saturating_add(r.OutOctets);
            }
        }
        FreeMibTable(table as *const _);
        Some((rx, tx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utility_scale_follows_ratio_and_guards_idle() {
        assert_eq!(utility_scale(40.0, 50.0), 0.8);
        assert_eq!(utility_scale(30.0, 0.5), 1.0);
        assert_eq!(utility_scale(f64::NAN, 50.0), 1.0);
        assert_eq!(utility_scale(100.0, 10.0), 2.0);
    }

    #[test]
    fn counter_delta_ignores_resets() {
        assert_eq!(counter_delta(100, 250), 150);
        assert_eq!(counter_delta(250, 100), 0);
    }

    #[test]
    fn only_up_hardware_non_filter_adapters_count() {
        assert!(is_counted_adapter(IF_HARDWARE, IF_OPER_UP));
        assert!(!is_counted_adapter(IF_HARDWARE | IF_FILTER, IF_OPER_UP));
        assert!(!is_counted_adapter(0, IF_OPER_UP));
        assert!(!is_counted_adapter(IF_HARDWARE, 2));
    }

    #[test]
    fn live_readers_return_values() {
        let ws = private_working_sets().expect("process list");
        assert!(ws.contains_key(&std::process::id()));
        let (used, limit) = commit_charge().expect("commit");
        assert!(used > 0 && limit >= used);
        assert!(physical_net_octets().is_some());
        assert!(CpuUtility::open().is_some());
    }
}
