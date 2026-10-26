# Workspaces

Keep windows grouped on numbered or named virtual workspaces per display, switch between them
instantly, and send windows or whole apps to a workspace, without using macOS Spaces.

## Sub-features

```ts
Switch workspace                                           // ws.switch
├── Option+1..0 / Option+A..Z / spaceman ctl switch <ws>
├── Incoming parked windows revealed -> laid out -> outgoing windows parked off-screen
│   └── Reveal not confirmed by the window server in time -> switch rolled back, old workspace stays
├── Focus -> last-focused tile of the incoming workspace
└── Already active -> stays, re-focuses its remembered tile (keeps current focus if it is there)

Back-and-forth                                             // ws.back-and-forth
└── Option+Tab / ctl back-and-forth -> previous workspace of the focused display's lane

Move window without switching                              // ws.move-window
├── Option+Shift+<ws> / ctl move <ws> -> window parked on target, current workspace reflows
└── ctl move-app <bundle-id> <ws>     -> every current window of that app

Focus follows                                              // ws.focus-follow
└── Focusing a window that lives on a hidden workspace (Dock, Cmd+Tab, notification)
    -> that workspace becomes active

Where a new window goes (first match wins)                 // ws.adoption
├── ignore rule        -> never managed
├── route rule         -> that workspace (parked if hidden); bundle-only route rules apply to the
│                         app's initial window while secondary windows stay on the active workspace
├── float rule / dialog / non-resizable -> floating on the workspace where first seen
├── remembered placement (startup window, first 60 s) -> remembered workspace
└── otherwise          -> active workspace of the display under the window's center

Displays                                                    // ws.displays
├── Default: 1-5 main, 6-10 secondary; [[workspaces]] monitor = main | secondary | UUID | name
└── Preferred display unplugged -> its workspaces fall back to main, return on reconnect
```

Hidden windows are **parked**, not minimized: moved so a one-pixel strip remains at a display edge,
with the original frame persisted to `parked-windows.json` first.

## How To Get To It (User POV)

```ts
Keyboard (while tiling is enabled)
├── Option+<1..0 | A..Z>        -> switch
├── Option+Shift+<1..0 | A..Z>  -> move focused window
└── Option+Tab                  -> back-and-forth
CLI: spaceman ctl switch | back-and-forth | move | move-app
Config: ~/Library/Application Support/Spaceman/config.toml -> [[rules]], [[workspaces]] (read at startup only)
Menu bar: shows the active workspace glyphs; does not switch
Mouse: drop a tile on another display -> moves it to that display's active workspace (see tiling.md)
```

Named-workspace keys exist only for letters referenced in config; they override built-in shortcuts
on the same letter.

## Existing Checks

- Unit (FakeBackend), `src/app/tests/switching.rs`:
  `switching_reveals_and_lays_out_incoming_beneath_before_hiding_outgoing` (ordering of reveal,
  layout, hide), `switch_keeps_outgoing_visible_when_incoming_reveal_never_arrives`
  (`reliability.rs`, rollback), `moving_to_an_inactive_workspace_does_not_switch`,
  `focusing_a_parked_window_activates_its_workspace`,
  `workspace_back_and_forth_tracks_each_display_lane_independently`,
  `switching_to_an_unconfigured_named_workspace_is_rejected`.
- Unit (FakeBackend), `src/app/tests/reconciliation.rs`: adoption order
  (`bundle_route_applies_to_initial_window_while_secondary_windows_stay_on_active_workspace`,
  `title_route_rule_routes_matching_secondary_window`,
  `ignore_rule_releases_the_window_and_never_adopts_it`,
  `floating_rule_owns_window_without_adding_it_to_layout`,
  `new_window_opens_on_active_workspace_even_when_app_has_window_on_inactive_workspace`), display fallback
  (`missing_preferred_display_falls_back_without_double_visibility`,
  `active_preferred_workspace_falls_back_and_returns_after_hotplug`).
- Unit (FakeBackend), `src/app/tests/parking.rs`: parking without minimizing, re-parking windows that
  drift back, honoring windows that keep un-parking (parking exemption, working tree), and keeping
  slots when Accessibility briefly cannot find a hidden window.
- Unit (FakeBackend), `src/app/tests/reconciliation.rs`:
  `an_app_that_quits_while_hidden_releases_its_tiles_and_recovery_metadata` (an app that quits
  while its windows are parked frees their tiles without the workspace being shown).
- Live harness (not run): `scripts/live-workspaces.sh` switches 6/7/8 by real hotkeys, moves without
  switching, rapid-switch coalescing, and hard-kill recovery. Needs two displays and AeroSpace.

## Gotchas

- Unit tests model frames, not z-order or paint timing: "outgoing parked after incoming laid out"
  does not prove nothing flashes. See the stacked-window flash in
  [Fullscreen Layout](fullscreen-layout.md#gotchas).
- Some apps (Electron, Ghostty) move their own windows back on screen; Spaceman re-parks, then after
  repeated undoing honors the reappearance and re-homes the window to the visible workspace. In the
  working tree, a window that does so twice in five minutes stays tiled but is no longer hidden
  across switches (`parking_exempt`).
- Focus-follow switches workspaces as a side effect of any app activation, including ones the user
  did not intend (an app raising itself). Each `focus follow` line in `agent.log` records recent
  input timing and whether Command was held, so Command-Tab can be told apart from self-activation.
- Panels and non-standard windows are never parked, so they show on every workspace; an empty
  workspace exposes whatever unmanaged windows sit behind managed ones.
- Rules and display assignments are read only at startup; `ctl restart` exits but does not relaunch.
