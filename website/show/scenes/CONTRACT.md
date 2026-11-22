# 3D scenes: the contract between Blender and the film

The film never leaves his computer. The only 3D set is his study and its monitor, modelled by our own Python
scripts in this folder (`study/scene.py`) and rendered offline by Cycles (`study/hero.py`): the page's still
and the opening plate, a video the film plays with the live desktop set on the monitor's panel. No downloaded models. Textures may be CC0 only (Poly Haven), listed with their licences in
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
      mkdir -p captures/X; $B -b --factory-startup -noaudio -P website/show/scenes/study/hero.py -- <mode> cam=door ev=1.0 out=$PWD/captures/X'

Modes: `at=0,4` (stills at film times, `w= h= spp=`), `frames=0-221` (plate frames, with `corners-a-b.json`),
`test`, `rays`. Units are metres. Keep `.blend` files out of git.

## The story's world

Pinecrest, an ordinary street of white clapboard houses. **No. 12** (his) and **No. 14** face each other across
the street. His study is upstairs at the front of No. 12, lit at night by a streetlight down the road through the
blinds; nothing past the glass is modelled. "Six" is the cellar of No. 12; it is only ever seen through its camera.

## The opening plate (`public/show/plate/`, `study/hero.py frames=`, `tools/plate.ts`)

- Film 0 → 7.38 s (frames 0–221 at 30 fps): the asleep room, the screen waking (glow from 0.42 s), the move in
  from `door` onto the panel. `pose(t)` in `hero.py` is the film's move; its last frame frames the panel the way
  the flat desktop does, and `main.js` eases the plate onto the flat panel's rect over 4.5 → 7.38 s.
- The panel shows `desktop-soft.jpg` (a blur) at the frame's glow, so its light falls on the room; the live
  desktop covers it, mapped through the four corners in `opening.json` at the glow in the same file.
- `plate.ts` encodes `opening.mp4` (H.264, faststart, BT.709, keyframe every 0.5 s) and `opening.webm` (VP9).
  Grain and vignette are the film's own post pass, not baked.

## Offline renders (`public/show/renders/`, `renders/build.py` and the sets' previews)

| File | What |
| --- | --- |
| `intro.jpg` | `hero.py at=0 w=2400 h=1500`, the screen asleep: the page's still, the plate's frame 0, and what Canvas 2D pushes into. |
| `across.jpg` | The photograph No. 14 sends: his study window at night, from across the street. |
| `wall.jpg`, `wall-s.jpg` | His wallpaper: dunes at dusk. |
