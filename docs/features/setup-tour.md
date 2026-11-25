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
├── Arrange & start -> the open windows move, tiling starts; saves float rules only, no routes
├── Skip -> tour ends, every window where it was, tiling paused until Resume or next launch
└── One window or none -> "Nothing to arrange yet", primary reads Start tiling

Try it                                                               // tour.try
├── Press ⌥2, ⌥1, ⌥⇧Space on the real workspaces; each ticks green when heard
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
`arrange_and_start_saves_only_float_rules_for_small_utilities`,
`the_board_moves_a_tile_by_click_or_drag_and_floats_by_drop`,
`lessons_tick_on_the_real_shortcuts_and_coming_back_needs_going_there`,
`a_conflict_shows_only_when_its_switch_is_on`.

Scenes (`--scene`): `onboarding-welcome`, `onboarding-permission` (`-waiting`, `-ready`,
`-conflict`), `onboarding-arrange` (`-dual`, `-drag`, `-empty`), `onboarding-try-it`
(`-progress`, `-complete`).

## Known gaps

- No live harness or gauntlet step drives the tour: the arrangement on real windows, the Try it key
  tap, the floating window level, and both `Turn off` writes are covered by unit tests and scenes
  only.
- Try it hears a shortcut only if it holds Option and has a fixed key code. A rebound chord without
  Option, or on a letter key, is shown but never ticked.
- A window that opens between the plan being drawn and `Arrange & start` follows its app's last
  slot; an app that was not open then is tiled where you are, as always.
