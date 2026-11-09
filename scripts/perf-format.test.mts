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
