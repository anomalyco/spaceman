# Rounds

Each round: capture on a HiveMind browser worker (`show/tools/run-capture.sh`), review the clip,
contact sheets and strips, get a cross-family critique (`show/tools/critic.ts`, GPT-6.x, since the
show and engine were written by Claude), list the 10 weakest moments, fix, rebuild, republish.

Captures step the clock frame by frame, so the frame interval in `report.json` measures the
capture, not the show. The number that matters is `cpu`: main-thread time per frame for the
director, world, texture painting and WebGPU encoding.

## Round 0 → first watchable cut (r0b)

WebGPU ran end to end once the primitive stride (`FLOATS = 20`), the texture `used` flag and the
Canvas 2D start path were fixed. 103 s. The pull-back drew a giant arc at about 92 s (the head plane
faded in too early).

## Round 1 (critic: gpt-6.1-sol, `critique-r1-sol.md`, frames from r0b)

Ten weakest, in our order after reading the critique against the frames:

1. 0.0–0.7: ⌥1 looked as if it tiled the windows. → A separate Spaceman-start beat (status item
   and bar appear), then the tile at 0.7.
2. 13–22: moving and switching were one blur. → One whole Postbox example (⌥⇧2, then ⌥2, then ⌥1)
   before Browsy; bar scaled up to 2.2× for the section.
3. 30–35: the rule appeared fully formed. → Camera push onto Settings, field-by-field clicks
   (App, Action, Workspace).
4. 36–40: Dana's routing wasn't proved. → ⌥2 after the route, so we see her there.
5. 56–65: the hedge photos were thumbnails from the first frame. → First three mornings large,
   then the archive accelerates; hedge section cut by about 2.6 s.
6. 40–46: the drill had no close-up. → Camera push onto the note.
7. "Dinner (4)" → "apologies received" title was a written joke. → The calendar just says "Dinner (1)".
8. Browsy's "How to organize" page winked. → "Gutter guards: which kind?"
9. Outer ws6 "Mow" window was a gag. → The workspace the outer bar moves to is empty.
10. 92–101: the last look was too short. → Held the outer desktop longer; "No. 14" on the
    Corrections note ("Corrections from No. 14").

Also: the Dana/Ignore insert was removed (Ignore isn't a way to silence a person; it confused the
rules). The head-plane fade now waits for the camera to pass it, which fixed the arc.

Declined: cutting "Everyone gets the same amount of space." It was a sincere line and passed the
Strangelove test. Kept pending a second opinion (round 2 cut it).

## Round 2 (critic: gpt-6.2-astra, `critique-r2-astra.md`, frames from r1)

