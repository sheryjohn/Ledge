// Run: node --experimental-strip-types scripts/perf-format.test.mts
import assert from "node:assert/strict";
import { formatBytes, formatRate, percent, pushPoint, sparkPaths, panelBodyHeight } from "../src/perfFormat.ts";

assert.equal(formatBytes(0), "0 B");
assert.equal(formatBytes(512), "512 B");
assert.equal(formatBytes(1536), "1.5 KB");
assert.equal(formatBytes(15 * 1024 * 1024), "15 MB");
assert.equal(formatBytes(8.4 * 1024 ** 3), "8.4 GB");
assert.equal(formatRate(2048), "2.0 KB/s");
assert.equal(percent(50, 200), 25);
assert.equal(percent(5, 0), 0);
assert.equal(percent(300, 200), 100);

let buf = [] as { t: number; v: number }[];
for (let i = 0; i < 5; i++) buf = pushPoint(buf, { t: i, v: i }, 3);
assert.deepEqual(buf.map((p) => p.t), [2, 3, 4]);

assert.deepEqual(sparkPaths([{ t: 0, v: 1 }], 0, 1000, 100, 20), { line: "", area: "" });
const s = sparkPaths([{ t: 0, v: 0 }, { t: 1000, v: 100 }], 1000, 1000, 100, 20);
assert.equal(s.line, "M0.0,20.0 L100.0,0.0");
assert.ok(s.area.endsWith("Z"));

assert.ok(panelBodyHeight(2, 8) < 420 - 36, "panel must fit the top strip");
console.log("perfFormat: all assertions passed");

// Thresholds
import { windowAverage, cpuLevel, memLevel, diskLevel, worst, levelsFor } from "../src/perfFormat.ts";
const pts = [{ t: 0, v: 100 }, { t: 5000, v: 90 }, { t: 9000, v: 70 }, { t: 10000, v: 80 }];
assert.equal(windowAverage(pts, 10000, 10000), 80);
assert.ok(Number.isNaN(windowAverage([], 10000, 10000)));
assert.equal(cpuLevel(79.9), "ok");
assert.equal(cpuLevel(80), "warn");
assert.equal(cpuLevel(95), "crit");
assert.equal(cpuLevel(NaN), "ok");
assert.equal(memLevel(89), "warn");
assert.equal(memLevel(90), "crit");
assert.equal(diskLevel(20), "ok");
assert.equal(diskLevel(14), "warn");
assert.equal(diskLevel(4), "crit");
assert.equal(worst("ok", "warn", "ok"), "warn");
assert.equal(worst(), "ok");
const snap = {
  ts_ms: 10000, cpu_total: 99, cpu_cores: [], mem_used: 50, mem_total: 100, swap_used: 0, swap_total: 0,
  disks: [{ name: "C", mount: "C:\\", total: 100, free: 3, read_bps: 0, write_bps: 0 }],
  disk_read_bps: 0, disk_write_bps: 0, net_rx_bps: 0, net_tx_bps: 0, has_processes: false, process_count: 0, top_cpu: [], top_mem: [],
};
// One spike at the end of a calm window stays ok; the full disk drives the overall state.
const lv = levelsFor(snap, [{ t: 2000, v: 10 }, { t: 6000, v: 10 }, { t: 10000, v: 99 }]);
assert.deepEqual([lv.cpu, lv.mem, lv.disks[0], lv.overall], ["ok", "ok", "crit", "crit"]);
console.log("thresholds: all assertions passed");
