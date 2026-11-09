# Ledge

**A left-edge notch for Windows that gives you your machine's vitals at a glance.**

Ledge is a fork of [Nectar](https://github.com/kua8/Nectar) (itself a fork of [Bloom](https://github.com/SehajveerSingh2005/bloom)). It keeps Nectar's spring-animated island, its design language, and everything the notch already does, then moves it to the left edge of the screen and makes a Task Manager style performance panel the headline feature.

Think of it as a Dynamic Island that answers "what is my laptop doing right now?" the moment your cursor touches the left side of the screen.

---

## Why this exists

Opening Task Manager is a context switch: Ctrl+Shift+Esc, a new window, a tab, then close it. Most of the time the question is small. Is something eating CPU? Am I out of RAM? Is the disk pegged? Which process is the culprit?

Ledge answers that in under a second, in a panel that disappears the moment you move away. No window, no taskbar icon, no reserved desktop space.

## Use cases

1. **Quick vitals check.** Cursor to the left edge, see CPU, memory, disk, network, battery in one view, move away. Under a second, no clicks.
2. **Find the hog.** Fans spin up or the machine stutters. Reveal Ledge, the process list is already sorted by CPU, the offender is on top.
3. **Kill it.** A kill button on each process row, using Nectar's existing End task semantics. No trip to Task Manager.
4. **Memory pressure before it bites.** Live RAM and swap sparklines, a colour shift when pressure crosses a threshold, so a browser tab explosion is visible before the machine locks.
5. **Disk at a glance.** Per-volume free space and active I/O, useful before a large download, build, or video export.
6. **Everything Nectar already does.** Music with visualiser, Command Center (Wi-Fi, Bluetooth, DND, volume, brightness), battery and weather, calendar with Pomodoro. Scroll or swipe between modes. Performance is simply the default mode on reveal.
7. **Stay out of the way.** Peek mode on the left edge: fully hidden until hover, click-through everywhere except a thin reveal strip, never overlapping maximised windows.

## What changes from Nectar

| Area | Nectar today | Ledge |
|------|--------------|-------|
| Position | Top edge, horizontal island | Left edge, vertical island |
| Reveal trigger (Peek) | Cursor reaches top edge (y near 0) | Cursor reaches left edge (x near 0) |
| Default mode | Music / Status | Performance |
| System stats | Basic WMI readings in Status mode | Full panel: CPU per core, RAM, swap, per-disk, network, battery, top processes with kill |
| Dock | On by default | Off by default (still available in settings) |
| Branding | Nectar | Ledge |

Everything else carries over: the spring animation system, settings.json live reload, multi-monitor Show On, themes, Fixed/Smart/Peek modes, tray flyout, auto update plumbing.

## Technical plan

Stack is inherited from Nectar: **Tauri v2**, **Rust** backend talking to the Windows shell (global input hooks, WASAPI, COM media sessions, WMI), **React + TypeScript** frontend with spring animations, **Bun** for tooling. Windows only.

### Phase 1: Left-edge anchoring (the structural work)

- Add a `position` setting for the notch: `top` (existing) or `left` (new).
- In `src-tauri`, find where the notch window geometry is computed and where the Peek hit-test reads cursor position. Add a left-edge branch: window anchored at x = 0, vertically centred (or configurable y offset), growing horizontally on reveal.
- The springs already animate width, height, corner radius and position independently. For `left`, swap which axis is the "reveal" axis. Collapsed state is a thin vertical pill; expanded state grows rightward.
- Peek hot zone becomes a narrow vertical strip along x = 0 instead of a horizontal strip along y = 0. Keep the same thin click-through margin so screenshot tools and games are not blocked.
- Adjust frontend layout for a vertical island: mode switcher, content panels, and the visualiser need a portrait layout variant.

### Phase 2: Performance panel

- Add the `sysinfo` crate to the Rust backend. Poll on a timer (1 s default, configurable): CPU total and per core, memory and swap, per-disk total/free and read/write throughput, network rx/tx, battery, process list (name, PID, CPU %, memory).
- Push snapshots to the frontend via Tauri events. Keep a short ring buffer (60 to 120 samples) on the frontend for sparklines.
- Render in Nectar's design language: compact header row with the four headline numbers, sparklines for CPU and RAM, stacked bars for disks, a top-N process list (N configurable, default 8) sortable by CPU or memory.
- Kill action per process row, reusing the existing End task implementation from the dock.
- Make Performance the default mode on reveal; other modes remain reachable by scroll/swipe.

### Phase 3: Polish and branding

- Rename end to end (app id, installer, settings path, tray tooltip, splash). Keep the GPL-3.0 licence and the attribution chain to Nectar and Bloom in README and About.
- Thresholds and colour states (for example amber above 80 % CPU sustained, red above 90 % RAM).
- Optional: pin the panel open, compact "gauges only" collapsed state, per-mode default on reveal.

## Build prerequisites

- Windows 10/11
- [Rust](https://rustup.rs/)
- [Bun](https://bun.sh/)

```
git clone <your-fork-url> Ledge
cd Ledge
bun install
bun run tauri dev
```

Release build: `bun run tauri build`

## First session checklist (for Claude Code)

1. Fork `kua8/Nectar` on GitHub, clone the fork into this folder (or next to it).
2. Run it as is. Set notch to Peek, dock off. Feel the baseline.
3. Map the codebase: where is notch window geometry computed, where is the Peek cursor hit-test, where do modes live in the React side, how do Rust and React talk (commands and events).
4. Write that map into `ARCHITECTURE.md` before changing anything.
5. Start Phase 1 behind the `position: left` setting so `top` keeps working.

## Licence

GPL-3.0, inherited from Nectar and Bloom. Any distribution of Ledge must remain GPL-3.0 and credit both upstream projects.

## Conventions

- No em dashes anywhere: code comments, docs, UI strings, commit messages. Use colons, commas, periods, or reword.
- Keep the UI uncluttered. The whole point is glanceability; if a panel needs scrolling to read the headline numbers, it has too much in it.
