# Claude Code notes for Ledge

Read `LEDGE.md` first. It is the project brief, scope, and phased plan.

## Working rules

- Windows-only Tauri v2 app: Rust backend in `src-tauri`, React + TypeScript frontend in `src`, Bun for tooling.
- Keep upstream Nectar behaviour working behind the new `position: left` setting until Phase 1 is complete.
- Never use em dashes in any file, comment, string, or commit message.
- Prefer small, reviewable commits. One concern per commit.
- Before structural changes, update or create `ARCHITECTURE.md` describing what you found and what you are about to change.
- Licence is GPL-3.0. Preserve attribution to Nectar (kua8) and Bloom (Sehajveer Singh).

## Phase order

1. Left-edge anchoring and Peek hit-test
2. Performance panel (sysinfo crate, Tauri events, process list with kill)
3. Branding, thresholds, polish
