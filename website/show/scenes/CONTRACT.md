# 3D scenes: the contract between Blender and the film

The film never leaves his computer. The only 3D set is his study and its monitor, modelled by our own Python
scripts in this folder and either rendered live in the browser by the film's own WebGPU renderer
(`src/show/gpu.js`, glTF with baked light, loaded by `src/show/glb.js`) or rendered offline by Cycles as an
image. No downloaded models. Textures may be CC0 only (Poly Haven), listed with their licences in
`scenes/textures/LICENCES.md`; never image-model textures.

**Everything outside the study is a picture inside a window on his screen**, never the film's world:

- the photos in his photo library (`public/show/photos/`): phone photos made with an image model, which must pass
  as unedited iPhone photos. The list, with prompts, is in `notes/photos.md`.
- the security-camera feeds in Vigil (`public/show/feeds/`): stills made with an image model, run through the
  camera sensor in `src/show/feeds.js`. The list is in `notes/feeds.md`.

The street and the basement are no longer sets: `basement/build.py` and `street.py` stay in the repo only as
history and as reference renders for the image model.

Blender 5.2 is on the workers, not on Studio. Build through HiveMind from the worktree root:

    hivemind run --kind render --dir . -- sh -c 'B=/Applications/Blender.app/Contents/MacOS/Blender; \
      mkdir -p captures/X; $B -b --factory-startup -P website/show/scenes/<set>/build.py -- [quick] [previews] out=$PWD/captures/X'

Then copy what you keep from `captures/X` into `public/show/3d/` and `public/show/renders/`. Units are metres;
glTF is exported Y-up. Keep `.blend` files out of git.

## The story's world

Pinecrest, an ordinary street of white clapboard houses. **No. 12** (his) and **No. 14** face each other across
the street. His study is upstairs at the front of No. 12; from its window you see No. 14's clipped hedge
(~1.9 m, over the 6 ft limit), a streetlight, and No. 14's upstairs window lit warm with a lamp. "Six" is the
cellar of No. 12; it is only ever seen through its camera.

## study.glb + study.json (`study/build.py`, `street.py`)

- `Screen`: the monitor's panel, one quad, UVs 0..1 over the visible panel, 16:10, material `Screen`: the film
  draws the desktop there and lights the room with it.
- `Glass` materials are skipped by the renderer. Emissive materials without a base texture are drawn as
  emission × `emitStrength[name]`.
- Everything else is baked: base colour = albedo × all light but the screen's; emissive = albedo × the
  screen's light alone, so the film computes `base + glow × screen colour`. `study.json` carries the
  per-object exposure scales and the screen's corners.
- Cameras (by name in study.json): `CamScreen` (the panel fills a 16:10 frame), `CamChair`, `CamDesk`,
  `CamRoom` (near the door: the empty chair, desk, monitor, blinds).

## Offline renders (`public/show/renders/`, `renders/build.py` and the sets' previews)

| File | What |
| --- | --- |
| `intro.jpg` | `CamChair`, the screen asleep: the page's still. |
| `chair.jpg`, `room.jpg` | The study cameras, for Canvas 2D and before the room loads. |
| `across.jpg` | The photograph No. 14 sends: his study window at night, from across the street. |
| `wall.jpg`, `wall-s.jpg` | His wallpaper: dunes at dusk. |
