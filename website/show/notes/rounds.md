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

Then, from the probes (p6a, p6b) and the r5 frames at 100%:

6. The reply photo (116) re-shot again: the first phone version was a CG window frame on even grey siding.
   Now it's the whole upper storey from the pavement: night sky, the eave, one lit window, his chair and the
   monitor's glow behind the blinds; the street light dim and broad, the monitor the brightest thing in it.
7. The March screenshot (31) showed grey bars for text. It is now his March desktop painted by the same
   window painters as tonight (bylaws, drills, notes, the camera note, the sticky), piled on the wallpaper.
8. Browsy's 38 tabs drew as hatching. The open tab keeps its width and title; the rest crush beside it.
9. Settings, typing the workspace (71): the empty field showed "—" with the caret after it, like a typed dash.
   It now loses the dash and gets a focus ring.
10. Canvas 2D: the grade drew a dark box around the cursor (138). Cut-out textures skip the grade box.
11. Study: the window shot steps back from the blinds (the live depth of field can't melt a rail 25 cm from the
    lens); No. 14's roof faces outward (it drew black live); the hedge is remeshed so its lumps have vertices
    (it stayed a slab under the noise); the binders vary in width, depth and age, labels stuck on by hand.
12. Basement: the bins vary (depths, a short pair, one lid off and leaning), labels at different heights.
13. The bar close-up (59–61), frame by frame: the key cast sat beside the bar, so the close-up cropped it to
    "⌥ 2" at the frame's edge. During the close-up it now sits on top of the bar; at home it stays beside it.
    The switches themselves (47, 59) were checked frame by frame and kept: the old workspace is gone in two
    frames and the new one slides 32 px in over 0.22 s, one blended frame between, no dissolve.
14. The body (b8 → b9): the wrap still read as a white mannequin with ribs. The plastic is darker and greyer, the
    sheen weaker, the creases run on a slant (wound round him, not ribs), the form smoothed further, and the
    blood at the edge of the stain is darker so it stops reading pink.
15. Phones (probe p6d, 390×844): the bar close-up cropped both ends of the bar; its framing now opens until the
    whole bar fits the width. The low last shot, widened for a tall screen, put the wall in the middle and cut
    him off at the right; the basement now has a second camera for an upright phone, from past his feet along
    him to the pool and FOURTEEN, and a set's `<name>-tall` camera is used whenever the screen is taller than wide.
16. The study's screen light spread 160° → 180°, so the pool on the desk has no edge (r5 critic #5, the "hot
    stripe across the keyboard").
17. Every camera transition, frame by frame at 1/30 s (probe p6g, 14:20): the push into the screen ending in the flat
    desktop (7.3–7.5), the pull out (147.0–147.3), the cut to the window (152.1–152.2), to the basement (155.6),
    from black to the body (170.7) and to the card (176.7–176.9). All are clean cuts or continuous moves, with one
    exception: in Canvas 2D the push into the still's monitor ended 12% darker than the flat desktop it hands to
    (whites 216 → 245 in one frame), because the panel's "a screen in a room" dimming stayed on to the end. It now
    fades out over the last 45% of the push. WebGPU had a 3% step on whites only (240 → 247: bloom added in
    linear light in 3D and in display light flat). The panel's bloom is now added in display light like the flat
    path; probe p6h measures whites 248 / 248 / 247 across 7.3 / 7.37 / 7.4, and Canvas 2D 245 on both sides.
18. b10 (14:17): the body is now grey plastic wound with tape, creases on a slant, no ribs; the blood is dark.
    The phone's own camera for the last shot (After-tall) ends looking along him from past his feet to the pool
    and the FOURTEEN tag, the head half covering the tag.
