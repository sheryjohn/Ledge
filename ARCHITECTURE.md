# Ledge architecture

Ledge is a fork of Nectar (kua8), itself a fork of Bloom (Sehajveer Singh). This file maps the parts of the inherited codebase that Ledge changes, and records what each phase changes. It was written from the upstream tree at Nectar commit `0cfed70`.

## Process and windows

Tauri v2 app. One Rust process (`src-tauri/src`) owns several transparent, undecorated, always-on-top webview windows declared in `src-tauri/tauri.conf.json`:

| Label | Entry | Purpose |
|-------|-------|---------|
| `main` | `index.html`, `src/App.tsx` | The notch (island) on the primary notch monitor. |
| `main-<monitor>` | same | Extra notch windows for "Show On" multi-monitor. `state::is_notch_label` and `state::monitor_suffix` identify them. |
| `overlay` | `overlay.html`, `src/Overlay.tsx` | Full-screen click-through layer for the volume and brightness edge HUDs. |
| `dock`, `dock-<monitor>` | `dock.html`, `src/Dock.tsx` | Bottom dock. |
| `settings`, `calendar` | own html | Normal windows. |

## Notch window geometry

The notch window is not the island. It is a large transparent strip; the island is a `motion.div` drawn inside it, and everything else in the strip is made click-through on the fly.

Today the strip is always the full monitor width, `420 * nectar-scale * dpi` physical pixels tall, at the monitor's top-left. That rectangle is computed in two places:

- `commands.rs: init_notch_window` (smart and peek, and on every mode change).
- `services.rs: register_appbar` (fixed mode, and every re-sync). Despite the name it no longer registers an AppBar for the notch; it styles the window and positions it.

`services.rs: reposition_all_windows` and the monitor watcher call into both.

## Peek hit-test (mouse hook)

`services.rs` installs a `WH_MOUSE_LL` hook. The hook only records the cursor and wakes a worker thread, which runs `process_mouse_move`:

1. **Edge band.** `in_top_band` is true when the cursor is within 8 logical px of the notch monitor's top edge. `top_edge_armed` adds a dwell (`nectar-notch-edge-delay`, default 200 ms) before the band counts, so sweeping across a maximised window's tabs does not trigger a peek.
2. **Hover signal.** Armed band or the frontend's own hover (`set_notch_hovered`) sets a 500 ms hover expiry, and the result is emitted to the window as `notch-edge-hover`. The frontend uses it to un-hide the island in peek and smart.
3. **Click-through.** The frontend reports the island's CSS rect through `update_notch_rect` (stored in `state::notch_rects`). The hook converts it to physical pixels, pads it (20 px sideways, 5 px below, plus 10 px hysteresis once grabbed), and turns off click-through only when the cursor is in that box, or in the edge band within 60 px of the island's span. Everything else in the strip passes clicks through.
4. `hit_test_extra_notch_windows` repeats steps 1 to 3 for `main-<monitor>` windows using `monitor_rect_for_hwnd`.

The geometry above is hard-coded to the top edge (y near 0, x span).

## Frontend notch (src/App.tsx)

- One big component. Notch visibility mode (`fixed`, `smart`, `peek`) comes from `nectar-notch-mode`; `isHidden` combines mode, fullscreen, overlap and the edge-hover state.
- The island is the `motion.div` with `ref={nectarRef}`. framer-motion springs animate `y` (hide offset, `-100` when hidden), `width`, `height` and the four corner radii independently. `getDynamicWidth` and `getDynamicHeight` size it per mode.
- Modes are `nectarMode`: `status`, `music`, `calendar`, `command-center`, `shelf`, `agent`. Scroll or swipe cycles them (`handleWheel`).
- Settings sync: `useSettingsSync` maps settings keys to setters. Settings live in `settings.json`, are mirrored to `localStorage`, and are live reloaded. Rust reads them through `utils::get_setting_str` (an in-memory cache).

## Rust and React contract (relevant subset)

