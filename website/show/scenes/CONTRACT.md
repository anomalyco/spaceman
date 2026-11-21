# 3D scenes: the contract between Blender and the film

Every physical thing in the film (the study, the monitor, the room, the street, the basement) is modelled by
our own Python scripts in this folder and either rendered live in the browser by the film's own WebGPU
renderer (`src/show/gpu.js`, glTF with baked light, loaded by `src/show/glb.js`) or rendered offline by
Cycles as an image. No downloaded models or textures.

**The one exception:** the photos in his photo library (`public/show/photos/`) are phone photos made with an
image model. They appear only inside the Photos window on his screen, never as the film's world, and must
pass as unedited iPhone photos (ordinary framing, a little off level, phone noise, the same hedge and street
from shot to shot). The list of them, with prompts, is in `notes/photos.md`.

Blender 5.2 is on the workers, not on Studio. Build through HiveMind from the worktree root:

    hivemind run --kind render --dir . -- sh -c 'B=/Applications/Blender.app/Contents/MacOS/Blender; \
      mkdir -p captures/X; $B -b --factory-startup -P website/show/scenes/<set>/build.py -- [quick] [previews] out=$PWD/captures/X'

Then copy what you keep from `captures/X` into `public/show/3d/` and `public/show/renders/`. Units are metres;
glTF is exported Y-up. Keep `.blend` files out of git.

## The story's world

Pinecrest, an ordinary street of white clapboard houses. **No. 12** (his) and **No. 14** face each other across
the street. His study is upstairs at the front of No. 12; from its window you see No. 14's clipped hedge
(~1.9 m, over the 6 ft limit), a streetlight, and No. 14's upstairs window lit warm with a lamp. The basement of
No. 12 is "six".

## study.glb + study.json (`study/build.py`, `street.py`)

- `Screen`: the monitor's panel, one quad, UVs 0..1 over the visible panel, 16:10, material `Screen`: the film
  draws the desktop there and lights the room with it.
- `Glass` materials are skipped by the renderer. Emissive materials without a base texture are drawn as
  emission × `emitStrength[name]`.
- Everything else is baked: base colour = albedo × all light but the screen's; emissive = albedo × the
  screen's light alone, so the film computes `base + glow × screen colour`. `study.json` carries the
  per-object exposure scales and the screen's corners.
- Cameras (by name in study.json): `CamScreen` (the panel fills a 16:10 frame), `CamChair`, `CamDesk`,
  `CamRoom` (near the door: the empty chair, desk, monitor, blinds), `CamWindow` (out at No. 14's lit window).

## basement.glb + basement.json (`basement/build.py`)

Concrete block, joists, a bare bulb (`Bulb`, emissive), stairs down the left wall, the walls sheeted in
plastic. Three places side by side toward the far wall, labelled on the floor SIXTEEN, MARK, FOURTEEN; the first
two old patches of newer concrete. Two states, separate objects, each baked with the other (and both, for
`Room` and `Detail`) hidden:

- `StateBefore`: the third place under builder's plastic, waiting, the label maker beside it.
- `StateAfter`: him on it, wrapped in plastic and tape, blood pooled and tracked.

Cameras: `Before` (from the stairs, the three places) and `After` (low and close on the third). The film shows
one state at a time.

## Offline renders (`public/show/renders/`, `renders/build.py` and the sets' previews)

| File | What |
| --- | --- |
| `intro.jpg` | `CamChair`, the screen asleep: the page's still. |
| `chair.jpg`, `room.jpg`, `window.jpg` | The study cameras, for Canvas 2D and before the room loads. |
| `before.jpg`, `after.jpg` | The basement cameras, for Canvas 2D. |
| `across.jpg` | The photograph No. 14 sends: his study window at night, from across the street. |
| `door.jpg` | His doorbell camera at night; `stills.json` gives No. 14's lit window (`DOOR_WIN` in content.js). |
| `wall.jpg`, `wall-s.jpg` | His wallpaper: dunes at dusk. |