19. The study re-baked (s14 → s17, 14:41): the binders differ in height and wear and the lower row fades; the
    hedge across the street has a broken top edge (voxel remesh, no subdivision blobs) and No. 14's roof faces
    out, so it catches the streetlight instead of reading as a black cut-out; the screen's light spreads 180°,
    so the pool on the desk has no edge. CamWindow went 7 cm too high in s16 (five slats over the roof); s17
    sits at 1.15 m, two soft slats across the top, the ridge clear.
20. Critic r6 (gpt-6.2-astra, 15:10, `critic-r6.md`). Its #3, "Mail is rubber" at 126.5: a window that grew or
    shrank stretched its old pixels until the re-raster landed, so text squashed vertically, and the closed note
    faded over 0.32 s under the growing neighbours. Now a resizing window shows its content at the size it was laid
    out, cropped or revealed from the top left with the window's background filling the rest (the way an app
    re-lays itself), and a closed window is gone in 0.1 s. Checked frame by frame at 126.2–126.7 (p6k), and at
    22.9, 43.4–45.6, 68.6, 88.3 (p6j): no squashed text anywhere.
21. Its #2, the archive's clock cycled 6:02 / 6:03 / 6:04. Morning times now come from a hash: 6:01–6:16, a
    couple of minutes earlier in summer and later after November starts (p6k at 108: 6:02, 6:09, 6:03, 6:06…).
    Not taken: #1 (another body pass; the close is the joke's landing and b10 is fresh), #4/#6 (the plates
    were re-baked an hour ago), #10 (the three notes are the same note on purpose; the line says so).
22. Its #5, the keys in the bar close-up (59.95) were as big as the bar's cells on a phone. Above the bar they are
    now 70% size, 10 px over it, so the bar stays the subject (p6l, desktop and phone).
23. Motion trace and perf (r6t, 15:16): the full film at 30 fps in 5321 frames, `trace-check` no issues. A new
    `capture.ts --trace` steps the film without screenshots, because the full screenshot pass no longer fits
    HiveMind's 30-minute browser cap (r6 was killed at frame 3,600 of 5,303). Main-thread time per frame at
    1440×900 @2x: busiest section p50 1.0 / p95 3.2 / max 11.2 ms (r5: 1.0 / 3.6 / 9.0); ending p50 0.9 / p95
    1.4 / max 3.4 ms. No errors on WebGPU, Canvas 2D, reduced motion or phone.
24. Its #8, the two-inch violation was harder to read than the caption. A bracket now joins the measured line and
    the dashed limit at their right end, labelled "+2″", pulled in on narrow crops so it fits a phone (p6m, p6n:
    WebGPU, Canvas 2D, phone at 98.5).