| Direction | Name | Meaning |
|-----------|------|---------|
| React to Rust | `update_notch_rect` | Island bounds in CSS px, relative to the window. |
| React to Rust | `set_notch_hidden`, `set_notch_hovered` | Lets the hook keep a hidden island click-through. |
| React to Rust | `change_notch_mode` | Re-runs `init_notch_window` for every notch window. |
| Rust to React | `notch-edge-hover` | Cursor is in the armed edge band or over the island. |

## Known collision: the left-edge volume HUD

Nectar already uses the left screen edge. The volume HUD (`overlay` window, `nectar-volume-side`, default `left`) reveals when the cursor rests in an 8 px band on the left edge, and its collapsed card is a 42 x 196 px tab vertically centred there. A left-anchored notch occupies the same strip.

## Phase 1 changes (this branch)

1. **New setting** `nectar-notch-position`: `top` (default, unchanged upstream behaviour) or `left`. It keeps the `nectar-` prefix like every other key until the Phase 3 rename.
2. **New module** `src-tauri/src/notch_geometry.rs` holds the position enum and pure, unit-tested functions for:
   - the strip depth (top: 420 logical px as upstream; left: 960, because expanded panels grow sideways and the widest passes 780),
   - the notch window rectangle on a monitor (top: full width by depth; left: depth by full height),
   - the edge band test (top: y near 0; left: x near 0),
   - the padded interactive box and the edge span around the island.
3. `init_notch_window`, `register_appbar`, the main hit-test and `hit_test_extra_notch_windows` call those functions instead of hard-coding the top edge. With `top` the numbers are identical to upstream.
4. **Frontend:** `App.tsx` reads `nectar-notch-position`. For `left` the screen container anchors the island to the left edge, vertically centred; the hide offset moves to `x`; the collapsed pill swaps to a thin vertical shape; the corners flush against the edge flatten; the expanded panel grows rightward. Collapsed status content is not shown in `left` yet, because it is laid out horizontally. A portrait collapsed layout is the remaining Phase 1 item.
5. **HUD collision:** when the notch is on the left, the volume HUD is treated as being on the right, both in the hook and in `Overlay.tsx`, so the two never fight over the same band.
6. **Settings UI:** a Position control in Settings, Notch tab (`src/settings/NotchPosition.tsx`). Saving the key makes `save_setting` re-run `init_notch_window` for every notch window with that window's own resolved mode, so per-monitor modes are kept.

## Phase 1 status

Done on branch `ledge/phase-1`: items 1 to 6 above.

Open:

- **Not yet compiled or run.** The machine this was written on had no Rust toolchain or Bun. First step on a dev machine: `cargo test --manifest-path src-tauri/Cargo.toml notch_geometry`, then `bun run tauri dev`, set Position to Left and Peek on.
- **Portrait collapsed content.** In `left` the collapsed pill is empty; status widgets, music compact view and agent wings only appear once the island is hovered and expanded.
- **Reveal semantics.** As upstream, touching the edge reveals the collapsed pill and hovering the pill expands it. The brief wants vitals the moment the cursor touches the edge, which likely means expanding straight away in `left` mode. Decide once Phase 2 makes Performance the default panel.
- **Expanded panel layouts** are unchanged landscape panels growing rightward. They fit, but the visualiser and mode switcher may want portrait variants.
- `isImpacted` (the flattened, wider top notch) has no left-edge meaning yet; it only flattens the left corners.

Not changed in Phase 1: the dock, the overlay HUD geometry itself, multi-monitor selection, and all mode content.

## Phase 2 plan: Performance panel

Backend, new module `src-tauri/src/perf.rs`:

