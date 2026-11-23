# Scrolling Strip (Labs)

Keep windows wide instead of shrinking tiles: a workspace's tiles become one horizontal strip of
columns, with the focused column at 50%, 66%, or 100% of the display and its neighbor in the rest.
Columns outside the viewport are parked off-screen like an inactive workspace's windows. An
experiment: off by default, behind `[labs] scrolling_strip`.

## Sub-features

```ts
Lab switch (Settings › Labs card / [labs] scrolling_strip / config set labs.scrolling_strip)
├── Off (default) -> Option+Shift+S not registered; ctl strip answers
│                    "Scrolling Strip is off (enable it in Settings › Labs)"; every workspace tiles as usual
└── On  -> hot-reloads: worker gate opens, Option+Shift+S registers           // strip.lab

Toggle (focused workspace) Option+Shift+S / ctl strip                         // strip.toggle
├── On : focused column 66% + right neighbor 34%; other columns parked durably
├── Off: every parked column restored, unchanged split layout re-applied
└── Fullscreen Layout on, or Option+/ -> strip off (the two modes exclude each other)

Viewport (pure math: src/strip.rs compute_strip_layout)                       // strip.viewport
├── 1 column -> fills the display
├── Focused column Full -> it alone
├── Otherwise two adjacent columns fill the display edge to edge, never past an edge
├── Focus inside the viewport -> viewport stays put (anchor)
└── Focus outside -> slides one column, entering from the side stepped toward

Keys on a strip                                                               // strip.keys
├── Option+H/L        -> previous/next column, no wrap; past an end -> adjacent display
├── Option+Shift+H/L  -> move the focused column
├── Option+-/=        -> width 100% -> 66% -> 50% and back; unsized columns follow the last pick
├── Option-backtick   -> next column, wrapping
└── Option+J/K        -> adjacent display (no vertical axis on a strip)

Following focus                                                               // strip.follow
├── Click / Dock / Command-Tab onto any column, even a parked one -> viewport slides to it
├── New tiled window -> becomes the focused column
└── Option+Shift+Space -> the column floats out and the strip closes the gap; tiling it again
                          brings it back as the focused column

Persistence and recovery                                                      // strip.persist
├── workspace-state.json "strip": enabled, widths, default width, anchor (all optional)
├── Switch away -> every column parked; back -> only the viewport revealed
├── Pause / quit / restart -> every parked column restored (parked-windows.json)
├── An app drags a parked column back on screen -> re-parked, like an inactive workspace's window
├── Display added or removed -> parked columns re-parked against the new displays
└── Ghostty native-tab replacement -> keeps the slot, its position and its width
```

Unit (FakeBackend), `src/app/tests/strip.rs`:
`lab_off_refuses_the_strip_and_tiles_as_usual_until_it_is_turned_on`,
`stepping_along_the_strip_parks_and_reveals_columns_durably`,
`focus_from_outside_spaceman_slides_the_viewport_to_a_parked_column`,
`pause_and_quit_restore_every_strip_parked_window`,
`switching_away_parks_every_column_and_back_reveals_only_the_viewport`,
`reordering_and_width_cycling_follow_the_focused_column`,
`turning_the_strip_off_reveals_every_column_in_the_split_layout`,
`ghostty_native_tab_replacement_keeps_the_strip_column_and_width`,
`a_strip_column_that_drifts_back_on_screen_is_re_parked`, and
`docking_a_display_re_parks_hidden_strip_columns`. Viewport math: `src/strip/tests.rs`
(every combination of 1-5 columns, widths, focus and anchor fills the area with columns inside it).
Persistence: `persisted_strip_round_trips_and_excludes_fullscreen_layout`. Config:
`parses_labs_with_every_experiment_off_by_default`, `labs_switches_round_trip_and_survive_other_edits`
(CLI), `labs_switch_round_trips_and_leaves_an_all_off_table_out` (Settings). Shortcut gating:
`scrolling_strip_shortcut_registers_only_while_its_lab_is_on`.

## How To Get To It (User POV)

```ts
Settings › Labs -> Scrolling Strip switch on
Focus a window on the workspace
└── Option+Shift+S / spaceman ctl strip -> toggle
```

## Existing Checks

- Unit tests above. No live harness drives the strip against real apps yet.

## Gotchas

- **Columns are parked, not slid.** macOS clamps windows dragged past a display edge, so the strip
  never places a column partly off-screen; off-viewport columns jump to the parked position instead
  of scrolling. No private APIs are involved.
- **A closed column's neighbor appears on the next cycle.** Reconciliation retires the closed window
  first; the column that slides into view is restored by the following observation cycle.
- The HUD's layout preview still draws the split layout for a strip workspace.