25. b11 (15:27; mini's Blender aborts at launch with an NSException, so the render is sent to pro). Its #9 and
    "other": the trail read as a row of brush dabs (its shade and width ran on sines). It is one continuous drag
    now, width and density from noise, thinning toward the sheet. The third sheet's far corner is turned back
    where the tape gave up, and one knelt-on crease runs across it toward the bulb (subtle at film size).
    Probe p6z (157, 170.77, 176.6 and the before/after times): no errors on any path.
26. Final build (15:30): the review build stamp is gone from the footer and `three` / `@types/three` are out of
    `package.json` (the renderer is our own `glb.js` + `gpu.js`). `scripts/check --no-web` passed. r7q (keyframes on
    every path, the page) and r7t (motion trace, perf): no errors on WebGPU, Canvas 2D, reduced motion or phone;
    `trace-check` 5,321 frames, no issues; main-thread time per frame at 1440×900 @2x: busiest section p50 1.0 /
    p95 3.5 / max 8.6 ms, ending p50 0.9 / p95 1.3 / max 2.7 ms. Before the press the page loads the HTML, the JS
    and CSS, `mark.svg` and the still (`intro.jpg`); everything else (tape, effects, photos, stills, both GLBs)
    loads after it, the basement in the background behind the study.


# Fourth pass: the study in Cycles (Nov 21–22)

Kit: "none of it looks high quality enough". The opening becomes a path-traced plate: the asleep still, the
wake-up and the move in are rendered in Cycles (`show/scenes/study/scene.py`, `hero.py`, Blender 5.2 on pro)
and played as video, with the live desktop set on the panel's quad in every frame.

## Round h · lighting (h1–h9)

1. The room was rebuilt for Cycles: CC0 Poly Haven textures (oak floor, plaster, oak veneer, wool, fleece;
   `textures/LICENCES.md`), modelled blinds with ladders and a pull, a radiator, a sash window, a socket.
2. One streetlight down the road to the south-east is the key, through blinds tipped 18°, so its bands land on the
   desk wall and across the print; the landing light through the open door behind the camera is the warm fill; the
   Mac mini's LED is the only light on the desk. No. 14 is gone from the window: past the glass is night.
3. Camera `door` (from the doorway, 42° vfov, f/2.8) picked over four others; the move in replicates
   `camera3()` onto the screen, and its end frame puts the panel's corners where the flat desktop's will be.

## Round h-1 (critic: gpt-6.1-sol, `critique-h1-sol.md`, frame `h-round1/still-t0.jpg`)

4. Its #1, a daylight-crisp window: the streetlight got a 11 cm source (soft penumbra), the landing fill came up.
   #2, a slab chair: the cushions are subdivided and bowed, the back thicker. #3, striped desk edge: the edge
   banding uses the veneer turned along it. #4–#7: pens at angles, the trackpad off square, a folded letter, the
   print larger with heavier contours, binders in three colours with hand-written labels.

## Round h-2 (critic: gpt-6.2-astra, `critique-h2-astra.md`, h10 at t=0 and t=4)

5. Its #3, cast-metal aluminium: a finer, quieter grain at roughness 0.33–0.37. #4, pens as black rods: a tapered
   tip with a nib, an end cap, a clip on one. #5, the leaning book a solid block: boards, a cloth spine, a page
   block inset between them. #2, the window bright top to sill: the spot narrowed to 11° and feathered, the slats a
   shade greyer. #1, a black cut-out monitor: the panel is dark glass under a full, slightly rough coat (0.17–0.30
   by noise), so the window's light lies on it as a faint sheen.
6. The plate: frames 0–221 (film 0 → 7.38 s) render from `hero.py frames=`; a held frame (camera still, glow
   unchanged either side) is copied rather than rendered. The panel shows a blurred desktop in the plate, so its
   light falls on the room and nothing sharp can ghost through the live overlay as it fades up.
   `show/tools/plate.ts` encodes H.264 (faststart, BT.709, keyframe every 0.5 s) and VP9, and writes
   `opening.json` (per frame: corners, glow).

## Round f-1 (critic: gpt-6.1-sol, `critique-f1-sol.md`, feed frames 89.2–176.5 s at 1x)

7. Its #3, the clocks disagree at the end: the menu bar still said Fri 11:56 PM over a cellar camera stamped
   Sat 12:04 AM. The menu bar now moves on to Sat Nov 21 12:04 AM when six opens.
8. Its #2, generic web type: at 2x (`capture.ts --x2`) the desktop is SF Pro with clean edges; the softness was
   the 1x JPEG capture. Not changed.
9. Not taken: #1, grain, vignette and the grade over the UI (this is a film of a screen, not a screen recording;
   the renderer brief asks for them); #4–#7 (window construction, app fidelity, the streetlight cone, the cellar
   shrouds) wait for the tape Kit picks, since tape H changes the feeds.
10. Plate encodes, measured on frames 0–109 of the 2400×1500 16-bit plate, decoded to 16-bit and levelled ×2–×5
    in the dark wall and floor (`band-wall.png`, `band-floor.png` on the report page): no banding in any of the
    three. HEVC Main 10 at CRF 18 keeps the plaster grain (SSIM 0.992); VP9 at CRF 24 smoothed it, so VP9 moved
    to CRF 19 (0.993); dithered H.264 at CRF 16 is noisier (0.942) but unbanded.

