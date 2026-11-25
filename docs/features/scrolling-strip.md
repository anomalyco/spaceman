# Scrolling Strip (Labs)

Keep windows wide instead of shrinking tiles: a workspace's tiles become one horizontal ribbon of
columns (each 33%, 50%, 66%, 80% or 100% of the display, 50% by default) seen through a viewport
camera that glides on a spring, as in niri. Columns out of view peek 16 points past a free display
edge and are recorded like an inactive workspace's windows. An experiment: off by default, behind
`[labs] scrolling_strip`.

## Sub-features

```ts
Lab switch (Settings › Labs row / [labs] scrolling_strip / config set labs.scrolling_strip)
├── Off (default) -> the strip's shortcut not registered; ctl strip answers
│                    "Scrolling Strip is off (enable it in Settings › Labs)"; every workspace tiles as usual
├── On  -> hot-reloads: worker gate opens, the strip's shortcut registers     // strip.lab
├── Shortcut (the card's second row, only while the switch is on / [labs] scrolling_strip_shortcut,
│   default alt+shift+s)
│   └── hot-reloads; claims its chord ahead of built-ins, [shortcuts] bindings still win;
│       Backspace in the recorder puts the default back                       // strip.shortcut
├── Customize… (quiet button beside the switch, only while it is on) -> sheet     // strip.sheet
│   ├── the strip under the display
│   ├── Glide tuning (below; also [labs] / config set labs.…)
│   ├── Shortcuts: toggle, glide, move, width; every chord read from the file, so the base
│   │   modifier, a rebound shortcut and the strip's own chord show as they are ("Not set" if unbound)
│   └── Done, Escape or a press outside closes; switching the strip off closes it too
├── Glide tuning (the sheet / [labs] / config set labs.…)                        // strip.tuning
│   ├── Glide 120 · 180 · 280 · 400 ms   -> scrolling_strip_duration_ms (80-900, default 180)
│   ├── Feel Bouncy · Snappy · Soft      -> scrolling_strip_damping 0.78 · 1.0 · 1.18 (0.6-1.3)
│   ├── Next to another display Tuck · Slide -> scrolling_strip_shared_edge "tuck" | "slide"
│   └── a value the file holds that isn't a preset shows as an extra segment; all hot-reload
│       before the next key press, and a running glide is retuned in place

Toggle (focused workspace) Option+Shift+S / ctl strip                         // strip.toggle
├── On : columns at their widths from the left edge, no glide; the rest peek or park durably
├── Off: every peeking or parked column restored, unchanged split layout re-applied
└── Fullscreen Layout on, or Option+/ -> strip off (the two modes exclude each other)

Ribbon and camera (pure math: src/strip.rs)                                   // strip.viewport
├── ribbon(): columns end to end at their widths
├── minimal_scroll(): the camera moves only when the focused column leaves the view, by the least distance
├── rest_layout(): in view -> its frame (a partly visible column runs off a free edge)
│                  out of view past a free edge -> 16 pt sliver (PEEK_PX), recorded as parked
│                  out of view past an edge shared with another display -> parked
└── Nothing shows on the other side of a shared edge (ScrollingDisplayBounds clamps it)
    ├── Tuck (default): a column stays wholly inside its display
    ├── Slide: a column may overhang the edge by up to half its width less 8 pt, so its center
    │          stays on its own display; macOS ("Displays have separate Spaces") clips the rest
    └── Curtain (StripMotion::curtain): a column held at a shared edge stays under the neighbor
        sliding across it: arriving, it is uncovered as the neighbor slides off; leaving, it is
        covered, then parked by the settling layout

Glide (src/app/strip/motion.rs -> src/app/app_loop/glide.rs)                   // strip.glide
├── Worker: records every column the glide takes out of view in parked-windows.json and resizes
│           columns whose width changed, then aims CameraSpring (SpringParams::for_duration from
│           [labs]: by default critically damped, a 1,280 pt move arriving within 1 pt in 180 ms)
│           from its current position and velocity; a moved column gets its own offset spring;
│           sends a StripMotion to the main thread
├── Main thread: CADisplayLink runs only while a glide does; each vsync samples the springs and
│                writes positions through InteractivePresenter (per-app lanes, newest frame wins,
│                no readback, enhanced UI held off for the whole glide); its clock runs up to
│                30 ms behind the worker's so the first frame shown is the start of the motion
├── Cycles mid-glide -> every column the window server lists keeps its slot
├── Rapid taps -> the running spring is retargeted, one continuous glide; a tap steps on from
│                 the column the last one aimed at, even before it has focus
├── Focus -> the column a key moves to is focused as the key lands, so typing reaches it at
│            once, and glides in on top: the curtain never tucks it or raises a neighbor over it
├── Curtain -> columns sliding across a tucked one (leaving across a shared edge) are raised over
│              it if the window server stacks it in front
├── Focus refused mid-glide -> retried when the glide settles instead of failing the key
├── Settled (within 1 pt and slow; the final frame snaps to the exact rest) -> link stops; 47 ms
│              later (MAX_PRESENT_LAG + a 60 Hz frame) the worker lays the strip out once,
│              peeks/parks out-of-view columns
│              (a recorded column still on screen at a shared edge too), retires recovery
│              frames of columns back in view
└── Switch, move, toggle, pause or quit mid-glide -> glide halted before windows are touched

Keys on a strip                                                               // strip.keys
├── Option+H/L        -> previous/next column, no wrap; past an end -> adjacent display
├── Option+Shift+H/L  -> move the focused column (neighbors glide to their new places)
├── Option+-/=        -> width 33% · 50% · 66% · 80% · 100%; unsized columns follow the last pick
├── Option-backtick   -> next column, wrapping
└── Option+J/K        -> adjacent display (no vertical axis on a strip)

Following focus                                                               // strip.follow
├── Click / Dock / Command-Tab onto any column, even one out of view -> camera glides to it
├── New tiled window -> becomes the focused column
└── Option+Shift+Space -> the column floats out and the strip closes the gap; tiling it again
                          brings it back as the focused column

Persistence and recovery                                                      // strip.persist
├── workspace-state.json "strip": enabled, widths, default width, anchor (all optional)
├── Switch away -> every column parked; back -> only the viewport revealed, then peeks
├── Pause / quit / restart -> every peeking or parked column restored (parked-windows.json)
├── An app drags an out-of-view column back -> put back at its sliver or parking spot
├── Heal never re-homes or exempts a column of a visible strip: the strip places it
├── Display added or removed -> parked columns re-parked; peeks past a new seam parked
└── Ghostty native-tab replacement -> keeps the slot, its position and its width
```

