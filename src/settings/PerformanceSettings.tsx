import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Gauge, ListOrdered, Timer } from "lucide-react";
import { SettingRow } from "./SettingRow";
import { Dropdown } from "./Dropdown";
import { useT } from "../i18n";

// Ledge: Performance panel settings. Self-contained like NotchEdgeDelay: reads and writes
// its own keys, and the notch window picks changes up through settings-changed.

const ROWS = ["5", "8", "12"];
const INTERVALS = ["500", "1000", "2000"];

function save(key: string, value: string) {
  try { localStorage.setItem(key, value); } catch { }
  invoke("save_setting", { key, value }).catch(() => { });
}

function read(key: string, allowed: string[], fallback: string): string {
  try {
    const v = localStorage.getItem(key);
    return v && allowed.includes(v) ? v : fallback;
  } catch {
    return fallback;
  }
}

export function PerformanceSettings() {
  const t = useT();
  const [enabled, setEnabled] = useState(() => {
    try { return localStorage.getItem("nectar-panel-performance-enabled") !== "false"; } catch { return true; }
  });
  const [rows, setRows] = useState(() => read("nectar-perf-top-n", ROWS, "8"));
  const [interval, setIntervalValue] = useState(() => read("nectar-perf-interval-ms", INTERVALS, "1000"));

  return (
    <>
      <SettingRow icon={Gauge} label={t("notch.panelPerformance")} desc={t("notch.panelPerformance.desc")}>
        <label className="toggle-switch">
          <input
            type="checkbox"
            checked={enabled}
            onChange={() => {
              const next = !enabled;
              setEnabled(next);
              save("nectar-panel-performance-enabled", String(next));
            }}
          />
          <span className="slider"></span>
        </label>
      </SettingRow>
      {enabled && (
        <>
          <SettingRow icon={ListOrdered} label={t("notch.perfRows")} desc={t("notch.perfRows.desc")}>
            <Dropdown
              value={rows}
              onChange={(v) => { setRows(v); save("nectar-perf-top-n", v); }}
              options={ROWS.map((n) => ({ value: n, label: n }))}
            />
          </SettingRow>
          <SettingRow icon={Timer} label={t("notch.perfInterval")} desc={t("notch.perfInterval.desc")}>
            <Dropdown
              value={interval}
              onChange={(v) => { setIntervalValue(v); save("nectar-perf-interval-ms", v); }}
              options={INTERVALS.map((ms) => ({ value: ms, label: `${Number(ms) / 1000} s` }))}
            />
          </SettingRow>
        </>
      )}
    </>
  );
}
