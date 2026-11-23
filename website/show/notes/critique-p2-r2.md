<!-- gpt-6.2-astra, 2026-11-21T18:29:45.146Z -->

### 1. Ten weakest moments, ranked

| Rank | Time | What reads cheap or false | Code/edit fix |
|---|---|---|---|
| 1 | 1:50.6–1:55.2 | The plan selects the two occupied places. Then “You’re the last thing” lands on **empty asphalt**. The film has drawn the exact image for that sentence and left it two seconds early. | Move the framing from the selected pair to the dashed **14**. Hold through “thing.” Only then cut to the doorbell. |
| 2 | 0:32.6–0:46.5 | Mail leaves, but at 0:44 the two destination wells look empty. At 0:46 the glass marker straddles two wells; the trace records a 78 px reversal. The close-up makes the bar *less* intelligible. | Keep three fixed, numbered wells. On **move**, change icon occupancy while the marker stays on 1. On **switch**, move only the marker and change the desktop. Keep inactive icons legible. |
| 3 | 1:35.3–1:39.4 | The three signatures get the bottom half of the screen; the already-read inbox and Dana get the top half. Two notes reverse by 952 and 475 px on entry. The eye follows the layout bug instead of counting to three. | Compute the final three-note layout before the first entrance. Frame the signatures together; replay the opening pair of close gestures with the same cursor dwell and interval. |
| 4 | 0:54–0:59.5 | At 0:55 the rule still says “C”; then Chatter is visibly floating over workspace 1. It looks like another manual move. | Have `Chatter · Open on · 2` complete on “two.” Keep workspace 1 visible as the purple icon appears in well 2; **then** switch there to read the messages. |
| 5 | 0:13.8–0:16.1 | On “the same note,” the featured document is **Untitled 47**, with different copy. The viewer inspecting the frame is the viewer most likely to disbelieve the line. | Feature the three matching first lines in a stepped stack. Keep Untitled 47 behind them until both closes finish. |
| 6 | 0:18–0:22.4 | “I installed a small program. It’s called Spaceman.” At 0:19.3, 0:20.6 and 0:22.4 the salient image is still the same beige pile. A tiny status change doesn’t establish what caused the tiling. | Show one actual launch action with the named Spaceman icon; let the first tile leave the pile on “tiled.” No installer montage. |
| 7 | 1:18.4–1:24.4 | The count changes from 15 to 92 to 160; roof, hedge silhouette and framing repeat like one JPEG under different tints. It proves the counter works, not that he got up every morning. | Keep his obsessive camera position. Vary wet pavement, overcast sky, leaf growth and shadow direction across a small bank of plates. Make the searched August morning recognizably one of them. |
| 8 | 1:08.4–1:10.8 | House and street are introduced so fast their contents are still chasing obsolete positions. Groceries/minutes reverse about 712 px; laundry/trash about 390. That is not spring overshoot. | Pre-layout each workspace offscreen. Stagger entrances into **final** bounds; switch the workspace as a unit rather than solving again on each child mount. |
| 9 | 1:55.2–1:59.2 | The doorbell fills the image, but the physical monitor reveals a four-pane desktop. The eye has to rediscover the feed just as it should be finding the lit window across the street. | Use the live desktop as the monitor texture. Start inside the doorbell tile’s exact UV bounds; pull back through the bezel on one camera curve. Transfer focus to the window, not the glass reflection. |
| 10 | 0:11–0:31 | The saved screenshot contains grey rules in place of text. It remains a prominent tile after the real windows settle: a wireframe inside an otherwise finished OS. | Render the earlier pile once to an offscreen texture **before** adding the screenshot window. Use that capture at sufficient resolution for the closest push. |

### 2. Product truth

**Tiling is taught. Move versus switch is mostly narrated.** The essential demonstration is that `⌥⇧2` sends **Mail, not me**; `⌥2` subsequently sends **me, not Mail**. Keep the focused title bar, chord and numbered destination readable in the same shot. Reserve the visitor’s turn for switching; suspend the scripted marker animation while the visitor owns it. The rule needs one visible result **without** a move chord.

### 3. Ending and sound

**The killing reads in sequence; the bundle alone could be a rolled rug.** Two prior entries, the remaining signature, the returned chair and “Present… one” do the work. With the revised cut on “Present,” roughly two seconds on the aftermath is enough. Register it to the last empty-floor framing and hold still; a push invites prop inspection. **The single highest-value change is #1:** let him call No. 14 a thing while we see the place already reserved for him.

No audio take is attached, so I can’t judge the voice treatment or mix. One sound check matters: **silence on a running cassette is not digital zero.** Carry the same low kitchen noise and tape transport through black. No sting, illustrative sound effect or change of stereo space at the reveal.

### 4. Finish

The supplied desktop images are JPEG sheets, not isolated 2× frames. I wouldn’t diagnose single-pixel seams or gradient banding from them. Defects visible even here:

- **Canvas2D Settings:** the hard, offset rounded shadow looks like a second panel behind the first. Replace it with a narrow contact shadow and a broader, lower-opacity falloff.
- **WebGPU chrome:** title-bar lettering goes grey-on-grey before body copy does. Dim inactive controls rather than the whole header; rasterize type at final DPR × camera scale. Keep optical defocus off the focused label.
- **Downstairs:** the heavy perimeter survives the blue grade better than the dates and unchecked entry. Reduce wall weight; raise label contrast. Filter the fine hatching independently rather than sharpening the whole plan.
- **Recorder continuity:** the exhibit and kitchen machines have different-looking cassette surrounds and front controls. A handle can fold; a faceplate can’t change. Use one recorder asset under the two lighting setups.

### 5. Phone and opening still

I see landscape, Canvas2D and reduced-motion strips, not a portrait capture. **Don’t center-crop** the three signatures, the monitor-left/window-right handoff or the horizontal bundle. Use per-shot bounds; contain the wide aftermath rather than cutting off both ends. Give the visitor three numbered tap targets above the safe area instead of an Option-key instruction.

On the still, the play instruction sits on the brightest steel reflection; the duration line nearly disappears. Put one high-contrast **“Play Side A · 2:30”** control beside the recorder, retain `⌥1` as a desktop hint, and enlarge the tag in the mobile crop. **“Found running” is the hook; it’s currently the fine print.**