Unit (FakeBackend), `src/app/tests/strip.rs`:
`lab_off_refuses_the_strip_and_tiles_as_usual_until_it_is_turned_on`,
`stepping_along_the_strip_peeks_parks_and_reveals_columns_durably`,
`focus_from_outside_spaceman_scrolls_to_a_parked_column`,
`pause_and_quit_restore_every_peeking_and_parked_column`,
`switching_away_parks_every_column_and_back_reveals_only_the_viewport`,
`reordering_and_width_presets_follow_the_focused_column`,
`turning_the_strip_off_reveals_every_column_in_the_split_layout`,
`ghostty_native_tab_replacement_keeps_the_strip_column_and_width`,
`a_strip_column_that_drifts_back_on_screen_is_put_back`,
`docking_a_display_parks_columns_that_peeked_past_the_new_seam`,
`a_focus_step_glides_and_settles_into_the_rest_layout`, `rapid_taps_retarget_one_glide`, and
`switching_workspaces_halts_a_running_glide`, `a_cycle_mid_glide_keeps_a_column_caught_off_screen`,
`gliding_across_a_shared_edge_and_back_never_exempts_a_column_from_parking`,
`a_column_arriving_across_a_shared_edge_takes_focus_at_once_on_top`, and
`rapid_taps_step_from_the_aimed_column_and_retry_a_refused_one`. Curtain and free-edge slide-in:
`a_column_crossing_a_shared_edge_stays_under_the_neighbor_sliding_across_it`,
`free_edges_tuck_nothing`, `the_column_taking_focus_glides_in_on_top`, `a_column_arriving_past_a_free_edge_slides_in_from_its_sliver`. Slide at a
shared edge: `sliding_past_a_shared_edge_keeps_each_column_centered_on_its_own_display`,
`sliding_parks_at_rest_and_still_curtains_the_rest_of_the_way` and
`sliding_lets_a_column_overhang_a_shared_edge_up_to_its_center` (`src/motion/tests.rs`). Glide
tuning: `the_defaults_are_the_default_spring`, `a_shorter_glide_is_a_proportionally_faster_spring`,
`default_spring_is_critically_damped_and_settles_a_column_move_in_180_ms` (`src/motion/tests.rs`),
`tuning_round_trips_and_stays_out_of_the_file_at_its_defaults`, `tuning_out_of_range_is_refused`
(`src/labs/glide.rs`), `retuning_a_spring_keeps_its_motion_continuous`,
`strip_glide_tuning_round_trips_and_leaves_defaults_out` (CLI, `src/cli/tests/labs.rs`),
`glide_tuning_round_trips_and_leaves_its_defaults_out` (Settings, `agent/src/rules/glide.rs`) and
`pickers_offer_the_presets_and_a_custom_value_from_the_file` (`agent/src/labs/tuning.rs`). Customize
sheet: `customize_opens_the_strip_sheet_and_only_while_the_strip_is_on` (`agent/src/labs.rs`) and
`every_key_in_the_sheet_follows_the_file` (`agent/src/labs/customize.rs`). Ribbon,
camera and glide math: `src/strip/tests.rs`
(minimal scroll, peeks, seams, 120 Hz intermediate frames, retarget continuity). Enhanced UI hand-off:
`a_restore_owed_during_a_hold_is_paid_when_the_last_sink_closes` (`src/macos/presenter/holds.rs`).
Persistence: `persisted_strip_round_trips_and_excludes_fullscreen_layout`. Config:
`parses_labs_with_every_experiment_off_by_default`, `labs_switches_round_trip_and_survive_other_edits`
(CLI), `labs_switch_round_trips_and_leaves_an_all_off_table_out` (Settings). Shortcut gating:
`scrolling_strip_shortcut_registers_only_while_its_lab_is_on` (also a picked chord),
`the_strip_shortcut_defaults_to_option_shift_s_and_reads_any_chord`,
`chords_print_back_in_config_syntax_and_as_menu_symbols`, and the recorder's
`recording_the_strip_shortcut_keeps_its_lab_on` (`agent/src/panes/shortcuts/recorder/tests.rs`).