- Adds the `sysinfo` crate (0.33). A dedicated thread samples CPU (total and per core), memory and swap, each fixed disk (total, free, read and write bytes per second), network receive and transmit bytes per second summed over interfaces, and the process list (name, PID, CPU, memory).
- Process CPU is normalised by core count so it matches Task Manager, where all cores at full load read 100 %. PID 0 (System Idle Process) is skipped.
- While a notch window says the panel is open (`set_perf_active`), the thread takes a full sample every interval. Otherwise it takes a light sample every 5 s with no process scan, so the sparklines already have history when the panel opens. Activation wakes the thread for an immediate full sample.
- Each sample is emitted to all windows as the `perf-snapshot` event. It carries the top N processes by CPU and the top N by memory, so the frontend can switch sort without a round trip.
- Settings: `nectar-perf-interval-ms` (default 1000, clamped to 500 to 5000) and `nectar-perf-top-n` (default 8, clamped to 3 to 20).
- Kill reuses the existing `kill_process` command (PID based, verifies the image name, refuses protected system processes).

Frontend:

- `src/hooks/usePerf.ts` subscribes to `perf-snapshot`, keeps the last 150 samples for sparklines (plotted by timestamp over two minutes, so sparse idle samples and dense active ones share an axis), and toggles `set_perf_active` with the panel's visibility.
- `src/perfFormat.ts` holds the pure formatting and sparkline maths. `node --experimental-strip-types scripts/perf-format.test.mts` checks it without any packages installed.
- `src/components/PerformancePanel.tsx` renders the header row of four numbers (CPU, memory, disk, network), CPU and memory sparklines, one bar per fixed disk, and the process list with a sort toggle and a kill button per row.
- `performance` becomes a notch mode, first in the default panel order, the mode shown on reveal, and switchable off in Settings like the other panels.

## Phase 2 status

Done on branch `ledge/phase-1`: everything in the Phase 2 plan above. Kill uses a two-click confirm (the first click arms the button for 3 s) because the row button sits under the cursor while scanning the list.

Verified on the authoring machine:

- `scripts/perf-format.test.mts` passes under Node 22.
- Every changed TypeScript file parses (TypeScript 6.0 transpile check).
- A type check without `node_modules` shows no errors in changed code other than ones caused by the missing packages.

Not verified, needs a dev machine:

- **Rust has never been compiled.** `sysinfo` calls were written against the 0.33.1 docs. `Cargo.lock` does not list `sysinfo` yet; the first `cargo build` adds it. Run `cargo test --manifest-path src-tauri/Cargo.toml` for the `notch_geometry` and `perf` tests.
- Real `tsc` and `vite build` (`bun run build`).
- Behaviour: panel height against the top strip with many disks, sampler CPU cost, kill of an elevated process (expected to fail with a readable error in the row).

Left for Phase 3: threshold colours, per-core view, renaming, and translations of the new strings (English is the fallback for every other language).

## Phase 3 plan: rename, thresholds, polish

### Rename map

Identity is what Windows and other tools key on. Every item below changes together so Ledge and an installed Nectar can live side by side without touching each other.