## Round k-1 (critics: gpt-6.1-sol and gpt-6.2-astra, `critique-k1-sol.md`, `critique-k1-astra.md`; 36 stills from probe k3 against tape K's lines, prompt `critic-k1.md`)

Both asked the same question beat by beat: what does the viewer understand that he didn't say?
Both read the spine (camera on three, dossiers, the album, the bins, seven, the cellar) and the ending as
unmistakable. Their fixes overlapped almost entirely.

11. Both, the order for fourteen cameras unsampled under "one or two more": it was there (39.1–41.3) but
    came straight off the saved still. The silence after "I found it pleasing." went from 4.5 to 6 s, so the
    walk, the zoom, the save and the order each get their time and the order sits under "one or two more".
    Tape K is now 105.8 s (1:46).
12. Both, the woman with the dog lost in the zoom: she was a dark figure against a dark car and fence. She
    now wears a mid-grey long coat (lighter than the car, darker than the lit wall) with dark hair to the
    collar, and reads as a woman with a dog at 1×, 2.2× and 3.6× (probes k4, k5).
13. Both, the album out of order (Jun–Nov 2026, before the February bins): "Hedge" now runs from the day
    after the cameras went up, Fri Dec 12 2025, to Mon Feb 16 2026, two days before the bins. Mornings to
    Jan 11, then every hour to Jan 24, then every night until dawn. Each picture's weekday comes from its
    date, and the date moves on after midnight. The menu bar reads Mon Feb 16 11:58 PM; the month label
    shows the year. The first flick of the scroll lands on December.
14. Both, "Joan → 7" not readable before the cat: the entry lands as he says "there" and the frame pushes in
    on the first two weeks of March. "Mark → 7" lands with June's calendar, the frame pushes in on it, and it
    holds into the silence before "Where is Alan now?". A 1.8 s silence after "her cat ever since." keeps the
    group chat off Mark's line.
15. Both, the plastic order forecasts three bodies: the duct tape and "Qty 3" are gone. It is clear 6 mil
    sheeting, ordered alone.
