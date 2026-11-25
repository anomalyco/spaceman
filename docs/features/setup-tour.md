# Setup Tour

The first-run tour, and the same four steps from Settings › General › Setup tour › `Run`. It asks
for Accessibility, shows a suggested arrangement of the windows already open, and starts tiling only
when you say so.

## Sub-features

```ts
Welcome                                                              // tour.welcome
└── "Spaces, tiled." + Get started; nothing has moved

Accessibility                                                        // tour.permission
├── Allow -> System Settings; granted -> Allowed, then on to the next step after 700 ms
├── A row only for what is really on: Stage Manager, macOS edge tiling, another window manager
│   └── Turn off writes com.apple.WindowManager (undone in System Settings); Quit asks the app
└── No Launch at login here: the step asks for one thing

Your workspaces                                                      // tour.arrange
├── One card per workspace ("1 · ⌥1"), real app icons in Dwindle's frames
├── Suggestion: Web 1, Code/Terminal 2, Chat/Mail 3, Docs/Notes/Design 4, Media 5
│   ├── At most 2 tiles a workspace on a display <= 1728 pt wide, 3 on a wider one
│   ├── An app with many windows is split evenly over workspaces (4 browser windows -> 2 + 2)
│   ├── Two displays: Web and Code on the main (1-2), Chat and Docs on the second (6-7)
│   └── Unknown app -> its LSApplicationCategoryType, else the Docs workspace
├── Click an icon -> next workspace; drag it -> any other card, or the Floating line
├── "Floating: Calculator · Stickies" only when such utilities are open; press a name to tile it
├── Arrange & start -> the open windows move, tiling starts; no routes. Saves a float rule only
│   for a floated app Spaceman would tile, and a tile rule for a tiled built-in float
│   (src/defaults.rs, the same table the board floats by)
├── Skip -> tour ends, every window where it was, tiling paused until Resume or next launch
└── One window or none -> "Nothing to arrange yet", primary reads Start tiling

Try it                                                               // tour.try
├── A miniature desktop: two workspaces of two windows, a menu bar, a Workspace HUD capsule
├── Four moves, by key or by click: ⌥2, ⌥1, ⌥⇧Space (float), ⌥⇧F (Fullscreen Layout)
│   ├── Each ticks green the first time it changes the miniature; four -> "All 4 mastered"
│   ├── The chords follow [shortcuts] (Base modifier, rebinds); free play goes on afterwards
│   └── Reduce Motion: workspaces and windows cross-fade instead of travelling
├── Setup window has the keyboard -> global hotkeys held off; a chord moves only the miniature
│   └── Another window in front -> the real shortcut runs and the miniature follows along
├── The window floats above the tiles while this step shows; it never advances by itself
├── Launch at login checkbox
├── Undo & restore -> tiling paused, every window back at its frame from before Arrange
└── Done

Leave without the menu bar                                           // tour.leave
└── Settings › General: Quit Spaceman (Pause & restore windows · Quit)
```

Unit, `src/onboarding_plan/tests.rs` (the classifier, pure):
four browser windows on a laptop, mixed tiled and floating apps, the dual-display split, the
category fallback, even overflow, and that an inventory carries no titles.

Unit (FakeBackend), `src/app/tests/arrange.rs`:
`an_arrangement_spreads_open_windows_over_workspaces_without_cramming_one`,
`undoing_an_arrangement_puts_every_window_back_and_forgets_it`,
`arranging_again_moves_windows_a_workspace_already_owns`,
`an_arrangement_leaves_ignored_windows_and_unmanaged_workspaces_alone`,
`the_setup_inventory_carries_no_titles_and_counts_parked_windows`,
`toggle_float_on_spacemans_own_window_does_nothing`.

Unit, `agent/src/onboarding/tests.rs` (flow, board and fixtures):
`the_permission_step_has_no_launch_at_login_and_try_it_holds_it`,
`try_it_never_advances_by_itself`,
`skip_leaves_the_windows_alone_and_undo_goes_back_to_the_plan`,
`arrange_and_start_saves_no_rules_for_utilities_that_float_by_default`,
`tiling_a_built_in_float_on_the_board_saves_a_tile_rule`,
`the_board_moves_a_tile_by_click_or_drag_and_floats_by_drop`,
`a_conflict_shows_only_when_its_switch_is_on`.

Unit, `agent/src/onboarding/demo/tests.rs` (the Try it miniature):
`switching_to_two_goes_there_and_counts_only_when_the_desktop_moves`,
`switching_back_to_one_counts_after_going_there`,
`floating_lifts_the_focused_window_and_the_other_takes_the_screen`,
`fullscreen_layout_toggles_and_keeps_the_other_window_behind`,
`four_moves_complete_it_in_any_order_and_free_play_goes_on`,
`the_default_chords_are_heard_by_key_code`,
`a_custom_base_modifier_moves_every_chord_and_the_window_hears_what_the_tap_cannot`,
`a_press_sinks_its_keys_for_a_beat_and_a_held_modifier_for_as_long_as_it_is_held`,
`the_ring_shows_once_when_the_last_move_is_made`,
`a_press_counts_only_on_try_it_and_undo_starts_the_desktop_over`.

Motion (the real view at 120 Hz, clicked and typed at), `agent/src/onboarding/motion_tests.rs`:
`a_click_makes_the_move_and_the_workspaces_slide_with_the_marker_stretching`,
`a_click_gives_the_stage_the_keyboard_and_real_keys_make_the_other_moves`,
`arriving_from_the_arrangement_gives_the_stage_the_keyboard`,
`floating_lifts_the_window_off_the_grid_while_the_other_springs_wide`,
`fullscreen_fills_the_screen_and_leaves_a_tab_of_the_window_behind`.

Unit, `src/app/app_loop/live_shortcuts.rs`:
`a_recording_holds_for_its_limit_and_the_tour_for_as_long_as_it_practices`.

Scenes (`--scene`): `onboarding-welcome`, `onboarding-permission` (`-waiting`, `-ready`,
`-conflict`), `onboarding-arrange` (`-dual`, `-drag`, `-empty`), `onboarding-try-it`
(`-ws2`, `-floated`, `-fullscreen`, `-complete`).

## Known gaps

- No live harness or gauntlet step drives the tour: the arrangement on real windows, the Try it key
  tap, the hotkey hold while the setup window has the keyboard, the floating window level, and both
  `Turn off` writes are covered by unit tests and scenes only.
- The key tap hears a Try it chord only if it holds Option. A chord without Option is heard by the
  setup window's own key events, so only while the miniature has the keyboard; clicking its keys
  always works.
- The miniature always has the same four windows (Safari, Terminal, Notes, Mail), not the ones open.
- A window that opens between the plan being drawn and `Arrange & start` follows its app's last
  slot; an app that was not open then is tiled where you are, as always.
