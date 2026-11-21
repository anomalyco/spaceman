# 3D scenes: the contract between Blender and the film

Every physical thing in the film is modelled by us in Blender (built by Python scripts in this folder) and either
rendered live in the browser with three.js's WebGPU renderer (glTF with baked light) or rendered offline by Cycles
as an image file. **No image-model output anywhere.** No downloaded models or textures either: geometry and
materials are procedural or made by our scripts.

Blender 5.2 is on the workers (pro, mini), not on Studio. Build and render through HiveMind, e.g. from the
worktree root:

    hivemind run --kind render --dir . --fetch website/public/show/3d -- \
      /Applications/Blender.app/Contents/MacOS/Blender -b --factory-startup -P website/show/scenes/study/build.py -- --bake --export

Units are metres. glTF is exported Y-up (the exporter's default conversion). Keep `.blend` files out of git
(`/private/tmp` or the worker's scratch); commit the scripts and the exported assets.

## The story's world

Pinecrest, an ordinary suburban street at night. **No. 12** (his house) and **No. 14** (the neighbour's) face each
other across the street, ~22 m apart. His study is upstairs at the front of No. 12. From his study window you see
No. 14: a two-storey house with a tall, dense, clipped hedge (~1.9 m, slightly over the 6 ft limit) along its
front, a streetlight, and No. 14's upstairs window lit warm. No. 12 has a basement ("six is downstairs").

## study.glb (public/show/3d/study.glb)

- Objects (each a mesh with its own material; join static props into a few meshes per texture atlas):
  - `Screen`: the monitor's visible panel, one quad, UVs exactly 0..1 over the visible panel (u to the right,
    v up), physical aspect **16:10**. Material named `Screen` (no textures; the film draws its desktop there).
  - `Glass`: the study window's pane(s), material `Glass` (the film draws it as a faint reflective pane).
  - `No14Window`: No. 14's lit upstairs window pane, emissive warm; the film can dim it.
  - Everything else: baked. Each material's **baseColorTexture** is the bake of everything except the screen's
    light (albedo × all other light, diffuse only, denoised), and its **emissiveTexture** is the bake of the
    monitor's light alone (albedo × light from the screen, as if the screen showed a light warm-grey desktop,
    linear colour ~(0.93, 0.91, 0.87)). The film computes `base + screenGlow × screenColour`, so the room
    can be lit by whatever the screen shows, and is dark when the screen is asleep. Emission strength 1.
  - Use a second UV map only for baking if needed; the exported baked textures must use `TEXCOORD_0`.
- Cameras, exported as glTF cameras with these names (perspective, vertical FOV in the glTF):
  - `CamScreen`: looking straight at the Screen so the panel exactly fills a 16:10 frame.
  - `CamChair`: from a seated eye height behind the desk chair, the monitor and desk in front, room dark.
  - `CamRoom`: pulled back near the door: the empty chair, the desk, the monitor, the window with blinds.
  - `CamWindow`: at the window, looking out between the blinds at No. 14's lit window and hedge.
- Budget: ≤ 8 MB total, atlases 2048–4096 JPEG (or WebP), triangles ≤ 300k.

## basement.glb (public/show/3d/basement.glb)

The room on his floor plan ("Downstairs", 1:50): 5.6 m × ~4.4 m, stairs coming down along the left wall from
the top-left corner, a boiler bottom-left, a shelf on the right wall, a door bottom-right. Three places side by
side along the far wall, each 0.8 m × 2.0 m, gaps 0.35 m, the first 1.55 m from the left wall, 0.45 m from the far
wall. Places 1 and 2 are old patches of newer concrete in the floor (saw-cut seams, slightly different tone);
place 3 is fresh: dark, still-wet concrete, a trowel, a bucket, a bag of cement, folded builder's plastic. One
bare bulb. Baked like the study (no screen light: the emissiveTexture can be omitted).
- Two states, as separate objects, each baked with the other hidden: `StateBefore` (place 3 not yet filled:
  builder's plastic sheeting laid flat over it and lapping onto the floor, the edges weighted with a few bricks, a
  bucket and a bag of cement beside it) and `StateAfter` (place 3 freshly filled: dark, still-wet concrete, a trowel
  laid exactly parallel to it, the bucket rinsed, the plastic folded into a neat square). Nothing gruesome is
  shown: the horror is in how tidy it is. The film shows one state at a time.

- Cameras: `CamStairs` (from halfway down the stairs, the three places in view), `CamFloor` (low and close,
  the third place in the foreground, the other two beyond).

## Offline renders (public/show/plates/, Cycles, JPEG q≈80, denoised)

| File | What |
| --- | --- |
| `intro.jpg` 2560×1600 | From `CamChair`, the screen asleep (black glass with faint reflections), the room in streetlight through the blinds. The page's first image, so it must be beautiful and light (≤ 350 KB). Also `intro-s.jpg` 1280×800 (≤ 90 KB). |
| `study.jpg` 2200×1375 | From `CamRoom`, the screen dark, for the no-WebGPU fallback. Report the screen's four corners in pixels in `stills.json`. |
| `window.jpg` 2200×1375 | From `CamWindow`, fallback. |
| `basement.jpg` 2200×1375 | From `CamStairs`, fallback. Also `basement-close.jpg` from `CamFloor`. |
| `photo.jpg` 1800×1200 | The photograph No. 14 took: from across the street at night, his lit study window upstairs at No. 12 (the monitor glow visible inside). |
| `hedge-0..3.jpg` 1200×800 | No. 14's hedge from the same spot every morning (his driveway, eye height): hedge across the middle, No. 14's roof above it, a red-and-white survey pole, a mailbox, lawn, sidewalk. 0 clear summer morning, 1 overcast after rain (wet pavement), 2 autumn low sun with fallen leaves, 3 misty dawn. Report the hedge top's and a 6 ft (1.83 m) line's normalised y in `stills.json`. |
| `door.jpg` 1280×720 | His doorbell camera at 11:47 PM: wide-angle from No. 12's porch toward the street and No. 14's hedge, night. |
| `wall.jpg` 2560×1600 + `wall-s.jpg` 960×600 | His desktop wallpaper: a quiet landscape (dunes or hills at dusk), our own procedural terrain and sky. |
