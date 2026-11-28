# Workspaces

Keep windows grouped on numbered or named virtual workspaces per display, switch between them
instantly, and send windows or whole apps to a workspace, without using macOS Spaces.

## Sub-features

```ts
Switch workspace                                           // ws.switch
├── Option+1..0 / Option+A..Z / spaceman ctl switch <ws> / a workspace in the menu bar menu
├── Incoming parked windows revealed -> laid out -> outgoing windows parked off-screen
│   ├── A window minimized (⌘M) or hidden (⌘H) while away -> back at its frame but left minimized
│   │   or hidden and never focused; a tile leaves the layout and rejoins as a new tile
│   ├── A fixed-size window under a `tile` rule -> moved to its tile's corner at its own size;
│   │   the layout still counts as done, so the switch completes
│   └── Reveal not confirmed by the window server in time -> switch rolled back, old workspace stays
│       (a window whose app was not responding stays hidden when that app recovers); the same
│       when a window sits on a macOS desktop that is not showing and cannot be laid out: it
│       goes back to its parking spot there (#775)
├── Focus -> last-focused tile of the incoming workspace, keyboard focus included
│   (that window raised and made main, then its app brought forward with `AXFrontmost`), unless
│   one of its windows already has focus (e.g. focus follow): that window keeps it. If the app
│   hands focus to another of its windows on a hidden workspace as it comes forward, that
│   window's workspace is not revealed and the chosen window is focused again, even when the
│   app does so up to 1.5 s later with no key press or click in between (once per window)
└── Already active -> stays and re-focuses its remembered tile (keeps current focus if it is there);
    a remembered window minimized or hidden since is skipped and stays set aside
    ├── Pressed again (`Option+<ws>` / `ctl switch`) while focus is on that workspace -> the
    │   next tile takes focus, wrapping at the end of the tile order; the way to walk the shared
    │   frame in Fullscreen Layout
    ├── Focus on another display's workspace -> only brings focus back; the press after that cycles.
    │   The workspace it left stays on screen and remembers its focused window (tile or float)
    └── Menu bar or HUD click on the active workspace -> never cycles, keeps the current focus

Swipe                                                      // ws.swipe
├── Three-finger swipe left/right -> next/previous workspace in the menu section of the display
│   under the pointer (visible + occupied, in workspace order), stops at either end
├── Natural scrolling off (Trackpad settings) -> the directions flip, as with macOS Spaces: swipe
│   right -> next, left -> previous. Read on every swipe
├── Suppressed while a pointer button is down or a window drag is active (including macOS
│   Accessibility "Three Finger Drag")
├── An empty or partial trackpad report mid-swipe (fingers still down) no longer restarts the
│   swipe; only a reported lift, fresh fingers landing, or such reports lasting 0.15 s end it
├── Labs › Swipe switch on ([labs] swipe_switch) + [settings] trackpad_swipe = false (Settings ›
│   Shortcuts › Swipe to switch workspace) -> swipes never switch; macOS keeps the gesture. Reloads
│   live. With the Lab off, trackpad_swipe is ignored and the Shortcuts row stays fixed
└── Labs › Four-finger swipe on ([labs] four_finger_swipe) -> the swipe takes four fingers and
    three-finger swipes stay with macOS (for Accessibility "Three Finger Drag"). Reloads live

Step to the next or previous workspace (Lab: [labs] workspace_steps)  // ws.step
├── ctl switch next|prev / a [shortcuts.bindings] chord (no built-in one) -> the swipe's stops
│   (visible + occupied, in workspace order) on the focused display, stopping at either end;
│   with no visible window focused, the display move-workspace would use. A focused floating
│   window (pinned, or kept by its workspace while Dragged Floats Follow Their Display is off)
│   counts as the display it is on, even after a drag away from its workspace's display and
│   with its center in that display's menu bar strip
├── ctl move next|prev [--follow] -> the focused window to the next/previous enabled workspace of
│   its display, empty ones included, exactly as move <ws> [--follow]; stops at either end
└── Lab off -> bindings skipped at registration, ctl reports "Workspace steps are off"

Back-and-forth                                             // ws.back-and-forth
├── Option+Tab / ctl back-and-forth -> the workspace focused before this one, on any display
│   (one global focus history; after a restart it starts from the focused display's lane history,
│   even when another selector for that display assigns the previous workspace)
├── A focus command still settling counts: a lagging Accessibility report of a window it left
│   rewrites no history
├── A focused pinned window counts as the workspace its display shows, never its hidden home
├── Target already on screen -> focus moves there, no display changes, nothing is parked
│   ├── Its app handing focus to its other window on another display as it activates -> focus
│   │   goes back to the target's window (once, while the request settles); a pinned one
│   │   counts as the workspace its display shows, so a pin on the target's own display
│   │   keeps focus
│   └── Press again -> bounces back to the workspace the focus came from, refocusing the
│       window (tile or float) it left there, including a pin from another workspace while it
│       still sits on that display
├── Target on screen but empty -> only shows its HUD; focus history is left alone
└── Target hidden -> its own display switches to it and focuses its remembered tile

Move workspace to another display                          // ws.move-workspace
├── Option+Shift+Tab / ctl move-workspace [next|prev|<display>]
├── Orders displays left-to-right; wraps around for next/prev; persists assignment to config.toml
│   in place, keeping its rules (including title_contains), comments and formatting; removing an
│   entry follows the same comment rule as Settings (see docs/configuration.md): a comment set off
│   by a blank line stays, one directly above the entry goes with it, and an emptied inline
│   `workspaces = [...]` is dropped with its remaining comments moved out
├── With no visible window focused (an empty workspace), moves the workspace switched to last
├── [labs] empty_workspace_focus: also after choosing an empty workspace on another display while
│   a window elsewhere keeps focus (see "Empty workspace chosen on another display" below)
├── Reveals & lays out source display's previous workspace -> lays out moved workspace on target
│   -> parks target's displaced workspace (which is only that display's lane history, even when
│   another selector for the same display, such as a UUID, assigns it); the replacement is the
│   workspace the source display showed last, whichever selector assigns it
├── A replacement window minimized (⌘M) or hidden (⌘H) while away stays minimized or hidden, as
│   on a switch; a tile leaves the layout and rejoins as a new tile
├── A float carried to a shorter display (its center off every display) still comes back
└── Preserves Fullscreen Layout and focus on the moved workspace (focus history untouched); HUD on both displays

Empty workspace chosen on another display (Lab)            // ws.empty-choice
├── Off by default; Settings › Labs › Workspaces › Focus empty workspaces on other displays,
│   [labs] empty_workspace_focus = true, or spaceman config set labs.empty_workspace_focus true
├── Showing an empty workspace focuses nothing, so the window on the display you came from keeps
│   keyboard input and stays reported as focused; with the Lab off, commands follow that window
├── Chosen with Option+<ws>, ctl switch, the menu or the HUD, on a display other than the focused
│   window's (a window Spaceman does not manage, such as one an ignore rule matches, counts by the
│   display it is on) -> it is the focused workspace for commands, wherever it shows:
│   ├── Option+Shift+Tab moves it; the other display's workspace stays and nothing about it is saved
│   ├── Commands that need a window (Option+Shift+F, Option+Shift+H/J/K/L, Option+Shift+<ws>,
│   │   Option+Shift+Space) report there is none, as on an empty workspace on the display in use
│   └── Option+H/J/K/L starts from its display and focuses the nearest tile that way
├── Ends when another window takes focus, Spaceman focuses a window (Option+H, Option+<ws> of
│   another workspace), the workspace gains a window or leaves the screen, a display is connected,
│   unplugged or rearranged, or the Lab is turned off
├── Clicking the window that already had focus does not end it: Accessibility reports no change
├── Unchanged: Option+Tab, an empty workspace chosen on the display in use, and a chosen workspace
│   with windows (the window that takes focus decides)
└── Not affected: where a new window opens, and typing (the window that kept focus keeps the keys)

Move window without switching                              // ws.move-window
├── Option+Shift+<ws> / ctl move <ws> -> window parked on target, current workspace reflows and
│                                        hands focus to a remaining visible window (if any); a
│                                        window minimized or hidden there stays set aside (#743)
├── Within ~0.7 s after macOS reports a desktop change -> waits for the layout hold, retiles, then
│   moves the focused window of the arriving desktop (#770); the same wait covers focus, resize,
│   layout, float and pin commands
├── After three or four fingers slide sideways on the trackpad (until ~1.5 s after they lift),
│   even before macOS reports the change -> waits the same way, then moves the arriving desktop's
│   focused window (#777)
├── Target on screen on another display -> tiled there (a floating window is carried over, same
│   size and relative place); focus follows it and Option+Tab goes back
├── Target hidden on another display -> parked off the edge of the display it is on; its recovery
│   frame moves onto the target's display, so showing the workspace brings it up there
├── ctl move-app <bundle-id> <ws>     -> every current window of that app, tiled or floating, in one request
│   └── Another native tab selected while the move runs -> when a moved tile's tab is ordered out
│       within 2 s of the move and the one unowned window of the app is where that tile was, that
│       window takes the moved tile, so the tab group follows to the target (#759)
└── [labs] float_follows_display: a float dragged with its center onto another display moves to the
    workspace showing there, keeping its frame (src/app/commands/float_drop.rs); same display, no
    workspace there, pinned or Scratchpad windows, and app or macOS moves change nothing

Send tab to workspace                                      // ws.move-tab
├── ctl move-tab <ws> [--follow] / a [shortcuts.bindings] chord (no default chord)
├── Presses the focused app's "Move Tab to New Window" (AXIdentifier moveTabToNewWindow:, else its
│   title: English, Chromium's plural, or any of AppKit's MenuCommands.loctable translations)
├── Disabled or missing (one tab, or no native tabs) -> moves the whole window, exactly like move
├── Pressed -> PendingTabDetach (800 ms): the app's first window not on screen before the press is
│   routed straight to <ws> ahead of any route rule (parked when hidden, never tiled on the source;
│   a floating tab moves onto <ws>'s display when <ws> is on screen elsewhere)
│   ├── Chromium: the new window is the tab; the source window keeps its slot
│   └── AppKit tab group: the moved tab keeps its window and the group's next tab appears in place,
│       so that window takes over the source slot (told apart by which one left the old frame)
├── Without --follow -> focus goes back to the window that stays; with --follow -> switches to <ws>
│   and focuses the tab
├── A newer move-tab while one is pending -> observes until the pending tab is claimed for its own
│   <ws> or expires, then presses (issue 570)
├── The press acts on the app's key window, so a newer move-tab keys the tab group it means first:
│   the group's next tab when the earlier tab was sent away (even if that settled just before the
│   newer command ran), without letting focus follow chase it on a hidden workspace; without
│   --follow, focus then goes back to the window it was on (issue 597)
└── No window within 800 ms -> dropped; later windows open normally

Mouse follows focus ([settings] mouse_follows_focus)       // ws.mouse-follows-focus
├── "off" (default) -> never moves the pointer
├── "display"       -> on explicit keyboard/ctl focus changes (switch, back-and-forth,
│                      move-workspace, focus, move onto an active workspace, including
│                      Option+Shift+H/J/K/L off the edge onto another display), if the pointer is not
│                      already on the target display, warps to the center of the focused window (or
│                      the target display when empty)
├── "window"        -> same explicit triggers, warping whenever the pointer is outside the newly
│                      focused window (or empty display)
├── Goes to the frame the window accepted when it refused its tile's size, and follows
│   move-tab --follow onto the detached tab's display
└── Never warps on passive cycles, focus-follow, Cmd+Tab, mouse clicks, swipes, drags, or while
    any mouse button (left, right or middle) is held; re-associates mouse movement immediately
    after warping

Focus follows mouse ([labs] focus_follows_mouse, off)      // ws.focus-follows-mouse
├── Main thread (src/app/app_loop/hover.rs): the pointer moved, then rested ~150 ms -> Core
│   Graphics names the front window there (no AX); the menu bar, Dock, an open menu, or
│   Spaceman's HUD/Settings in front -> nothing
├── Worker (src/app/focus/hover.rs): a tile on a visible workspace -> focused, pointer untouched;
│   floats, the Scratchpad and unmanaged windows -> nothing
├── Skipped when the pointer has moved on, rests where mouse follows focus warped it, a button is
│   held, a drag or a Space slide is under way, or another window is in front of the point by the
│   time the worker runs the hover (a float a queued workspace switch showed; #539)
├── A keyboard or `ctl` focus command or workspace switch queued while the pointer is still
│   settling cancels that rest; only a fresh move hovers again (#732; src/app/app_loop/hover.rs)
└── The warp point is skipped only until the pointer leaves it: moving away (resting elsewhere or
    not) and back to that exact point hovers again (#522; src/app/tests/focus_follows_mouse.rs)

Window overlay ([labs] window_overlay, off)                 // ws.window-overlay
├── Main thread (src/app/app_loop/window_overlay.rs, src/window_overlay.rs): the base modifier and
│   Shift (Option+Shift) held alone for 150 ms (320 ms within 2 s of a key or hotkey), no button
│   down, tiling on -> the worker reads every visible window 60 ms into the hold (no AX on the main
│   thread) and an outline and a card show on each, on every display, when the hold completes
├── Any key or hotkey while held (Option+Shift+1, +H...) -> hidden, and not shown again until the
│   keys are let go; Command, Control or a mouse button at the start -> nothing; letting go -> hidden
├── Outline (agent/src/window_overlay/frame.rs): just inside each window; amber on the focused
│   window, brighter under the pointer, dashed on a float, cut where a float covers it; never
│   takes the pointer
├── Card (agent/src/window_overlay/card.rs, chips.rs): app icon and name (never a title) with a
│   focus dot; a status line (Workspace N / On every workspace, or what the hovered control would
│   do, with the destination's app icons); a chip per workspace that sends the window there
│   (current bright, occupied with up to three dots, empty dim, other displays after a rule);
│   Float, Pin, Fullscreen (tiles), lit while on; arrows to swap with a neighbouring tile
├── Placement (agent/src/window_overlay/place.rs): centred on the largest part of the window that
│   no float in front covers, whole inside the display, never on another card; a window narrower
│   than the full card gets the compact one (chips wrap at five, icon-only buttons)
├── Buttons act on the card's window, focused or not, through the focused-window commands' own code
│   (src/app/window_overlay.rs); no activation, focus change or pointer warp; cards refresh while held
└── Fullscreen Layout shows only the front tile's card. Every card requires fresh on-screen
    WindowServer bounds intersecting its display beyond the parking strip; hidden, minimized,
    dismissed panels (even with a live AXWindow), missing and zero-size windows have none.
    Covered by src/app/tests/window_overlay.rs

Focus follows                                              // ws.focus-follow
├── Focusing a window that lives on a hidden workspace (Dock, Cmd+Tab, notification)
│   -> that workspace becomes active, with focus on that window (not its remembered tile)
│   └── Switch interrupted by that same activation -> retiles every 40 ms for 500 ms and follows it
│       as soon as Accessibility catches up
│   └── Activation slides macOS to another Space (Cmd+Tab out of a native fullscreen app)
│       -> follows once the slide settles; a follow the slide made fail is retried, not re-homed
├── Focusing a window on another display's visible workspace -> no switch; Option+Tab remembers it
├── Focusing a pinned window (`action = "pin"`, `ctl pin`, a `pin` shortcut) -> never switches: it shows everywhere
└── No focused window reported -> follows the frontmost app only when all its windows, tiled
    or floating, are on one workspace; windows the window server no longer lists (just closed,
    not yet retired) don't count, so closing all of an app's windows never switches (#516)

Where a new window goes (first match wins)                 // ws.adoption
├── Disposition: the first matching [[rules]] entry (bundle_id, title, title_contains), over the
│   built-in default (Stickies, Calculator, 1Password and other utilities float; src/defaults.rs),
│   over the window's traits (non-resizable floats), each deciding only what the ones before left
│   open (src/config/disposition.rs). action = "tile" tiles a built-in float again; a route rule
│   for a built-in float floats it on the routed workspace, and a bundle-only route rule floats
│   an app's first window there even when it is a dialog or splash (an Open/Save panel shown
│   before any other window is parked with a hidden routed workspace; #252, #687)
├── ignore rule        -> never managed
├── route rule         -> that workspace (parked if hidden), joined before any display reconciles,
│                         so a failed park leaves it unowned and the next cycle routes it again;
│                         bundle-only route rules apply to the app's initial window while
│                         secondary windows stay on the active workspace (a window floated by
│                         hand, sent to the Scratchpad, or floated by a rule after it tiled
│                         still counts as the initial one, #551, #664),
│                         unless the rule says windows = "all" and [labs] route_all_windows is
│                         on (src/config.rs, src/app/cycle/affinity.rs); in Columns it joins
│                         the destination workspace's focused tile split as if it had opened
│                         there, or its root when no focused tile is left; in Dwindle it
│                         splits that workspace's focused tile, including one just clicked
│                         while it shows (#603) (src/app/cycle/routing.rs)
├── remembered placement (startup window, first 60 s, the app relaunched within 30 s of
│   quitting, a returning app, or a new process presenting 2+ windows at once whose memory spans
│   2+ workspaces) -> remembered workspace, matched by frame, then size, then creation order
│   (floating if matched by a float rule, dialog/non-resizable without a tile rule, or remembered
│   as floating; a remembered float moves to its remembered position on that workspace's
│   display, keeping the size the app reopened at); a claimed window whose first park to a
│   hidden workspace fails keeps its claim and is parked there again next cycle, a float at its
│   remembered position (#689)
├── float / pin rule, built-in float, dialog / non-resizable (no tile rule) -> floating on the
│   active workspace under the window's center (unless a route rule above sends it
│   elsewhere); each float belongs to its own workspace, so two
│   Stickies notes can live on workspaces 1 and 2
└── otherwise          -> tiled on the active workspace of the display under the window's center

Settings › Rules                                           // ws.rules-pane
├── Settings opens on General; Rules is a pane to visit (or spaceman://settings/rules)
├── `Where new windows open` ("Optional. Any window can move to any workspace"): a tile per
│   workspace with the apps routed there; drag an icon to another tile or to `Floating`
├── + on a tile, or `Add app`, lists every running app, ruled or not
│   ├── an app with no rule -> a rule for every window, at once
│   └── an app with a rule (its name ends in …) -> the `Title contains` sheet: the typed text
│       becomes title_contains, so Safari can open Work windows on 1 and Claude windows on 2.
│       The field starts empty and is never filled from a window's title. A filter another rule
│       of the app already has, or none while the app has an every-window rule, can't be added
├── A filtered rule is written ahead of the app's every-window rule (first match wins) and its
│   icon wears a dot; right-click a rule -> `Title contains…`, `Float` or `Pin on all
│   workspaces`, `Remove rule`
└── `Floating by default`: a quiet chip for each running built-in float (src/defaults.rs) with no
    rule of its own, and for any a rule tiles. Its menu: `Float` (no rule), `Tile`
    (action = "tile"), `Pin on all workspaces` (action = "pin", which moves it to `Floating`)

Toggle Float                                               // ws.float
├── Option+Shift+Space / ctl float / the Window overlay's Float -> a tile leaves its workspace's
│   tiles, keeps focus, and the remaining windows re-fill its space
├── A first float lifts the window to a card centered on its display's usable work area: 72% × 78%
│   of it, capped at 1280 × 860 points, and never wider or taller than the tile it leaves (a narrow
│   column keeps its width, a short row its height)
├── A window that floated before returns to the frame it last floated at, recorded when float or pin
│   tiled it again; on a display whose work area changed it is clamped onto that display. The frames
│   live in memory only, and a closed window's is dropped
├── An app that clamps its own size (a minimum, an aspect ratio, an open sheet) keeps the position
│   it accepted and stays floating
└── Another press, ctl float, or the overlay's Float tiles it again in stable order

Pinned floats                                              // ws.pin
├── action = "pin" rule, spaceman ctl pin, or a [shortcuts.bindings] chord bound to "pin" on the focused window (floats it first, with Toggle Float's lift; again unpins)
├── Never parked by a switch, a move or a display change: shows on every workspace of its display
├── Left on a macOS desktop that isn't shown, it stays pinned (or a hand float stays floating) even
│   when its app has a window on the desktop shown, and comes back that way (#768)
├── Keeps its owning workspace for memory and the HUD; unpinning hands it to the workspace showing it
├── A pin rule added while the window is parked brings it back (matched by bundle and the full title
│   it last showed, so an earlier title rule still wins; title rules wait until it is observed);
│   a parked tile leaves its workspace's tiles and floats there first, owned by that workspace
├── Rule pins are not saved; after a restart they match again, even for an app whose windows are
│   all minimized and so never observed: such a window is matched by its app's bundle and its
│   current title, read from the running app (an earlier title rule still wins; while its title
│   cannot be read and a title rule could decide, it is not pinned, #697)
└── Hand pins persist in workspace-state.json (`pinned`); pause, quit and restart restore as usual

Reopened at a parking spot                                 // ws.stale-parked
├── An app quit while some windows were parked reopens them one pixel inside a display corner
├── Every such window Spaceman neither parked nor owns (not only the focused one) is brought onto
│   that display, cascaded 24 pt apart, then recalled or adopted like any new window
├── Always-on-top windows stay where they are: Spaceman ignores them, so it never moves one there
├── A window an `ignore` rule matches, by app or title, stays where it opened; it is judged once
│   and not read again until its frame or the displays change
└── One whose title a rule needs but Accessibility cannot resolve yet waits, retried with the
    same backoff as a window that cannot move

Displays                                                    // ws.displays
├── Default: 1-5 main, 6-10 secondary; [[workspaces]] monitor = main | secondary | UUID | name
├── Settings › Workspaces offers every connected display as its own picker segment and card:
│   Main and Secondary with two displays, and past two Main then each display by its own name,
│   numbered when two share one (Studio Display 1 / Studio Display 2; Display 2/3/4 in the
│   segments past three displays); picking one writes its UUID, the same selector
│   Option+Shift+Tab persists, and a saved UUID renders back as that display's name
├── A row set to a display other than its built-in default says Default: <that display> in the
│   picker's own words; a row already on its default says nothing, including one saved by the
│   UUID of the display that is also Secondary
├── Preferred display unplugged -> its workspaces fall back to main, return on reconnect; its
│   saved assignment keeps a Disconnected displays card with the picker on Not connected
├── Each display remembers, by its UUID, which selector's workspace it showed last, so two
│   selectors naming one display (secondary and its UUID) never compete: a reconnect, a restart
│   and a config edit that only changes the selector all keep the same workspace in front,
│   whether the edited workspace is the one shown or one hidden behind it
├── A workspace that moves display with a reconnect brings its floats from the display it left
├── Deleting the last declaration of an unused named workspace makes it unavailable at once
├── Display added or removed -> hidden windows re-parked against the new displays; visible owned
│   windows left with their center off every remaining display keep their slots and snap back onto
│   their workspace's display
└── Sleep: a window Accessibility cannot find while the window server still lists it keeps its
    slot; it is forgotten only after 30 s unreachable with a display awake, never during a
    background wake or lid sleep (#739); its app quitting or the window server dropping it still
    retires it at once

Workspace order ([settings] workspace_order)                // ws.order
├── The order the HUD row, the menu bar menu and three-finger swipes list a display's workspaces
├── Listed workspaces first, in the order given; the rest follow as 1-10, then A-Z
├── Unset or empty -> 1-10, then A-Z, as before
├── Empty workspaces flicked through while the HUD is up stay in the row where the order puts them
├── Only where workspaces are listed: never enables a letter; shortcuts, routes, and saved state
│   are unchanged
├── Unknown or repeated entry -> config rejected
└── Applies on config reload; removing it brings back the default order

Workspace HUD ([settings] hud, on by default)               // ws.hud
├── A switch, a window moved to another workspace, a split-axis or Fullscreen Layout toggle, or a
│   tiled window joining a visible Fullscreen Layout workspace
│   -> a capsule on every display, each with its own row: every workspace on that
│      display holding windows, plus the visible one, in workspace order; a marker on the visible one
├── hud_position = "bottom" (default) -> the capsule rests 60 pt above each display's bottom edge,
│   clear of the Dock; "top" -> 60 pt below its top edge, clear of the menu bar
├── hud_style = "icons" (default) -> each workspace as its number and its apps' icons (one per app,
│   │  tile order); the visible group bright, the others dimmed; nothing changes size
│   └── "layouts" -> each workspace as its layout with an app icon per tile, the visible one larger
│      (others .62×, dimmed); an empty one is a dot; Fullscreen Layout is one tile with the front
│      app's icon and the others smaller beside it
├── No text but workspace numbers, and never a window title; icons are looked up by bundle ID
├── Appears: fades in, scales from .96 and moves 6 pt into place (a bottom capsule rises, a top
│   one drops); stays 1.15 s after the last event; leaves the same way back in about 0.17 s
├── Interactive without stealing focus (OverlayInput::Regions, KeyPolicy::None):
│   ├── Hover over the capsule -> pauses auto-dismiss (and rescues a mid-exit capsule); resumes
│   │   0.6 s after the pointer leaves
│   ├── Click a workspace slot -> switches that display to that workspace without activating
│   └── Right-click (or Ctrl+click) an app icon -> native menu editing the app's one every-window
│       rule in config.toml, in two groups:
│       ├── how its windows sit: `Float <App>` or `Tile <App>` (a built-in float tiles with an
│       │   action = "tile" rule and floats again by losing it), and `Pin <App> on all
│       │   workspaces` (action = "pin") or `Unpin <App>`
│       └── where its new windows open: `Always open <App> on Workspace N` (action = "route"),
│           checked while it is the rule; picking the checked one removes it
├── Another switch while it's up, or on its way out -> the same capsule carries on from where it
│   is: the marker's leading edge sets off first and its trailing edge catches up; in Layouts the
│   cells grow and shrink and their neighbours slide. Only the display that switched moves its
│   marker; every capsule holds and leaves together
├── Shown again after hiding -> the marker (and in Layouts the large cell) starts on the
│   workspace the switch left and travels as the capsule fades in
├── A workspace joins or leaves the row -> it fades and scales in place; a moved window's icon
│   leaves one workspace and appears in the other; the marker stays with you
├── The same workspace's layout changes -> its tiles move to their new frames
├── More workspaces than fit -> the row scrolls to keep the visible one in view, its cut edge fades
├── Status toast (e.g. `Tiling paused — built-in display` / `Tiling resumed — external display`) ->
│   reuses the same non-activating capsule on each display even when `settings.hud = false`, holds
│   for 2.5 s before leaving in about 0.17 s, and updates in place when another toast arrives
└── Reduce Motion -> the same changes as a short fade, with nothing travelling or scaling
```

