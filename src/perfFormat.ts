// Ledge: pure helpers for the Performance panel. No React or Tauri imports, so they can be
// checked with plain Node (see scripts in ARCHITECTURE.md).

export type ProcInfo = { pid: number; name: string; cpu: number; mem: number };

export type DiskInfo = {
  name: string;
  mount: string;
  total: number;
  free: number;
  read_bps: number;
  write_bps: number;
};

export type PerfSnapshot = {
  ts_ms: number;
  cpu_total: number;
  cpu_cores: number[];
  mem_used: number;
  mem_total: number;
  swap_used: number;
  swap_total: number;
  disks: DiskInfo[];
  disk_read_bps: number;
  disk_write_bps: number;
  net_rx_bps: number;
  net_tx_bps: number;
  has_processes: boolean;
  process_count: number;
  top_cpu: ProcInfo[];
  top_mem: ProcInfo[];
};

export type Point = { t: number; v: number };

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 1536 -> "1.5 KB". One decimal below 10, none above, like Task Manager. */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  let i = 0;
  let v = n;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = i === 0 || v >= 10 ? 0 : 1;
  return `${v.toFixed(digits)} ${UNITS[i]}`;
}

export function formatRate(bps: number): string {
  return `${formatBytes(bps)}/s`;
}

export function percent(used: number, total: number): number {
  if (!total || total <= 0) return 0;
  return Math.max(0, Math.min(100, (used / total) * 100));
}

/** Appends a point and drops the oldest beyond `cap`. Returns a new array. */
export function pushPoint(buf: Point[], p: Point, cap: number): Point[] {
  const next = buf.length >= cap ? buf.slice(buf.length - cap + 1) : buf.slice();
  next.push(p);
  return next;
}

/**
 * SVG path data for a sparkline over the last `windowMs`, plotted by timestamp so idle
 * (sparse) and active (dense) samples share one time axis. Values are 0..max.
 */
export function sparkPaths(points: Point[], now: number, windowMs: number, w: number, h: number, max = 100): { line: string; area: string } {
  const start = now - windowMs;
  const visible = points.filter((p) => p.t >= start - windowMs * 0.1);
  if (visible.length < 2) return { line: "", area: "" };
  const x = (t: number) => Math.max(0, Math.min(w, ((t - start) / windowMs) * w));
  const y = (v: number) => h - (Math.max(0, Math.min(max, v)) / max) * h;
  const coords = visible.map((p) => `${x(p.t).toFixed(1)},${y(p.v).toFixed(1)}`);
  const line = `M${coords.join(" L")}`;
  const area = `${line} L${x(visible[visible.length - 1].t).toFixed(1)},${h} L${x(visible[0].t).toFixed(1)},${h} Z`;
  return { line, area };
}

/** Pixel height of the Performance panel body for the given content, header row excluded. */
export function panelBodyHeight(diskCount: number, rows: number): number {
  const stats = 40;
  const sparks = 44;
  const disks = diskCount > 0 ? 8 + diskCount * 18 : 0;
  const listHeader = 26;
  const list = Math.max(1, rows) * 22;
  const padding = 16;
  return stats + sparks + disks + listHeader + list + padding;
}
