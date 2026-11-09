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
   - the notch window rectangle on a monitor (top: full width by depth; left: depth by full height),
   - the edge band test (top: y near 0; left: x near 0),
   - the padded interactive box and the edge span around the island.
3. `init_notch_window`, `register_appbar`, the main hit-test and `hit_test_extra_notch_windows` call those functions instead of hard-coding the top edge. With `top` the numbers are identical to upstream.
4. **Frontend:** `App.tsx` reads `nectar-notch-position`. For `left` the screen container anchors the island to the left edge, vertically centred; the hide offset moves to `x`; the collapsed pill swaps to a thin vertical shape; the corners flush against the edge flatten; the expanded panel grows rightward. Collapsed status content is not shown in `left` yet, because it is laid out horizontally. A portrait collapsed layout is the remaining Phase 1 item.
5. **HUD collision:** when the notch is on the left, the volume HUD is treated as being on the right, both in the hook and in `Overlay.tsx`, so the two never fight over the same band.
6. **Settings UI:** a Position control in Settings, Notch tab.

Not changed in Phase 1: the dock, the overlay HUD geometry itself, multi-monitor selection, and all mode content.
