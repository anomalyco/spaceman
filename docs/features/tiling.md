# Tiling

Arrange a workspace's windows as non-overlapping tiles in nested horizontal/vertical splits, and
navigate, reorder, nest, resize, and balance them from the keyboard or the pointer.

## Sub-features

```ts
Automatic tiling                                        // tile.auto
├── New managed window -> joins the last focused tile’s split, or the root without a surviving focus
├── Window closes -> its slot retires, siblings reflow
│   └── App keeps the closed window alive off screen -> slot retires after two seconds
└── Ghostty native tab replaces its window -> same slot, order, weight, nesting

Focus                                                   // tile.focus
├── Option+H/J/K/L / ctl focus <dir> -> nearest tile in that direction, crossing displays
│   └── In Fullscreen Layout -> previous/next tile in stable order (see fullscreen-layout.md)
└── Option+` / ctl focus next        -> next tile in stable order, wrapping

Reorder                                                 // tile.reorder
├── Option+Shift+H/J/K/L / ctl move-direction <dir> -> swap/move within the tree
│   ├── At the workspace edge -> moves to the adjacent display's active workspace
│   └── Perpendicular move    -> re-orients or creates a mixed split
└── Pointer drag
    ├── Over a sibling     -> live-previewed swap, applied on mouse-up
    ├── Near a tile edge   -> nested split
    └── Onto another display -> moved to that display's active workspace

Size                                                    // tile.size
├── Option+- / Option+= / ctl resize shrink|grow -> 50 px at the immediate split
├── Pointer resize       -> split weights follow both edges of a corner drag, preview during the gesture
├── ctl balance          -> equalize the focused tile's immediate split (no default hotkey)
└── Window refuses its tile size (app minimum) -> its tile grows to fit and siblings give up
    the space; if necessary, try stacking an adjacent pair

Axis                                                    // tile.axis
└── Option+/ / ctl layout -> flip the root split axis (also exits Fullscreen Layout)
```

## How To Get To It (User POV)

```ts
Keyboard: Option+H/J/K/L, Option+Shift+H/J/K/L, Option+`, Option+-, Option+=, Option+/
Pointer:  drag a managed window (reorder/nest/move display), drag its edge (resize)
CLI:      spaceman ctl focus | move-direction | resize | balance | layout | retile
```

Input arriving while the worker is busy queues in order (up to 64); queued input is dropped on
pause, quit, or hotkey loss.

## Existing Checks

- Unit (FakeBackend), `src/app/tests/layout.rs`: `focus_navigation_follows_linear_tile_order`,
  `focus_navigation_crosses_to_the_adjacent_display`,
  `workspace_focus_cycle_wraps_through_tile_order_in_every_layout`,
  `keyboard_resize_applies_and_persists_the_focused_layout`,
  `balance_applies_and_persists_the_focused_parent_split`,
  `reorder_moves_to_the_adjacent_display_at_a_tile_boundary`,
  `perpendicular_reorder_creates_a_mixed_split_for_three_windows`,
  `layout_toggle_repartitions_and_persists_the_focused_workspace`.
- Engine and layout tree: `new_window_joins_the_focused_nested_split` and
  `corner_drag_moves_both_nested_boundaries` cover insertion and diagonal resizing.
- Unit (FakeBackend), `src/app/tests/gestures.rs`: drag swap preview, edge-nesting preview,
  cross-display drop; `startup.rs` covers gesture classification and `src/app/input.rs` covers
  queued final gestures.
- Unit (FakeBackend), `src/app/tests/reconciliation.rs`:
  `closed_window_its_app_keeps_ordered_out_gives_up_its_tile` (an app closes its window by ordering it out),
  `workspace_whose_windows_all_leave_the_screen_keeps_its_tiles` (another Space is showing).
- Unit (FakeBackend), `src/app/tests/minimums.rs`: a window that refuses its tile width gets a wider
  tile (middle, last, several at once, and observation-cycle cases); a three-window row that cannot fit stacks two neighbors, and a wider display restores the requested row; Fullscreen Layout ignores minimums. `src/layout_tree.rs`:
  `fitting_minimums_reaches_into_nested_splits_and_leaves_impossible_splits_alone`.
- Fixture replay: `cargo run -- replay fixtures/ghostty-tab-replacement.json` (reconciliation of a
  native-tab replacement, without the runtime).
- Live harness (not run): `scripts/live-ghostty-tabs.sh` for native-tab slot preservation.

## Gotchas

- Tests check requested frames; apps with minimum sizes may refuse them. Spaceman grows such a
  window's tile to the size it kept, taking the space from its siblings in proportion to what each
  can spare, and logs a `minimum size:` line in `agent.log`. Minimums are learned per window, only
  from refusals visible right after the write, and kept until the window closes or Spaceman
  restarts. Apps that clamp
  later (Electron aspect ratios) are still recorded as constrained instead of fought every cycle.
  If the requested split cannot fit, Spaceman tries grouping one adjacent pair of root children
  on the other axis. It checks both width and height and keeps window order. Automatic fits
  are temporary; the requested layout returns when it fits again. If no pair solves the shortage,
  windows may overlap. A sustained overlap involving a constrained tile appears in `spaceman doctor`
  as `constrained_overlap`.
- Some apps (many SwiftUI apps) close a window by ordering it out; the window server keeps
  listing it. Once its app has no window on screen, its tile is released after two seconds instead
  of staying as an empty gap. Minimizing an app's only window or hiding the app (Command-H) does the
  same; the window comes back as a new tile at the end of the layout. While nothing on the display
  is observed (another macOS Space is showing), tiles are kept.
- `balance` has no default hotkey; it is CLI-only.
- A window that enters macOS native fullscreen (for example browser video) loses its slot after two
  seconds and returns as a new tile at the end of the layout
  ([known defect](fullscreen-layout.md#gotchas)).
- Floating windows (rules, modal/dialog windows without an enabled fullscreen button, or non-resizable
  windows) are owned by a workspace (and can be moved between workspaces) but are never tiled or
  resized.
- New windows open on the active workspace of the display under the window's center unless matched by
  an explicit `route` rule or startup placement recall.
