// Assembles the opening plate from hero.py's frames: public/show/plate/opening.mp4 (H.264, faststart,
// BT.709, a keyframe every half second so seeks land fast), opening.webm (VP9, for browsers without
// H.264, open-source Chromium among them) and opening.json (per frame: the panel's
// corners and the screen's glow). Run where ffmpeg is: bun show/tools/plate.ts <frames dir> [crf]
import { readdirSync, mkdirSync } from "node:fs"
import { join } from "node:path"
import { TAPE } from "../../src/show/tape.js"

const dir = process.argv[2]
const crf = process.argv[3] ?? "20"
if (!dir) throw new Error("usage: plate.ts <frames dir> [crf]")
const out = join(import.meta.dir, "../../public/show/plate")
mkdirSync(out, { recursive: true })

const meta: Record<string, { glow: number; corners: number[][] }> = {}
for (const f of readdirSync(dir).filter((f) => /^corners-\d+-\d+\.json$/.test(f))) Object.assign(meta, await Bun.file(join(dir, f)).json())
const frames = readdirSync(dir).filter((f) => /^f\d{4}\.png$/.test(f)).sort()
const n = frames.length
for (let i = 0; i < n; i++) if (frames[i] !== `f${String(i).padStart(4, "0")}.png` || !meta[i]) throw new Error(`frame ${i} missing`)

const probe = Bun.spawnSync(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", join(dir, frames[0])])
const [w, h] = probe.stdout.toString().trim().split(",").map(Number)

const enc = Bun.spawnSync(
  ["ffmpeg", "-y", "-loglevel", "error", "-framerate", "30", "-i", join(dir, "f%04d.png"),
   "-vf", "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
   "-c:v", "libx264", "-preset", "slow", "-tune", "film", "-crf", crf, "-g", "15", "-bf", "2",
   "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv",
   "-movflags", "+faststart", "-an", join(out, "opening.mp4")],
  { stdout: "inherit", stderr: "inherit" },
)
if (enc.exitCode) throw new Error("ffmpeg failed")
const vp9 = Bun.spawnSync(
  ["ffmpeg", "-y", "-loglevel", "error", "-framerate", "30", "-i", join(dir, "f%04d.png"),
   "-vf", "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
   "-c:v", "libvpx-vp9", "-crf", String(+crf + 12), "-b:v", "0", "-row-mt", "1", "-g", "15", "-deadline", "good", "-cpu-used", "2",
   "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv",
   "-an", join(out, "opening.webm")],
  { stdout: "inherit", stderr: "inherit" },
)
if (vp9.exitCode) throw new Error("ffmpeg (vp9) failed")

const r = (v: number) => Math.round(v * 1e5) / 1e5
await Bun.write(
  join(out, "opening.json"),
  JSON.stringify({
    fps: 30,
    frames: n,
    size: [w, h],
    end: TAPE.marks.head + 0.7, // director.js: the move's end, the hand-off to the flat desktop
    land: 4.5, // from here the frame eases onto where the flat desktop's panel will be
    glow: Array.from({ length: n }, (_, i) => meta[i].glow),
    corners: Array.from({ length: n }, (_, i) => meta[i].corners.map(([x, y]) => [r(x), r(y)])),
  }),
)
const size = Bun.file(join(out, "opening.mp4")).size
const size2 = Bun.file(join(out, "opening.webm")).size
console.log(`plate: ${n} frames ${w}x${h} -> opening.mp4 ${(size / 1e6).toFixed(2)} MB, opening.webm ${(size2 / 1e6).toFixed(2)} MB`)
