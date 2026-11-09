import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { percent, pushPoint, type PerfSnapshot, type Point, type ProcInfo } from "../perfFormat";

// About two minutes at the 1 s active cadence; idle samples arrive every 5 s.
const HISTORY_CAP = 150;

export type PerfState = {
  snap: PerfSnapshot | null;
  topCpu: ProcInfo[];
  topMem: ProcInfo[];
  processCount: number;
  cpuHist: Point[];
  memHist: Point[];
  swapHist: Point[];
};

const EMPTY: PerfState = { snap: null, topCpu: [], topMem: [], processCount: 0, cpuHist: [], memHist: [], swapHist: [] };

/**
 * Ledge Performance data. `listening` subscribes to perf-snapshot (the notch window only);
 * `active` tells the backend the panel is on screen so it samples processes at full rate.
 */
export function usePerf(listening: boolean, active: boolean): PerfState {
  const [state, setState] = useState<PerfState>(EMPTY);

  useEffect(() => {
    if (!listening) return;
    const unlisten = listen<PerfSnapshot>("perf-snapshot", ({ payload: s }) => {
      setState((prev) => {
        const t = s.ts_ms;
        return {
          snap: s,
          // Light samples carry no processes; keep the last full list.
          topCpu: s.has_processes ? s.top_cpu : prev.topCpu,
          topMem: s.has_processes ? s.top_mem : prev.topMem,
          processCount: s.has_processes ? s.process_count : prev.processCount,
          cpuHist: pushPoint(prev.cpuHist, { t, v: s.cpu_total }, HISTORY_CAP),
          memHist: pushPoint(prev.memHist, { t, v: percent(s.mem_used, s.mem_total) }, HISTORY_CAP),
          swapHist: pushPoint(prev.swapHist, { t, v: percent(s.swap_used, s.swap_total) }, HISTORY_CAP),
        };
      });
    });
    return () => { unlisten.then((f) => f()).catch(() => { }); };
  }, [listening]);

  useEffect(() => {
    if (!listening) return;
    invoke("set_perf_active", { active }).catch(() => { });
  }, [listening, active]);

  useEffect(() => () => { invoke("set_perf_active", { active: false }).catch(() => { }); }, []);

  return state;
}
