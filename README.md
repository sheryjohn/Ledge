# Ledge

**A left-edge notch for Windows that shows your machine's vitals at a glance.**

Touch the left edge of the screen and Ledge slides out with CPU, memory, disk and network, live history charts, and the processes using the most. End a runaway process from the same panel. Move the cursor away and it disappears. No window, no taskbar button.

Ledge is a fork of [Nectar](https://github.com/kua8/Nectar) by kua8, which is a fork of [Bloom](https://github.com/SehajveerSingh2005/bloom) by Sehajveer Singh. It keeps Nectar's spring-animated island and everything it already does: music with a visualiser, Control Center, battery and weather, calendar with Pomodoro, the shelf. Nectar's original README is kept in [docs/NECTAR-README.md](docs/NECTAR-README.md).

## What Ledge adds

- **Left-edge position.** Settings > Notch > Position puts the notch on the left edge, vertically centred. In Peek mode it stays hidden until the cursor rests on the left edge. Top is still available.
- **Performance panel.** The first panel on reveal. Four headline numbers, CPU and memory charts covering the last two minutes, free space for each fixed disk, and the top processes sorted by CPU or memory. Each process row has an End task button. It needs two clicks, and system-critical processes are refused.
- **Colour states.** Tiles and charts turn amber or red when CPU stays high, memory runs short or a disk fills up. The collapsed left pill shows the worst state, so you can see trouble without opening the panel.
- **Dock off by default.** Nectar's dock replaces the Windows taskbar while it runs. Ledge starts with it off. Turn it on in Settings > Dock.

Ledge uses its own settings, startup entry and install folder, so it can run alongside an installed Nectar.

## Build

Requirements: Windows 10 or 11, [Rust](https://rustup.rs/) with the MSVC build tools, and [Bun](https://bun.sh/).

```
bun install
bun run tauri dev
```

Release build: `bun run tauri build`. Tests: `cargo test --manifest-path src-tauri/Cargo.toml` and `node --experimental-strip-types scripts/perf-format.test.mts`.

Without the MSVC build tools, push a `ledge/*` branch. The Build workflow in `.github/workflows/build.yml` runs the tests and uploads `ledge.exe` as an artifact.

Settings are documented in [SETTINGS.md](SETTINGS.md). How the code fits together, and what each phase changed, is in [ARCHITECTURE.md](ARCHITECTURE.md).

## Licence

GPL-3.0, inherited from Nectar and Bloom. See [LICENSE](LICENSE). Any distribution of Ledge must stay under GPL-3.0 and credit both upstream projects.