16. Both, clocks that disagree: at 2:13 AM the feeds now move with the menu bar (they had said Nov 26,
    11:52 PM). The drafts to Joan show tonight in the header ("Mon 1:40 AM") with her old messages dated
    "Wed, Nov 26, 2025" above them. The dossier flicks were checked in sync (probe k6: every feed on each
    workspace reads the menu bar's date).
17. Sol, the letter under Chatter's menu: Postbox comes forward as the letter opens. Astra, the inbox's
    4,812 messages matched the album: the inbox now holds 1,206.
18. Not taken: cutting "It also gives you numbered workspaces." and shortening the tiling line. Those are
    Kit's calls; they're flagged to him with what the picture already shows. Not taken: "Only the one
    window." (Sol: expendable). The cut to the cellar lands on it, and the line is the mild version of what
    we see. Not taken: Sol's note that daylight shows through the blinds at the start. In the plate it's
    the streetlight, and the critics saw one frame.
19. Not taken: "Then Mark." for "I moved Mark in June." (Astra). The date is the only date he gives, and
    the calendar repeats it on purpose.

## Draft 17 (Kit's notes on draft 16), Nov 22

Tape K unchanged (take k7, 105.8 s). Commits `a3eb9eb` (controls, opening, clips, sound, framing) and
`60d76d6` (the macOS look pass).

1. Playback controls: a quiet bar at the bottom with play/pause, restart, scrub, captions and mute. It shows
   on hover or when paused and hides after an idle mouse. Space plays and pauses, ← and → seek 5 s, M mutes.
   A seek rebuilds the world from the start, replayed quietly to the new time, then restarts the tape and the
   beds there. "Watch again" no longer skips the opening plate.
2. Framing: the camera keeps at least 94% of the desktop in view (it went to 34% before). The focused tile
   gets an accent edge and a soft blue glow, and the others dim by 17%. Key lines were enlarged to read at
   full screen: the order is 22 pt in a card, the album reads "Hedge 4,812 Photos" at 17/15 pt, the lid
   sticky is 26 pt, and "Joan → 7" and "Mark → 7" are 15.5 pt bold.
3. Opening: a heap of ten windows. He hunts through four of them, then ⌘W closes six, 0.17 s apart. Spaceman
   tiles the four that are left, then he opens two more (4 → 5 → 6).
4. Motion: the cursor moves on a quintic Hermite arc (no dead stop on arrival) and presses for 85 ms. The bar
   pops in from 0.84 scale with a slight overshoot. Album flicks get a short push before they coast.
5. CCTV: real generated clips for the door camera, the cat and Alan (notes/feeds.md).
6. Sound: no thunks, closes or shutters; closing a window is silent. Six dry scissor-key taps (75 ms, about
   −20 dB under the voice) rotate with small gain and pitch jitter, plus one Force Touch click at −27 dB.
   Room tone sits 52 dB under the voice. The last two switches (to 6 at "still", then 7) are silent. The
   keys were cut from ElevenLabs sound-effect takes by `show/voice/keys.py` (high-passed at 350 Hz, low-passed
   at 11 kHz, levelled by their loudest 30 ms), and the spectrograms were checked for tails.
7. macOS look: Sequoia chrome (lights #ff5f57/#febc2e/#28c840, 12 pt with 8 pt gaps, grey when inactive;
   neutral title bars; sidebars in their own material up under the lights).
   - Safari: the address centred in the title bar, a tab bar, Favourites on a new tab, and order pages
     with the shop's header and a tick.
   - Messages: a conversation list when wide, bubbles with tails (blue for him), the iMessage timestamp, and
     a compose field.
   - Mail: SF-style mailbox symbols, accent selection, and a header block with initials, subject and To.
   - Notes: a yellow selection, list rows with a date and preview, circular checklists, the date centred.
   - Calendar: Day/Week/Month control, today in red, tinted events with a solid edge.
8. Spaceman itself, matched to the agent's source:
   - Settings is `view.rs`/`map.rs`: five panes under the lights, the version at the foot, a raised
     panel, and Rules as "Where apps open", a tile per workspace, plus "Float, never tiled". The rule is
     now made the way the app makes it: Add on workspace 2's tile, then Chatter from "Running apps".
   - The bar wears the HUD capsule from `hud/pill.rs` (#161618, a #303036 hairline) and says what the HUD
     says: "Workspace 3 · 2 windows", "Moved to Workspace 2 · 3 windows". Changes roll in the way of travel.

## Draft 18 (script draft 18, tape L), Nov 22

1. Tape L, take l5 of eight (Sarah as the detective): word-perfect, unhurried, and a clean "great many".
   113.97 s, −18.0 LUFS. The recorder's play clunk and reel whir (ElevenLabs SFX, `music.ts`) are mixed
   into the tape at 0.0 by `score.recorder()`, so seek, pause and export carry them.
2. The score (`score.py`): three ElevenLabs Music stems (cello at 33 s, dread at 55 s, peak placed so the
   cut lands 21.3 s into it, on its crest), premixed into one `score.mp3` that starts at 30.0 s. Its gain is
   ducked from the tape's words and inserts, there's a hush on "Where is Alan now?", and the held last second
   swells +3 dB. It starts on the audio context's clock (`delay`), and the export mixes it as kind `score`.
3. Weaves: the to-do list with "Return Mark's drill", ticked on Mark's workspace before "Mark → 7";
   Messages → 2 in Rules; the door camera as a pinned float (`pin: true`, a "Floating" badge, raised); a Rules
   glimpse ending "Cellar → 7".
4. Apple's app names and icons (`public/show/icons/`, from `/System/Applications`); Vigil stays invented.
   The licensing risk is flagged to Kit on the report.
5. Photos (`library.js`): the real grid, 2 px gaps, sticky headers, blur-to-sharp hydration, 18 new photos
   (`notes/photos.md`), no repeats nearby, dusk by 75%, one night row per section, Alan last.
6. The end: `TAPE.marks.end` is the last word plus 1.0 s (END 111.567), then 0.65 s of black and silence,
   then the bright page.
7. The study (`scene.py`, `clues.py`): the desk is stained walnut, and there's an architect's lamp (matte
   black and brass) with a warm pool on a yellow legal pad of timings (22:31 WED 5 …). The night past the
   blinds is a deeper blue. Every desk object is squared to the edge: keyboard, trackpad, three parallel
   pens, a steel rule, the mug's handle. Mark's drill stands in the far corner with a sticky note ("MARK /
   No. 10"). A roll of polythene sheeting and two tins of cat food sit on the bookcase's lower shelves.
   Index cards 4, 5, 6 and a blank 7 are pinned to a corkboard left of the screen. The handwriting is Caveat
   (OFL). Bug found on the way: the lamp's brass rim was a capped cylinder, a lid over the mouth that blocked
   the spot. It's open now, a ring.

## Draft 19 (Kit's notes on draft 18), Nov 22

1. The tape's start "sounds terrible": `recorder()` looped a 2.6 s motor take for 6 s at +4 dB, before the
   voice compressor, so the opening silence pumped it up. Now it's one dry play-key press (key-3 of six
   new ElevenLabs takes; the least bright, a single latch, −50 dB within 0.13 s), low-passed at 6 kHz and
   faded out by 1.0 s. It's mixed after the compressor and after loudnorm, at a fixed peak
   (`score.KEY_DB`). Measured in the built tape: the key peaks at −16.8 dBFS, and 0.5–5.5 s sits at
   −41.7 dB RMS. The room itself is ~5 dB lower (vent, hiss and hum amplitudes).
2. Talking over the dog walker: the pause after "I found it pleasing." is 3 s, not 6. "So I bought one or
   two more" now plays over ⌘S and typing "dog walker 2", and the order lands as he names her. The push
   back from the order waits for it (`backT`). Everything after moves 3 s earlier (END 108.567).
3. The score, re-arced on word anchors (`score.stems(A, end)`):
   - a lone cello at the door camera;
   - dread-1 (lifted at its start) for the neighbours and the hedge;
   - urgent-0 (new: a pulsing cello ostinato with climbing violins) peaking on "I moved Mark in June";
   - hush-0 (new, −9 dB) under the three questions;
   - and on the switch to seven, climax-0's stab followed by climax-2 and peak-1, both cresting on the cut.
   The arc itself isn't smoothed, so the switch is a hit (silent → −22 dB in 0.2 s). The duck is smoothed.
   Under his voice in the climax, 1–3 kHz is carved out (a band-pass subtraction, −8 dB at 1.8 kHz), and a
   tanh soft limit takes the crest. Per second, under his voice: cello −33, hedge −35, bins −26, Mark −27,
   hush −39 to −45, the switch −26, the last line −20, the held second −13.
4. The end: 0.75 s of black and dead silence, then a dark card (#0a0a0c, a faint radial light), 220 pieces
   of confetti from two corner cannons (`src/confetti.ts`: closed-form drag, gravity, sway and flip, a pure
   function of time since the burst; skipped under reduced motion), and a children's "Yaaay!" (yay-0,
   -15 LUFS, kind `card`). The export mixes card audio after the cut and steps the confetti per frame
   (`window.__confetti`).
5. Messages: the eleven dashed "Draft" bubbles are gone. Joan's one grey bubble ("Just putting mine out
   Wednesdays now, hope that's ok!"), then a timestamp, then his four sent blue bubbles (run-aware tails,
   wrapping, a small pop as each one sends) and "Delivered". The thread scrolls to its newest message, as
   Messages opens it.
6. Safari's Favourites: vector touch icons on glassy tiles (`favicons.js`): a pine crest, a civic front, a sun
   and cloud, a map with a road and a pin, a newspaper, a gold-shielded bank. Tab favicons and initials
   avatars stay: that's what Safari and Messages draw at those sizes.
7. Photos: the blur was the thumbnails falling back to their 9 px copies whenever the scroll was fast, at
   about a viewport per frame. Now there are three unhurried throws down the first day, then ⌘↓ to the newest.
   Each tile is a grey placeholder until its row first comes into view, then loads in a wave (soft
   0.07 s, sharp 0.2 s, left to right with scatter). It's computed from the scroll path and the film time
   (`scrollOn`, `arrival`), so seeking and the export are exact.
8. Windows: they open from 94% with a small settle (0.34 s, bounce 0.3) and fade in ahead of their size.
   They close to 95% and fade in 0.11 s while the rest reflow after 0.06 s. Tiles settle in 0.38 s,
   critically damped, keeping their velocity. A float rises 3 pt with a 1.5% swell and a wider shadow (the
   `lf` spring).
9. A click on the picture pauses and plays, except a tap on the workspace bar during "your turn".

## Draft 20 (the rename: Pane, at anoma.ly/pane), Nov 22

1. Tape M is tape L with one sentence re-taken: "Then I installed a small program called Pane." Eight
   takes (q1-q8, `textPane` in `mono.json`) read the passage in context, from "From the start?" to
   "nothing on top of anything else.", in L's voice and direction. q5 is spliced into l5 by
   `process.py`'s new `splice` step, cut in the pauses either side of the sentence, with 12 ms fades,
   −0.6 dB. Scribe hears "…called Pane. It tiles your windows.", its pitch is 85 Hz (as in l5), and the
   pauses either side keep their length and their room level (−41 dB). The tape is 0.09 s longer (111.06 s).
2. Kit's beat: on that sentence he clicks Safari's address field in the window in front, types
   `anoma.ly/pane` (a quick hand, the rhythm at 0.55×), and the page loads (a blue bar along the field):
   the end card's dark page in miniature (`panepage.js`), with the full address shown. He clicks Download
   for macOS as he says "Pane"; the button presses in and Safari's download button drops into the
   toolbar and fills. Then ⌘W sweeps the clutter, Pane's item comes into the menu bar on "It tiles", and
   the four tile. The workspace bar still only comes up on a switch, so it doesn't appear here.
3. Everything the film shows says Pane: the app's menu name, its Settings (Version 0.3.0), its icon file
   (`icons/pane.png`). The card: "Pane" and "For your windows."; the site's Download is
   `Pane-v0.3.0-macos-arm64.zip`; the export's card reads `anoma.ly/pane`.
4. The site lives at https://anoma.ly/pane/: Vite `base: "/pane/"`, `worker.ts` (redirects /pane to
   /pane/, strips the prefix for `ASSETS`, answers Range for media, as Hex's worker does), and
   `wrangler.jsonc` on Anomaly's account. The gallery and capture builds still pass `--base ./`.

## Draft 21 (the neighbours renamed, and the craft pass), Nov 22

1. The neighbours are Alan (four, No. 10), Juliana (five, No. 16) and Dax (six, No. 14), first names
   only. Tape M is a new full take of the finalized script (`textN` in `mono.json`, v7 of eight), so
   the splice step is gone. v7's pauses and pitch (91 Hz) sit closest to L, and Scribe hears every name
   and full stop. The detective's new question, "Where is Dax now?", is cut from the first of her four
   repetitions in `dax-2.mp3`; he answers "He's on seven, with the other two." The tape is 116.23 s.
2. The study: the corkboard holds four cards (4, 5, 6 and a blank 7), small but readable from the door
   camera; the focus holds on the legal pad in the dark and racks to the monitor as it wakes.
3. The workspace bar is Pane's Icons style (`agent/src/hud/icons.rs`): each number with its apps'
   icons, a marker on the current one, and a moved window flies into its icon on the new number.
4. Details: a hand over the download link and an I-beam over the address field (the cursor hides while
   he types); camera white balance per camera and a blinking REC; Rules shows each neighbour's chip;
   the plastic order carries its order number, its delivery (Feb 28, two days before "Juliana → 7")
   and his other tabs (the HOA bylaws, a cat feeding guide); Mail shows the Cc to the HOA committee; the
   street's group has one unread when Messages first opens (3, then 11, later, where the list is out of
   view), and on Juliana's dark house its notification slides in, sits 2.2 s and goes, unclicked. Alan goes on seven on June 11; two nights later the drill is ticked
   off his list. Every date, weekday and clock agrees.