| Thing | Nectar | Ledge |
|-------|--------|-------|
| App identifier, settings and data folder | `com.kua8.nectar` | `com.sheryjohn.ledge` (fresh settings; Nectar's are left alone) |
| Executable, Cargo package | `nectar.exe`, `nectar` / `nectar_lib` | `ledge.exe`, `ledge` / `ledge_lib` |
| Product name, window titles | `nectar`, `Nectar Dock`, `Nectar Settings` | `ledge`, `Ledge Dock`, `Ledge Settings` |
| Single-instance mutex and event | `NectarSingleInstance`, `NectarOpenSettings` | `LedgeSingleInstance`, `LedgeOpenSettings` |
| Window classes | `NectarAttentionHook`, `NectarDisplayMonitor` | `LedgeAttentionHook`, `LedgeDisplayMonitor` |
| Logon task, uninstall key, shortcuts | `Nectar`, `...\Uninstall\Nectar`, `Nectar.lnk` | `Ledge`, `...\Uninstall\Ledge`, `Ledge.lnk` |
| Install registry key | `Software\kua8\nectar` | `Software\sheryjohn\ledge` |
| Credential store names, user agent, log file | `Nectar CalDAV`, `Nectar`, `nectar.log` | `Ledge CalDAV`, `Ledge`, `ledge.log` |
| Updater | `kua8/Nectar` releases | `sheryjohn/Ledge` releases |

Kept on purpose: the `nectar-` settings key prefix, the `.nectar` CSS class and internal function names such as `quit_nectar`. They are invisible to users, and renaming them would only churn the diff against upstream.

**Updater.** The updater must not point at Nectar's releases, or a renamed Ledge would offer to replace itself with Nectar. It now points at the fork. The fork has no releases yet, so the updater finds nothing. Publishing Ledge releases later needs a new signing key pair, because the public key in `tauri.conf.json` belongs to upstream.

**Uninstall safety.** Both uninstallers (the app's own and `installer-app`) only delete a whole folder when its name contains the product name. That check moves from `nectar` to `ledge` in both, with tests, so the two products can never delete each other's folders.

User-facing text: "Nectar" becomes "Ledge" in all six locales. README and Settings > About credit Nectar (kua8) and Bloom (Sehajveer Singh) under GPL-3.0.

### Thresholds

Colour states, computed in `perfFormat.ts` and unit tested:

- CPU: amber when the 10 second average is at or above 80 %, red at or above 95 %. Sustained, so a single spike does not flash.
- Memory: amber at or above 80 %, red at or above 90 %.
- Disk: amber when a volume has less than 15 % free, red below 5 %.

The headline tiles, sparklines and disk bars take the colour. The collapsed left-edge pill takes the worst state, so trouble is visible without opening the panel.

### Polish

- Translations of the Ledge strings into the five other bundled languages.

## Phase 3 status

Done: the rename map above, updater pointed at the fork, uninstall safety tests, Ledge text in all six languages with translations of every Ledge string, credits in README and About, and threshold colours including the collapsed pill. Nectar's original README is kept at `docs/NECTAR-README.md`.

Not done from the optional list: pinning the panel open, a gauges-only collapsed state and a per-mode default on reveal. The existing "Remember last panel" setting covers part of the last one. The em dashes in upstream locale strings are left as upstream wrote them.

### Known gap: numbers differ from Task Manager

Measured on the authoring machine, `sysinfo` and Task Manager read different counters:

| Metric | Ledge today | Task Manager | Effect |
|--------|-------------|--------------|--------|
| CPU total | `% Processor Time` | `% Processor Utility`, which scales by clock speed | Differed by 17 points under load (97 vs 79) |
| Process memory | Working set, shared pages included | Private working set | Ledge reads higher per process |
| Process list | One row per process | Grouped by app on the Processes tab | A browser shows as many smaller rows |
| Network | Bytes per second, summed over every interface | Bits per second, per adapter | Factor of 8, plus virtual adapters |
| Swap | Page file size from `GlobalMemoryStatusEx` | Committed memory | Different concept |

Matching Task Manager means reading `\Processor Information(_Total)\% Processor Utility` through PDH, private working set per process, showing network in bits per second for physical adapters only, and optionally grouping processes by executable name.

### Matching Task Manager (done)

New module `src-tauri/src/winperf.rs` reads the same Windows sources Task Manager does. Each reader returns `None` on failure and the sampler falls back to the `sysinfo` value, so a missing counter never blanks the panel.

| Metric | Source | Notes |
|--------|--------|-------|
| CPU total | PDH `\Processor Information(_Total)\% Processor Utility`, English counter name so it works on any display language | Capped at 100 like Task Manager |
| Process CPU | `sysinfo` per process, scaled by utility / time for the same sample | Makes the rows add up to the header, as on the Processes tab |
| Process memory | `NtQuerySystemInformation(SystemProcessInformation)`, field `WorkingSetPrivateSize` | One call for every process, no process handles, so elevated processes read too |
| Committed | `GlobalMemoryStatusEx`: limit `ullTotalPageFile`, used `ullTotalPageFile - ullAvailPageFile` | Replaces swap; snapshot fields renamed `commit_used` / `commit_total` |
| Network | `GetIfTable2`, rows with HardwareInterface set, FilterInterface clear, and status Up | Shown in bits per second (Kbps, Mbps, decimal) |

Not changed: the list stays one row per process, like the Details tab. Disk speeds already match.