Ten weakest (the critic's ranking held up against the frames):

1. 56–64: the hedge was a green polygon copied 14 times. → Painted photographic plates (three
   weathers) with the same fence joint, pole and sidewalk seam; each morning is a few pixels off
   register, in its own light, with the house's shadow lengthening through the year.
2. 13–21: the keycaps sat under the captions and send/follow read as one command. → Key-cast moved
   to the bottom left, clear of the bar and captions.
3. 74–78 (phone): call tiles stacked 3 + 1. → `tile()` puts four windows in a 2 × 2 grid in both
   orientations.
4. 93–95: the round head looked like the Call avatar. → A seated man: chair back, dropped left
   shoulder, neck, head turned slightly, an ear, rim light from the monitor.
5. 100.2–101: an empty wallpaper hold looked like the player unloading. → Key at 100.55.
6. 78–85: the menu clock jumped backwards across the montage. → One forward-running clock, Nov 18
   to Nov 20; historical dates stay inside documents.
7. 40–47: the drill box was already checked. → He clicks it as he says the line.
8. 66–71: "circulated again" wasn't visible. → The sheet shows "Sent to 8 households, Jun 10",
   then "Sent again to 8 households, Nov 11".
9. 36–40: bounced back to 1 while "It's on two" was still captioned. → Stay on 2 through the line,
   cut straight to the drill.
10. 85–91: the cursor pointed at the lit window before the voice did. → It parks on the feed.

Also: the still now has one high-contrast card ("A tiling window manager for macOS.", the ⌥1
keycaps, "A short film, 1:41" and a Sound on/off choice). Skip/CC/Sound sit top right on phones.
"Everyone gets the same amount of space." cut: a second critic made the same case, and the silent
call is colder.

Declined: a title-specific Ignore row (the story beat it served was already gone).

## Round 3 (critic: gpt-6.1-sol, `critique-r3-sol.md`, frames from r2)

Ten weakest:

1. 40.6–44: camera pushes near the screen edge showed black past it. → `frameOf` clamps the view
   inside the screen.
2. 68–71: the minutes "Sent" footer sat under the caption. → It's a share banner at the top of the
   sheet.
3. 13.9–21.8: the narration landed after the mail demo. → Split into "Mail went to two." (over the
   Postbox move, before ⌥2) and "The internet went to three." (over the Browsy move); longer hold on
   1 before switching (⌥2 at 15.9).
4. 47.9: "The house has a workspace." advertised the metaphor. → "Household things went on four.",
   which continues "Mail went to two."
5. 51.4: "Nobody wanted to take the minutes. So I take the minutes." said it twice. → "Nobody else
   wanted to take the minutes." Also "the second conversation" → "our second conversation".
6. 72.6–77.6: four still call tiles were ambiguous. → Two tiles, him and No. 16. No. 16 leaves at
   75.3 and his own tile takes the screen (tiling, shown once more).
7. 84–88: No. 14's window was lit from the first doorbell shot, so the light line had nothing to
   point at. → Dark until 87.3; it comes on as he watches, 0.7 s before the line.
8. 35.2–36.1: Dana sat on the wrong workspace for 0.9 s. → Routed after 0.3 s.
9. 40–46: "borrowed spring 2022" had an empty checkbox. → Secondary text, no box.
10. 77–86: the blue grade announced "now sinister". → About 40 % less.

Also: hedge shutters only on the first three photos and two later ones (no machine gun). The
pull-back reaches the whole desktop by 96.4 instead of 97.2 (a 4 s still look); cut at 100.85,
0.3 s after the other keyboard.

Declined: a persistent "⌥2, then ⌥3" instruction (the hint already names the keys and the bar
responds); re-cutting the montage to three selections (it carries the clock forward and shows the
rules list, which is the product beat); a changing acoustic perspective for the car (needs new SFX;
listed as a next improvement).

## Round 4 (critic: gpt-6.2-astra, `critique-r4-astra.md`, frames from r3)

Ten weakest:

1. 96.4–100.55: you could tell whose house was in the photo before whose desktop it was. → "Corrections
   from No. 14" set at 30 px with **Not sent** under it in a draft red; the body stays small.
2. 24–30: the visitor's hint was at the top, the effect at the bottom. → "Your turn. Try ⌥2, then
   ⌥3." sits just above the bar (touch: "Tap a workspace in the bar.").
3. 37.6–40: on 2, the HOA email competed with Dana. → After ⌥2 the camera settles on her message,
   with Postbox's edge still in frame.
4. 8.9, 40.8, 67.0: captions showed the last clause before the voice reached it. → Captions run one
   sentence at a time, timed from the voice's word alignment.
5. 13.9–21.8: keycaps far left, evidence in the bar. → Key-cast sits beside the bar.
6. 40–46: the drill was ticked before the neighbour objected. → He ticks it at 46.3, after "He said I
   didn't need to." The note has an empty box from the start; "borrowed spring 2022" has none.
7. 77.6–84.3: the rules list flashed by. → The camera pushes onto it so the title rule
   (`Viewer, "No. 14 — Hedge" → 5`) reads.
8. 92–95.6: the man nearly disappeared. → Brighter monitor light on his near shoulder.
9. 47.2–50.8: four overstayed. → Cut to 5 at 50.5, just after "four".
10. 63.6–65.4: the precision was easy to miss. → The archive stops growing at 64.4 and holds on
    6′ 2.75″ / 6′ 3″ before the camera pulls out at 65.6.

Also: "Press ⌥1 or click to play" on the still; cut moved to 100.66, while No. 14's windows are
leaving, so no empty wallpaper shows.

Declined: none outright. The phone ending still squeezes the house into a tall tile (the critic's
product note); listed as a next improvement.

## Measured (r4 capture, WebGPU, desktop 1440 × 900 logical)

Main-thread time per frame, busiest section (13–40 s, 263 frames): p50 0.9 ms, p95 4.2 ms, max 16 ms.
Ending (88–101 s): p50 1.2 ms, p95 14.7 ms, max 18.2 ms (window textures repainting as the outer
desktop builds). No errors on WebGPU, Canvas 2D, reduced-motion or phone paths. The visitor's
⌥2/⌥3/⌥1 during 24–30 switched 1 → 2 → 3 → 1.
