# Feature Map

What a Spaceman user can do, how they reach it, and what currently proves it works. Behavior is
described from the user's point of view; [Controls](../controls.md) and
[Configuration](../configuration.md) remain the reference for exact keys and schema.

```ts
docs/features/
├── README.md               // this index: availability, prerequisites, evidence rules
├── workspaces.md           // switch, move, hide/reveal, where new windows go
├── tiling.md               // split layouts: focus, reorder, drag, resize, balance, axis
├── fullscreen-layout.md    // workspace-wide stacked "every window fills the display" mode
└── restarts-and-recovery.md // what survives restarts/relaunches/reboots, restoration, doctor
```

## Availability

Source baseline: `main` at `7ad30eb` plus the uncommitted working tree of 2026-10-01.

| Surface | Status |
| --- | --- |
| Workspaces, tiling, Fullscreen Layout, parking/restoration | Source builds and the notarized preview artifact |
| Parking exemption, remembered placements (`window-memory.json`), arrangement journal, `spaceman doctor` | On `main` (source builds) |
| Single-process Rune Settings (`Activity`), first-run setup tour, non-activating Workspace HUD (`[settings] hud`) | On `main` (source builds) |
| Named workspaces A-Z | Opt-in: enabled by a `route` rule or `[[workspaces]]` entry |
| Secondary-display workspaces 6-10 | Require a second display; with one display they fall back to `main` |

## Common Prerequisites

- macOS with Accessibility permission granted to `Pane.app`. Without it, or without global
  hotkey registration, Pane starts paused and only restores parked windows.
- No other window manager running (AeroSpace, yabai, Amethyst): two managers fight over frames.
- Only standard, movable, on-screen, non-native-fullscreen `AXWindow`s are managed. Panels,
  dialogs, and other subroles are invisible to Spaceman and stay global (see
  [Compatibility](../compatibility.md)).
- CLI paths assume `~/.local/bin/pane` (or `~/.local/bin/spaceman`) is on `PATH`; layout-changing `ctl` commands require
  automatic tiling to be enabled.

## Evidence Conventions

Each feature file labels evidence as one of:

- **Unit (FakeBackend)**: a test in `src/app/tests/` driving `WorkspaceRuntime` through the fake
  backend. Proves runtime decisions and the native calls requested, not real application behavior,
  z-order, timing, or what is painted on screen. The whole suite (229 tests) last passed on
  2026-10-01 in this working tree.
- **Live harness (not run)**: `scripts/live-workspaces.sh` and `scripts/live-ghostty-tabs.sh` drive
  real hotkeys against disposable Ghostty windows. They require AeroSpace enabled initially and two
  displays; neither was executed for this map.
- **Observed**: behavior seen in `agent.log` or `journal.jsonl` on the maintainer's machine, with the
  date. A log line proves what Spaceman requested, not what the user saw.
- **User report**: what the user saw, unverified by tooling.

Absence of a check is a **gap**, not a defect; a defect is a reproduced or reported wrong outcome.

## Keeping This Map Current

Update the affected feature file when a change alters a shortcut, CLI command, menu item, rule
semantics, what survives a restart, or which tests cover a flow. Add a new file only for a distinct
user goal, not for a new module.
