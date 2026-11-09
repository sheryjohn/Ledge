import { useEffect, useRef, useState, type MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { useT } from "../i18n";
import { formatBits, formatBytes, formatRate, levelsFor, percent, sparkPaths, type Level, type Point, type ProcInfo } from "../perfFormat";
import type { PerfState } from "../hooks/usePerf";
import "./PerformancePanel.css";

const SPARK_WINDOW_MS = 120_000;
const SPARK_MIN_SPAN_MS = 5_000;
const SPARK_W = 160;
const SPARK_H = 28;
const CONFIRM_MS = 3000;

type Sort = "cpu" | "mem";

function readSort(): Sort {
  try {
    return localStorage.getItem("nectar-perf-sort") === "mem" ? "mem" : "cpu";
  } catch {
    return "cpu";
  }
}

function Sparkline({ label, value, points, extra, level = "ok" }: { label: string; value: string; points: Point[]; extra?: Point[]; level?: Level }) {
  const now = points.length ? points[points.length - 1].t : 0;
  // Stretch short histories across the full width until the window has filled.
  const span = points.length ? Math.max(SPARK_MIN_SPAN_MS, Math.min(SPARK_WINDOW_MS, now - points[0].t)) : SPARK_WINDOW_MS;
  const main = sparkPaths(points, now, span, SPARK_W, SPARK_H);
  const second = extra ? sparkPaths(extra, now, span, SPARK_W, SPARK_H) : null;
  return (
    <div className={`perf-spark level-${level}`}>
      <div className="perf-spark-head">
        <span>{label}</span>
        <span className="perf-num">{value}</span>
      </div>
      <svg viewBox={`0 0 ${SPARK_W} ${SPARK_H}`} preserveAspectRatio="none" aria-hidden="true">
        {main.area && <path d={main.area} className="perf-spark-area" />}
        {main.line && <path d={main.line} className="perf-spark-line" />}
        {second?.line && <path d={second.line} className="perf-spark-line secondary" />}
      </svg>
    </div>
  );
}

function ProcRow({ p, sort, onKilled }: { p: ProcInfo; sort: Sort; onKilled: () => void }) {
  const t = useT();
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  const onKill = (e: MouseEvent) => {
    e.stopPropagation();
    clearTimeout(timer.current);
    if (!confirming) {
      // Two clicks: the first arms the button, so a stray click never ends a task.
      setConfirming(true);
      setError(null);
      timer.current = setTimeout(() => setConfirming(false), CONFIRM_MS);
      return;
    }
    setConfirming(false);
    invoke("kill_process", { pid: p.pid, name: p.name })
      .then(onKilled)
      .catch((err) => {
        setError(String(err));
        timer.current = setTimeout(() => setError(null), CONFIRM_MS);
      });
  };

  return (
    <div className={`perf-proc ${confirming ? "confirming" : ""}`} title={error ?? `${p.name} (PID ${p.pid})`}>
      <span className="perf-proc-name">{error ? t("perf.endFailed") : p.name.replace(/\.exe$/i, "")}</span>
      <span className={`perf-num ${sort === "cpu" ? "strong" : ""}`}>{p.cpu.toFixed(1)}%</span>
      <span className={`perf-num ${sort === "mem" ? "strong" : ""}`}>{formatBytes(p.mem)}</span>
      <button
        className="perf-kill"
        onClick={onKill}
        aria-label={confirming ? t("perf.confirmEnd") : t("perf.endTask")}
        title={confirming ? t("perf.confirmEnd") : t("perf.endTask")}
      >
        {confirming ? <span className="perf-kill-confirm">{t("perf.end")}</span> : <X size={12} strokeWidth={2.5} />}
      </button>
    </div>
  );
}

export function PerformancePanel({ perf }: { perf: PerfState }) {
  const t = useT();
  const [sort, setSort] = useState<Sort>(readSort);
  const [hiddenPids, setHiddenPids] = useState<Set<number>>(() => new Set());
  const s = perf.snap;

  const changeSort = (next: Sort) => {
    setSort(next);
    try { localStorage.setItem("nectar-perf-sort", next); } catch { }
  };

  // Forget killed PIDs once a fresh list no longer has them.
  useEffect(() => {
    if (hiddenPids.size === 0) return;
    const live = new Set(perf.topCpu.concat(perf.topMem).map((p) => p.pid));
    const still = new Set(Array.from(hiddenPids).filter((pid) => live.has(pid)));
    if (still.size !== hiddenPids.size) setHiddenPids(still);
  }, [perf.topCpu, perf.topMem]);

  if (!s) {
    return <div className="perf-panel perf-waiting">{t("perf.waiting")}</div>;
  }

  const memPct = percent(s.mem_used, s.mem_total);
  const lv = levelsFor(s, perf.cpuHist);
  const procs = (sort === "cpu" ? perf.topCpu : perf.topMem).filter((p) => !hiddenPids.has(p.pid));

  return (
    <div className="perf-panel" onClick={(e) => e.stopPropagation()}>
      <div className="perf-stats">
        <div className={`perf-stat level-${lv.cpu}`}>
          <span className="perf-stat-label">{t("perf.cpu")}</span>
          <span className="perf-stat-value">{Math.round(s.cpu_total)}%</span>
        </div>
        <div className={`perf-stat level-${lv.mem}`}>
          <span className="perf-stat-label">{t("perf.memory")}</span>
          <span className="perf-stat-value">{Math.round(memPct)}%</span>
        </div>
        <div className="perf-stat" title={`${t("perf.read")} ${formatRate(s.disk_read_bps)}, ${t("perf.write")} ${formatRate(s.disk_write_bps)}`}>
          <span className="perf-stat-label">{t("perf.disk")}</span>
          <span className="perf-stat-value">{formatRate(s.disk_read_bps + s.disk_write_bps)}</span>
        </div>
        <div className="perf-stat" title={`${t("perf.down")} ${formatBits(s.net_rx_bps)}, ${t("perf.up")} ${formatBits(s.net_tx_bps)}`}>
          <span className="perf-stat-label">{t("perf.network")}</span>
          <span className="perf-stat-value">{formatBits(s.net_rx_bps + s.net_tx_bps)}</span>
        </div>
      </div>

      <div className="perf-sparks">
        <Sparkline label={t("perf.cpu")} value={`${s.cpu_cores.length} ${t("perf.cores")}`} points={perf.cpuHist} level={lv.cpu} />
        <Sparkline
          label={t("perf.memory")}
          value={s.commit_total > 0 ? `${formatBytes(s.mem_used)} · ${t("perf.committed")} ${formatBytes(s.commit_used)}` : formatBytes(s.mem_used)}
          points={perf.memHist}
          extra={s.commit_total > 0 ? perf.commitHist : undefined}
          level={lv.mem}
        />
      </div>

      {s.disks.length > 0 && (
        <div className="perf-disks">
          {s.disks.map((d, i) => {
            const used = percent(d.total - d.free, d.total);
            return (
              <div className={`perf-disk level-${lv.disks[i] ?? "ok"}`} key={d.mount} title={`${d.name || d.mount}: ${t("perf.read")} ${formatRate(d.read_bps)}, ${t("perf.write")} ${formatRate(d.write_bps)}`}>
                <span className="perf-disk-name">{d.mount.replace(/\\$/, "")}</span>
                <div className="perf-bar"><div className="perf-bar-fill" style={{ width: `${used}%` }} /></div>
                <span className="perf-num">{formatBytes(d.free)} {t("perf.free")}</span>
              </div>
            );
          })}
        </div>
      )}

      <div className="perf-list-head">
        <span>{t("perf.processes")}{perf.processCount ? ` · ${perf.processCount}` : ""}</span>
        <div className="perf-sort" role="group">
          <button className={sort === "cpu" ? "active" : ""} onClick={(e) => { e.stopPropagation(); changeSort("cpu"); }}>{t("perf.cpu")}</button>
          <button className={sort === "mem" ? "active" : ""} onClick={(e) => { e.stopPropagation(); changeSort("mem"); }}>{t("perf.memory")}</button>
        </div>
      </div>
      <div className="perf-procs">
        {procs.map((p) => (
          <ProcRow key={p.pid} p={p} sort={sort} onKilled={() => setHiddenPids((prev) => new Set(prev).add(p.pid))} />
        ))}
      </div>
    </div>
  );
}