## How To Get To It (User POV)

```ts
Settings › Labs -> Scrolling Strip switch on
Focus a window on the workspace
└── Option+Shift+S (or the shortcut picked on the card) / spaceman ctl strip -> toggle
Settings › Labs -> Customize… -> the strip, Glide / Feel / Next to another display, and its keys
└── spaceman config set labs.scrolling_strip_duration_ms 380   (likewise _damping, _shared_edge)
```

## Existing Checks

- Unit tests above. No live harness drives the strip against real apps yet; the glide was recorded
  by hand in a macOS VM.

## Gotchas

- **Glides write positions only.** A width change resizes the column at once, then the camera and
  its neighbors glide. `agent.log` logs each glide (`strip glide … over Nms`) and how many frames the
  main thread presented.
- **Apps take positions at their own rate.** Each app's lane drops frames the app hasn't taken
  yet, so a slow app (TextEdit took about every other frame in the VM) moves at a lower rate, and
  the gaps between columns wobble mid-glide. Every column still lands on its exact position.
- **No gaps or padding yet.** Settings › Layout's gaps, edge padding and single-window width don't
  apply to a strip: its columns run edge to edge.
- **Shared edges.** A column partly in view on the side of another display is kept inside its own
  display, so it sits under its neighbor instead of running off the edge. That only looks right
  while the neighbor is stacked above it: the glide raises the neighbor when the window server
  says otherwise. The column a key moves to is the exception: it is focused at once, so it
  shows on top at its rest (Tuck) or half past the edge (Slide) from the first frame rather
  than waiting under its neighbor. Before the curtain, a column leaving across the seam stayed on screen after the glide; heal
  then took it for its app un-parking itself and, the second time, exempted it from parking for
  the session.
- **Slide needs separate Spaces.** `scrolling_strip_shared_edge = "slide"` relies on macOS drawing
  a window only on the display that holds its center, which is what "Displays have separate
  Spaces" (on by default) does. With it off, the half of a column past the edge shows on the other
  monitor mid-glide. Slide only changes the glide and a partly visible column's rest: a column that
  ends out of view is parked as in Tuck, never left resting past the edge.
- **Peek sliver vs. macOS clamping.** macOS keeps a window's title bar below the menu bar but
  doesn't pull a window back horizontally: parking keeps 1 point on screen and stays put, and the
  strip's 16-point sliver is wider than that.
- **A closed column's neighbor appears on the next cycle.** Reconciliation retires the closed window
  first; the column that comes into view is laid out by the following observation cycle.