Hidden windows are **parked**, not minimized: moved past a display's bottom corner so at most a
one-pixel-wide slice of the title bar, which macOS keeps on screen, remains visible, with the original
frame persisted to `parked-windows.json` first.

## How To Get To It (User POV)

```ts
Keyboard (while tiling is enabled)
├── Option+<1..0 | A..Z>        -> switch; pressed again on the active workspace -> next tile
│                                  (wraps, and is how Fullscreen Layout tiles get focus)
├── Option+` / Option+Shift+`   -> next / previous window on the workspace, floats included
├── Option+Shift+<1..0 | A..Z>  -> move focused window
├── Option+Tab                  -> back-and-forth (last two workspaces focused, any display)
└── Option+Shift+Tab            -> move focused workspace to next display
Trackpad: three-finger swipe left/right -> next/previous workspace (turn macOS's three-finger
          "Swipe between full-screen applications" off or move it to four fingers)
CLI: spaceman ctl switch | back-and-forth | move-workspace | move | move-tab | move-app | pin
     (switch next|prev and move next|prev [--follow] with [labs] workspace_steps = true)
Config: ~/Library/Application Support/Spaceman/config.toml -> [[rules]], [[workspaces]],
        [settings] workspace_order (edits apply when the file changes; new named-workspace shortcuts register on the next app start)
Menu bar: shows the active workspace glyphs; its menu lists each display's visible workspace (checked)
          and occupied ones with their apps' icons and Option shortcut -> choose one to switch
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
  `following_focus_to_a_hidden_workspace_keeps_the_window_that_took_it`,
  `a_follow_cancelled_by_the_activation_it_answers_follows_on_the_next_cycle` and
  `a_follow_that_fails_while_an_activation_cancels_stays_suppressed` (`focus.rs`, notification
  clicks),
  `workspace_back_and_forth_bounces_between_the_last_two_focused_workspaces`,
  `workspace_back_and_forth_on_one_display_switches_like_before`,
  `switching_to_an_unconfigured_named_workspace_is_rejected`.
- Unit (FakeBackend), `src/app/tests/workspace_order.rs`: `settings.workspace_order` orders
  `listed_workspaces` (unlisted workspaces follow in the default order; an entry neither enables a
  letter nor lists an empty workspace), the HUD row and three-finger swipes, and a config reload
  applies and clears it. Unit, `src/config.rs`:
  `workspace_order_accepts_numbers_and_letters_in_the_given_order`,
  `workspace_order_rejects_duplicate_and_unknown_workspaces`, and
  `saving_a_workspace_monitor_keeps_the_workspace_order` (Settings › Workspaces); CLI,
  `src/cli/tests/regressions.rs`: `config_commands_keep_and_show_the_workspace_order`; agent,
  `agent/src/hud/tests.rs`: `visited_empty_workspaces_keep_the_configured_workspace_order` (empty
  workspaces flicked through during one HUD show stay where the order puts them).
- Workspace steps Lab: Unit (FakeBackend), `src/app/tests/workspace_steps.rs`:
  `switch_steps_through_the_occupied_workspaces_of_the_focused_display_and_stops_at_the_ends`,
  `switch_steps_on_the_side_display_that_has_focus`,
  `with_no_window_focused_switch_steps_the_display_switched_to_last_and_move_refuses`,
  `switch_follows_the_workspace_order_and_named_workspaces`,
  `move_sends_the_focused_window_to_the_next_workspace_even_when_it_is_empty`,
  `move_next_with_follow_twice_ends_on_three_with_the_window`,
  `switch_skips_a_hidden_workspace_whose_only_parked_window_closed`, and
  `workspace_steps_do_nothing_while_the_lab_is_off`. Unit, `src/workspace/tests.rs`:
  `workspace_steps_stop_at_either_end_of_the_list`; `src/hotkeys.rs`:
  `workspace_step_bindings_register_only_with_their_lab_on`; `src/shortcuts.rs`:
  `commands_use_the_ctl_vocabulary`; `src/control.rs`: `parses_control_commands`; `src/config.rs`:
  `parses_labs_with_workspace_steps_off_by_default`; agent, `agent/src/panes/shortcuts/catalog.rs`:
  `workspace_steps_have_no_built_in_chord_and_name_their_bindings`. Regression,
  `src/app/tests/reported/step_dragged_pin.rs` (issues 561, 606): `switch next` from a pin
  dragged to another display steps that display, including with its center in that display's
  menu bar strip (issue 606, `switch_next_from_a_pin_centered_in_a_menu_strip_steps_that_display`),
  and from a pin on its home's display steps that one; `move-workspace` from a dragged pin still
  moves the pin's home workspace. Issue 652,
  `switch_next_from_an_unpinned_float_dragged_right_steps_the_right_display`: the same for an
  ordinary float that kept its workspace when dragged.
- Unit (FakeBackend), `src/app/tests/switch_repeat.rs`: a repeat `SwitchWorkspace` walks a
  3-tile workspace A -> B -> C -> A in split and Fullscreen Layout (remembered focus and HUD
  included), warps the pointer onto each next tile under `mouse_follows_focus`, steps on inside
  the focus settle window while Accessibility still reports the old window; with focus on another
  display's workspace the first press only re-focuses and the next one cycles; a float hands focus
  to the remembered tile; a menu-bar/HUD `SelectWorkspace` on the active workspace never cycles;
  a single-tile workspace stays put.
- Unit (FakeBackend), `src/app/tests/cross_display.rs`: moving a window to a workspace on
  another display, on screen (tiled, floating, focus history) or hidden (parked on its own display
  even when a write off the other display's edge is refused, shown on the target display later,
  left where it was when it cannot be parked), dragged floats parking off their actual display on
  switch or move (issue 148) and relocating from a third display onto the target display (issue
  152), and `move_app_moves_every_managed_window_of_the_app` (issue 121) and
  `move_app_moves_floating_windows_too` (issue 136). Unit, `src/macos/apply.rs`: a refused size write
  before a move no longer stops the move; a window that moves but cannot resize is constrained,
  not unavailable, so it never rolls back a switch (issue 726, `src/macos/apply/frame.rs`);
  focusing raises the window before bringing its app forward, and either `AXFrontmost` or AppKit
  activation brings the app forward (issue 92).
- Unit (FakeBackend), `src/app/tests/float.rs`:
  `a_first_float_lifts_a_full_screen_tile_to_a_centered_card` (72% × 78% of a 1000 × 800 work
  area), `a_first_float_keeps_a_narrow_tiles_width_and_centers_it`,
  `floating_again_restores_the_frame_it_last_floated_at` (also clamped onto a display that
  shrank), `an_ultrawide_first_float_stays_within_the_card_cap` (1280 × 860) and
  `closing_a_float_forgets_its_remembered_frame`; `toggling_a_tiled_window_floats_it_and_retiles_the_rest`
  now expects the lift, and a pin of a tiled window floats it the same way.
- Unit (FakeBackend), `src/app/tests/focus.rs`:
  `a_workspace_switch_that_cannot_focus_keeps_the_focus_restore_pending`,
  `a_workspace_switch_that_focuses_settles_the_focus_restore`,
  `reselecting_the_active_split_workspace_keeps_the_focused_tile` (issue 120),
  `leaving_with_a_float_focused_keeps_the_remembered_tile` (`float.rs`, issue 138),
  `visible_back_and_forth_returns_to_the_float_it_left` and
  `visible_workspace_shortcuts_return_to_the_float_they_left` (`float.rs`, issue 527), and the `mouse_follows_focus_*`
  tests covering `off` / `display` / `window`, empty-display centering, same-display and
  in-window skips, `move-workspace` / `move`, passive `cycle()` focus-follow suppression, and
  `mouse_down` guarding (plus `warp_reassociates_immediately_after_warping_and_skips_while_mouse_is_down`
  in `src/macos/apply.rs`). Whether keys really land in the incoming app on macOS 14 and later,
  and live `CGWarpMouseCursorPosition` behavior on hardware displays, are not covered by a test.
- Unit (FakeBackend), `src/app/tests/move_workspace.rs`:
  `move_workspace_to_occupied_display_promotes_source_previous_and_parks_displaced`,
  `option_tab_after_moving_the_focused_workspace_never_reveals_the_displaced_one` (issue 76),
  `workspace_back_and_forth_switches_a_hidden_workspace_on_another_display`,
  `cycle_records_a_click_on_another_display_for_option_tab`,
  `move_workspace_to_empty_display_and_three_display_wrap_around`,
  `move_workspace_single_display_is_a_noop`,
  `move_workspace_preserves_fullscreen_layout_and_focused_tile`,
  `move_workspace_leaves_a_replacement_tile_minimized_while_away_alone` (issue 682),
  `move_workspace_persisted_assignment_survives_config_reload`,
  `move_workspace_replaces_unplugged_assignment_and_rolls_back_on_error`;
  `src/config/write.rs`: `saving_a_workspace_monitor_keeps_title_contains_rules` (issue 574);
  `src/app/tests/empty_workspace_move.rs` (an empty workspace on a non-main display, issue 201);
  `src/app/tests/empty_workspace_choice.rs` (`[labs] empty_workspace_focus`: an empty workspace
  chosen on another display is the focused one, issue 360);
  `src/app/tests/dual_selector_move.rs` (a display with a UUID and a `secondary` selector, issue 202);
  `src/config/write.rs` tests (the saved assignment keeps config.toml's comments, inline or `[[workspaces]]`, issue 577).
- Unit (FakeBackend), `src/app/tests/reported/`, one file per report: two selectors naming one
  display across a restart (`restart_dual_selector.rs`, issue 386), a reconnect
  (`reconnect_dual_selector.rs`, issue 404) and a config edit (`config_same_display_alias.rs`,
  issue 423, and moving the edited workspace away revealing the one shown just before it, issue
  544; editing a hidden workspace's selector keeps the shown one in front, issue 542); moving a
  workspace back after switches between those selectors
  (`move_back_reveals_last_shown.rs`, issue 400); restored cross-selector history
  (`restored_cross_selector_history.rs`, issue 401); a tall float's round trip
  (`tall_float_round_trip.rs`, issue 403); floats returning on reconnect
  (`reconnect_carries_floats.rs`, issue 405) and a hidden workspace's parked float following a
  reassignment in config (`reassign_hidden_float.rs`, issue 543) and a shown workspace's float
  doing the same (`reassign_visible_float.rs`, issue 665); a reload that reveals a hidden
  workspace on its new display lays its parked tiles out there for good, even while their restore
  is unconfirmed, with no later recovery-position write (the test advances past the park-write
  throttle before reloading; `reload_reveals_parked_tile.rs`, issue 701); HUD rows after moving an empty
  workspace (`hud_rows_after_move.rs`, plus
  `moving_an_empty_workspace_away_retires_its_slot_on_the_old_display` in `agent/src/hud/tests.rs`,
  issue 419); a deleted named workspace (`config_removed_named.rs`, issue 424); Option+Tab after
  lagging focus reports (`back_and_forth_stale_report.rs`,
  `back_and_forth_fast_shortcuts.rs`, issue 399); Option+Tab from a focused pin
  (`back_and_forth_pinned_focus.rs`, issue 526), the return press refocusing that pin, even one
  dragged onto another display (issue 676), including when the target's app hands focus to
  its window on another shown display, even one that is itself pinned there
  (`back_and_forth_visible_sibling.rs`, issues 591 and 657); a first-window route staying used after its
  window is floated by hand, and still routing later windows with `[labs] route_all_windows`
  (`route_after_manual_float.rs`, issue 551); a numbered move under a lagging report
  (`numbered_move_stale_report.rs`, issue 397); no follow after closing all of the frontmost
  app's windows (`close_all_frontmost.rs`,
  `frontmost_app_whose_windows_all_closed_does_not_pull_a_switch`, issue 516); and pointer
  follow for `move-tab --follow` (issue 417), an accepted frame (issue 418) and a held right button (issue 422). Unit,
  `src/workspace/display_map.rs` and `src/app/focus/tests.rs`: the display records and the
  focus automaton on their own.
- Unit (FakeBackend), `src/app/tests/reconnect_unsettled.rs`: a tile whose display disconnects
  before its layout settles returns to its tile on reconnect, and a refusing app stays frozen (issue 203).
- Unit (FakeBackend), `src/app/tests/tear_regressions.rs` (sequences found by model-based testing):
  `switch_after_moving_a_window_there_survives_async_activation`,
  `frontmost_app_with_a_visible_floating_window_does_not_switch_away`,
  `reconnecting_a_display_keeps_parked_windows_hidden_and_owned`,
  `a_failed_re_park_after_a_display_change_is_retried`,
  `a_display_change_leaves_a_held_up_parked_window_alone`,
  `a_display_that_grows_at_the_bottom_re_parks_its_held_up_windows`,
  `a_failed_re_park_survives_a_second_display_change`,
  `re_parking_a_grown_window_hides_all_of_it`,
  `switching_away_parks_a_window_that_overflows_the_display_edge`,
  `laying_out_a_parked_window_retires_its_stale_recovery_frame`.
- Unit, `agent/src/panes/workspaces.rs`: the display picker with two displays (Main | Secondary
  kept, section captions name the real displays, a saved Secondary or Main UUID says no default
  note), three displays (two identically named externals told apart as `Studio Display 1` and
  `Studio Display 2`, picking the third writes its UUID, a saved UUID renders as its label, a
  saved UUID for Main resolves to Main, and a workspace saved by the Secondary display's UUID
  shows no `Default:` note while the third display names it as the default it differs from —
  issue 337), one display (Secondary still offered), an unplugged display (an extra `Not
  connected` segment, its `config.toml` entry kept), and four displays (numbered segments
  `Display 2`/`Display 3`/`Display 4`, names kept in the headings), a name too long for a segment
  (`a_long_display_name_stays_in_its_heading_and_only_its_segment_is_numbered`,
  `two_long_names_that_macos_suffixed_keep_their_names_and_an_unnamed_display_its_position` —
  issue 349), and workspace 10's title naming the keys `[shortcuts]` gives it
  (`workspace_10_names_the_keys_that_really_switch_to_it` — issue 350). Section cards and segments
  themselves are seen in the Playground
  (`Workspaces/workspaces-three-displays`, `Workspaces/workspaces-long-name`,
  `Workspaces/workspaces-custom-modifier`, `Workspaces/workspaces-four-displays`,
  `Workspaces/workspaces-unplugged`), not by a test.
- Unit, `src/macos/displays.rs`: `selector_for` (`main`/`secondary`/UUID) and `ordered` (left to
  right), shared with `Option+Shift+Tab`.
- Unit (FakeBackend), `src/app/tests/switching.rs`:
  `status_menu_lists_each_displays_visible_and_occupied_workspaces` (menu rows per display, checks,
  app order). The AppKit rows themselves (icons, tooltip of app names) are not covered.
- Unit: `swipes_step_through_the_occupied_workspaces_of_the_display_under_the_pointer`
  (`src/app/tests/switching.rs`), the recognizer in `src/swipe.rs` (fires once per swipe; ignores
  short, vertical and five-finger swipes; reports three or four fingers), its touch-report filter
  in `src/swipe.rs` (issue 781: `empty_and_partial_reports_mid_swipe_keep_its_origin`,
  `fresh_fingers_after_a_missed_lift_start_a_new_swipe`,
  `the_same_fingers_resuming_after_a_pause_switch_once_until_they_lift`,
  `fewer_contacts_are_believed_once_they_persist`,
  `persistent_empty_reports_count_as_the_missed_lift`) and
  `an_empty_report_mid_slide_refreshes_the_space_hold_without_lifting` (`src/macos/swipes.rs`),
  pointer-button / three-finger-drag suppression in
  `src/macos/swipes.rs` and `src/app/input.rs` (issue 151), the Lab-gated off switch in
  `trackpad_swipe_off_stops_swipes_only_with_its_lab_on_and_reloads_live`
  (`src/app/tests/workspace_order.rs`, issue 333), the four-finger Lab in
  `four_finger_swipe_lab_moves_the_swipe_to_four_fingers_and_reloads_live` (same file, issue 464;
  the main-thread drop of the other finger count in
  `only_swipes_with_the_configured_finger_count_are_admitted`, `src/app/input.rs`)
  and the Shortcuts row in
  `agent/src/panes/shortcuts/tests.rs`, and visible stranded-window recovery
  after display disconnect in `src/app/tests/display_disconnect.rs` (issue 150), which leaves
  a float restoring from the Dock at its own frame
  (`a_float_restoring_from_the_dock_is_not_moved_as_stranded` in
  `src/app/tests/float_regressions.rs`, issue 710). Live CGEventTap
  callback delivery in `src/macos/swipes.rs` is not covered.
- Unit (FakeBackend), `src/app/tests/reconciliation.rs`: adoption order
  (`bundle_route_applies_to_initial_window_while_secondary_windows_stay_on_active_workspace`,
  `title_route_rule_routes_matching_secondary_window`,
  `windows_all_sends_a_later_window_to_the_route_once_its_lab_is_on` (in
  `src/app/tests/rules.rs`, issue 472),
  `columns_route_into_a_visible_workspace_joins_its_focused_tile_split` and
  `columns_route_joins_a_nested_focused_split_and_keeps_the_other_column` (in
  `src/app/tests/rules.rs`, issue 494),
  `dwindle_route_splits_the_clicked_tile_not_the_one_remembered_from_the_last_visit` (in
  `src/app/tests/rules.rs`, issue 603),
  `ignore_rule_releases_the_window_and_never_adopts_it`,
  `floating_rule_owns_window_without_adding_it_to_layout`,
  `new_window_opens_on_active_workspace_even_when_app_has_window_on_inactive_workspace`), display fallback
  (`missing_preferred_display_falls_back_without_double_visibility`,
  `active_preferred_workspace_falls_back_and_returns_after_hotplug`).
- Unit (FakeBackend), `src/app/tests/utility_floats.rs`:
  `stickies_float_at_their_own_size_and_live_on_their_own_workspaces`,
  `a_tile_rule_overrides_the_built_in_stickies_default`,
  `title_contains_routes_matching_windows_without_persisting_titles`,
  `a_pinned_float_stays_visible_across_switches_and_never_pulls_focus` (pause restores nothing
  extra), `activating_an_app_whose_only_window_is_pinned_never_switches_to_its_owner` (frontmost
  app, no focused window reported), `a_pin_rule_added_while_a_note_is_parked_brings_it_back`,
  `a_pin_rule_added_while_a_tile_is_parked_floats_and_brings_it_back` (#678, its app's
  first-window route stays used),
  `a_pin_rule_matches_a_parked_window_whose_app_was_never_observed` and
  `a_never_observed_parked_note_keeps_an_earlier_title_rule_over_a_pin_rule` (#697, all three
  notes start on the display being switched, with ownership checked before parking),
  `a_pin_rule_reload_leaves_a_parked_note_an_earlier_title_rule_floats` (#549), and
  `ctl_pin_floats_and_pins_the_focused_window_then_unpins_where_it_is_seen`. Unit,
  `src/config.rs`: `built_in_float_defaults_apply_only_after_user_rules`,
  `pin_rule_parses_and_rejects_a_workspace`, `title_contains_matches_case_insensitively_in_memory`;
  `src/workspace/tests.rs`: `hand_pins_persist_for_floating_windows_and_rule_pins_do_not`,
  `pinned_floats_do_not_place_their_app_on_a_workspace`;
  `src/app/tests/float_regressions.rs`: `a_float_left_on_another_desktop_keeps_floating` (#768);
  `agent/src/map/tests.rs`: `pin_and_title_contains_rules_round_trip_as_non_app_wide_floats`.
- Unit, Settings › Rules and the HUD menu: `agent/src/main.rs`
  `settings_opens_on_general_not_on_rules`; `agent/src/map/tests.rs`
  `the_add_menu_lists_apps_that_already_have_rules`,
  `a_second_rule_for_an_app_takes_a_title_filter_that_starts_blank`,
  `built_in_floats_are_default_chips_and_take_a_tile_or_pin_override`;
  `agent/src/rules/filters_tests.rs` (a filtered rule is valid TOML ahead of the every-window
  rule, with comments and other tables kept); `agent/src/hud/tests.rs`
  `app_context_menu_names_route_rules_always_open_and_keeps_pin_for_the_pin_action` and
  `app_context_menu_respects_the_built_in_float_defaults`. No test drives the sheet's field or
  the menus through the pointer.
- Unit (FakeBackend), `src/app/tests/cross_display.rs`:
  `a_recalled_window_keeps_its_placement_across_a_refused_first_park` (a recalled tile and a
  recalled float whose first park is refused join their remembered hidden workspace once the
  window accepts the move, issue 689).
- Unit (FakeBackend), `src/app/tests/restart_recall.rs`:
  `every_window_reopened_at_a_parking_spot_is_rescued_and_recalled` (three Safari windows on
  workspaces 1-3, reopened minutes later in the background),
  `a_window_parked_by_spaceman_is_never_rescued`,
  `a_window_at_a_parking_spot_that_cannot_move_backs_off_and_is_rescued_later` (failed moves are
