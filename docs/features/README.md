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
├── scratchpad.md           // Lab: windows hidden off-screen until summoned
├── setup-tour.md           // first run: permission, the suggested arrangement, Try it
└── restarts-and-recovery.md // what survives restarts/relaunches/reboots, restoration, doctor
```

## Availability

Baseline: `dev` at 0.3.2 (`93897a4`). Everything below except named workspaces and
secondary-display workspaces ships in the notarized preview at
<https://anoma.ly/spaceman/download/macos-arm64> and in source builds.

| Surface | Status |
| --- | --- |
| Workspaces, tiling, Fullscreen Layout, parking/restoration | Preview artifact and source builds |
| Parking exemption, remembered placements (`window-memory.json`), arrangement journal, `spaceman doctor` | Preview artifact and source builds |
| Single-process Rune Settings, four-step [setup tour](setup-tour.md), non-activating Workspace HUD (`[settings] hud`) | Preview artifact and source builds |
| Named workspaces A-Z | Opt-in: enabled by a `route` rule or `[[workspaces]]` entry |
| Custom shortcuts (`[shortcuts]`) | Preview artifact and source builds; rebind, unbind, or replace the built-in keys in `config.toml` or by recording a chord in Settings › Shortcuts, with a base `modifier` for the built-ins, applied live without a restart. Covered by unit tests in `src/shortcuts.rs`, `src/hotkeys.rs`, `src/config.rs`, `src/app/app_loop/live_shortcuts.rs`, `agent/src/rules/shortcuts/tests.rs`, and `agent/src/panes/shortcuts/`; no live harness exercises a rebound chord or live re-registration yet |
| Scratchpad (`[labs] scratchpad`) | Opt-in Lab in source builds; off by default, shortcuts register only when on |
| Settings › Activity (`[labs] activity`) | Opt-in Lab in source builds; off by default, the pane stays out of the sidebar while `spaceman doctor` and the journal keep recording |
| Portrait edge rows (`[labs] portrait_edge_rows`) | Opt-in Lab in source builds; off by default, top/bottom display edge drop bands on portrait displays only when on |
| Physical display order and shape (`[labs] physical_displays`) | Opt-in Lab in source builds; off by default, switched in Settings › Labs › Menu bar & HUD. Menu-bar glyphs and menu sections left to right, portrait glyphs and HUD Layouts previews tall, same-named displays labelled by position. Covered by `src/app/tests/physical_displays.rs`, `src/macos/displays.rs` and `src/status_item.rs` unit tests; no live harness checks the drawn glyphs |
| Relative resize (`[labs] relative_resize`) | Opt-in Lab in source builds; off by default, switched in Settings › Labs › Windows; `resize width\|height grow\|shrink POINTS` reports `Relative resize is off` until it is on |
| Secondary-display workspaces 6-10 | Require a second display; with one display they fall back to `main` |
| Three or more displays | Assign workspaces in Settings › Workspaces (each display past `secondary` by UUID) or with `[[workspaces]] monitor = "<uuid>"` |

## Common Prerequisites

- macOS with Accessibility permission granted to `Spaceman.app`. Without it, or without global
  hotkey registration, Spaceman starts paused and only restores parked windows.
- No other window manager running (AeroSpace, yabai, Amethyst): two managers fight over frames.
- Only standard, movable, on-screen, non-native-fullscreen `AXWindow`s are managed. Panels,
  dialogs, and other subroles are invisible to Spaceman and stay global (see
  [Compatibility](../compatibility.md)).
- CLI paths assume `~/.local/bin/spaceman` is on `PATH`; layout-changing `ctl` commands require
  automatic tiling to be enabled.

## Evidence Conventions

Each feature file labels evidence as one of:

- **Unit (FakeBackend)**: a test in `src/app/tests/` driving `WorkspaceRuntime` through the fake
  backend. Proves runtime decisions and the native calls requested, not real application behavior,
  z-order, timing, or what is painted on screen. CI runs the whole suite for every release.
- **Live harness**: `scripts/live-workspaces.sh` and `scripts/live-ghostty-tabs.sh` drive real
  hotkeys against disposable Ghostty windows. They require AeroSpace enabled initially and two
  displays.
- **Live gauntlet**: `scripts/gauntlet.sh` (see [Development](../development.md#live-gauntlet)) runs a
  real Spaceman.app against disposable windows on one display and asserts each step against both
  the window server and Spaceman's state files, with a JSON receipt. Proves real frames, parking,
  focus, timing and restoration; it does not drive hotkeys or a second display.
- **Observed**: behavior seen in `agent.log` or `journal.jsonl` on the maintainer's machine, with the
  date. A log line proves what Spaceman requested, not what the user saw.
- **User report**: what the user saw, unverified by tooling.

Absence of a check is a **gap**, not a defect; a defect is a reproduced or reported wrong outcome.

## Keeping This Map Current

Update the affected feature file when a change alters a shortcut, CLI command, menu item, rule
semantics, what survives a restart, or which tests cover a flow. Add a new file only for a distinct
user goal, not for a new module.
