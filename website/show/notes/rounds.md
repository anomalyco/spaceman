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


# Second pass (Nov 21)

Kit's notes: name the product, "the sweet flesh of efficiency", three tapes (A/B/C) that differ in how far
the ending goes, a diegetic start, and an ending that's brutal and unmistakable. The film is cut to C.

## Round 2 · rebuild (10:00–10:30)

- Tape C (`c2`), 150.5 s: a half-second breath between paragraphs where the take ran them together,
  cuts placed in the quietest 30 ms between words (the alignment runs up to 0.35 s late on the last
  words), "Minutes" and "Present... one" lifted.
- `director.js` reads every time from the tape: `word()` takes the next match within 30 s of the last
  lookup (else the nearest before it), so repeated phrases ("I closed two", "Side by side") resolve
  without hard-coded times. HOLD and END come from the tape's long gaps and last word.
- The ending: three signed notes "About the cameras", two closed with the opening's gesture; ⌥6
  "Downstairs" (a floor plan with two hatched places, 16 · Mar 3 and Mark · Jun 11, a dashed third,
  14, and a ticked list); the doorbell; the study pull; the kitchen; "I've put the plastic down" cuts
  to the floor; black through the silence; "Present..." cuts to the floor afterwards; a hard cut to
  the card.
- The still is the exhibit (recorder with a typed tag); clicking the recorder or ⌥1 starts it. No
  skip, no sound toggle, captions toggle kept.
- Workspaces filled in one move (`openMany`): the five retargets the trace check flagged at the house,
  street and signatures are gone; one remains (the bar marker's deliberate return to 1).


# Third pass: the 3D film (Nov 21)

## Round 4 · photos library and the window shot (11:40–12:55)

- His Photos library replaces the hedge viewer: 14 phone photos (image model, see `photos.md`), 214 tiles
  captioned by date and time, the first morning measured, fill to 160 mornings, two decaying flicks up to
  August, Aug 14 opening out of its tile.
- Basement rebaked with Room/Detail baked with both states hidden (the stray shadow in After is gone).
- CamWindow: No. 14's front on its own 2048 atlas at 3× samples, the streetlight raking its clapboard, a night
  sky plane, the camera just under his blinds so the last slats cross the top of the frame.
- Capture r4: trace check clean (5,227 frames, no issues), no errors on any path; CPU p95 1.4 ms at the ending.
- Critic (gpt-6.1-sol, `critic-r4.md`): the body reads as a Halloween prop; No. 14 looks like a game set (old
  bake); the plastic reads opaque and the two old places empty; Canvas 2D shows a white monitor; phone portrait
  shows too much ceiling; the 3D screen is muddy; the plan reveal over-explains; sound unverified. Its ask for
  play/mute/skip controls is rejected (Kit: no controls).

## Round 5 · the pull, the screen, the ending, the body (12:45–)

- Tape: 2.8 s of silence after "He was the last thing." (was 0.5) and 2.4 s after "It never does." (was 1.6);
  tape 190.6 s, film 177.3 s. The aligner runs a word's end through a following silence, so the pull is timed
  from the gap itself, not from the word.
- The pull out of the screen: 4.5 s (was 1.4 s in practice: it started when the word ended, inside the
  silence). The camera keeps looking at the point it looked at on the screen and only turns to the room in
  the second half, and focus stays on the panel until the last 40%; the mirror of both on the way in.
- The panel is a display, not a lit surface: it skips the tone curve (its alpha flags it), so the desktop in
  the room is as bright and clean as the flat desktop it cuts from (it was grey through AgX).
- Ending: after "It never does." the light comes back on the same floor from the same place, afterwards, and
  the camera creeps down to the low close framing through "He's on six now. With the others." and a 1.1 s hold
  (was 0.55 s) before the hard cut.
- The plan reveal is one move: the plan framed at "Mark was in June", held, then the dashed 14 at "the last
  thing" (the zoom, the 16/Mark selection and the re-frame are gone).
- Basement: him, wound in the sheet (ellipsoids remeshed into one form, creased, pleated, pulled in at six
  tape bands, the overlap loose on the floor); blood as pools with fractal edges and runs, shaded by depth and
  laid over the floor sheet's creases; the floor sheet creased from its packet folds; the two old places as
  newer, lighter concrete with a fine seam.
- Canvas 2D: the desktop is drawn onto the stills' blank monitors. Phone portrait opens the field of view less
  (exponent 0.42, was 0.6).
- 13:05–13:20: blood on the wrap is now soaked through the plastic (soft-edged, dull, darkest where it pressed),
  not painted on; the floor drag marks darker and browner. The basement's floor sheet decimated (Room mesh
  110k → 33k vertices, basement.glb 11.4 → 6.8 MB).
- The mix, measured (nobody has listened): the room tone sat at −17 LUFS, level with his voice, so every
  silence was a roar; now −34 LUFS (RMS −33 dBFS), 17 dB under him. The detective was as loud as him (the
  compressor lifted him back up); he now goes in after the compressor, band-limited to 200–4000 Hz, at −21 LUFS
  against his −17.
- Captions: the aligner had "From the start?" 1.8 s late and 0.04 s long (it was never on screen); the first two
  replies are retimed from the waveform. A caption now ends at the pause after it, not when the aligner says.
- Workspace switch: the old workspace fades in 55 ms, so the two never sit on top of each other as a dissolve.
- Street (study rebake): fascia, gutters and downpipes on the eaves, a plinth, a lighter slate roof, and a blind
  part way down in every dark window (each at its own height).

## Round 6 (critic: gpt-6.2-astra, `critic-r5.md`, frames from r5) · 13:40–

r5 capture: no errors on any path (WebGPU, Canvas 2D, reduced motion, phone); busy CPU p95 3.6 ms, ending
1.4 ms. Read against the frames, the critic's points we took:

1. The occupied floor still read as a floor plan with a dummy on it (157, 173). → The last camera comes down to
   a low line along the three places (40 mm from 0.8 m, was 50 mm from 1.6 m), so SIXTEEN, MARK and him share
   one sightline; the floor sheet's two free edges curl; the wrap gets broken highlights where it faces the bulb
   and loses its pink wash (blood through it goes straight from milky plastic to dark).
2. The photo No. 14 sent read as a stage spot on pristine siding (116). → Re-shot from the pavement opposite, a
   little to one side, the streetlight's spill broad and soft; then made a phone photo: held a little crooked,
   grain in the shadows, colour noise, the phone's sharpening and JPEG (`show/tools/phonify.py`).
3. The window shot's roof drew black (inward faces) and the hedge was a cuboid (153). → Roof normals
   recalculated; the hedge's top and face wave by hand.
4. The six section stood still for eleven seconds (131–146, the cursor where it was). → His hand moves over the
   plan with the words: onto Mark's place, idling toward the empty one, then onto 14 for the click.
5. Three identical camera notes looked like test data (122). → One petition, three households: the same two
   lines, signed "Ruth and Alan, No. 16", "Mark (10)" with "Also still waiting on my drill.", and "The Abeles,
   No. 14" with "Thank you."

Not taken: a play button, mute and skip (#7; Kit asked for none); cutting "I tasted the sweet flesh of
efficiency" (#9; Kit wants it); shortening the six section by 8 s (the tape fixes its length).