## Draft 22 (back to Spaceman, and the page around the film), Nov 22

1. The name is Spaceman again, everywhere the film shows it: the line is "Then I installed a small
   program called Spaceman." (tape M is `textO`, take w8 of eight: 0.36 s pauses, 91 Hz, and Scribe
   hears it right; the tape is 112.90 s). The app is `spaceman` in `apps.js` with `icons/spaceman.png`;
   Safari goes to `anoma.ly/spaceman` and its page is `sitepage.js`; the card is "Spaceman" and "For your
   windows."; the download is `Spaceman-v0.3.0-macos-arm64.zip`. The site is at https://anoma.ly/spaceman/.
2. The page: over the bottom of the hero still, "For your windows.", one line on what Spaceman is, a
   platform-aware Download (`src/Download.tsx`, `src/platform.ts`; `?platform=` forces a state) and
   Watch the film. Below the film: six features with small demos (tiling, workspaces with the Icons
   bar, rules, native tabs, recovery, privacy), the shortcuts, a download section with the Terminal
   install, and a footer with the agent files. The release lives in `src/release.ts`; the JSON-LD and
   `/skill.md` (the repo's setup skill) come from `vite.config.ts`; the stylesheet is inlined.
3. The og:image is `og/og.html` shot by `show/tools/stills.ts` (the study behind, a crisp frame of the
   tiled desktop, the icon and the name). `show/tools/serve-site.ts` serves `dist/` through `worker.ts`
   for `ship-site-check --as https://anoma.ly/spaceman/`.

## Draft 23 (Kit's polish round, and Dax at the window), Nov 22

1. The header's mark is the app icon (`icon-192.png`, rounded, a hairline ring and a soft shadow) and
   "Spaceman" in the page's own type; the hand-drawn `S P Λ C E M Λ N` wordmark is gone. The footer uses
   the same icon.
2. The binders: lever-arch files in book cloth (Poly Haven's `rough_linen` under charcoal, navy, forest,
   slate and kraft), each with a printed label in a clear pocket (`clues.py`, IBM Plex Sans Condensed on
   off-white paper, numbered, set along the spine) and a nickel-rimmed finger hole. Widths, heights and
   depths vary; each is a hair off square. Two cloth hardbacks stand at the end of the upper shelf, the
   second leaning on the side.
3. The light: the streetlight is a big soft source (0.75 m, a 20° cone, fully feathered, deeper blue,
   about half the power), so the blinds throw a glow on the wall, not a stencil. The lamp is the key (15 W,
   a wider pool) with a small warm spill; a dim cool panel in the window rims the chair and a low warm
   bounce from the door side lets its fabric read.
4. The print is an architect's elevation of the terrace (`clues.py`) in a slim black aluminium frame
   hung flat to the wall. The drill is softer moulded forms: a waisted grip, a rounded body, a ribbed
   chuck, satin yellow over a soft-touch overmould.
5. Dax at the window is Dax Raad, with his blessing (Kit, Nov 22): the last photo of the Hedge album
   (`n-alan.jpg`, `photos.md`) and his CCTV clip on six (`seq-alan.jpg`, `feeds.md`).
6. `hero.py close=binders,wall,drill` renders close-ups of the details for checking.
