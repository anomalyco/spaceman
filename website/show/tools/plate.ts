// Assembles the opening plate from hero.py's frames: public/show/plate/opening-hevc.mp4, opening.webm and
// opening.mp4 (BT.709, faststart, a keyframe every half second so seeks land fast) and opening.json (per frame: the panel's
// corners and the screen's glow). Run where ffmpeg is: bun show/tools/plate.ts <frames dir> [crf]
import { readdirSync, mkdirSync } from "node:fs"
import { join } from "node:path"
import { TAPE } from "../../src/show/tape.js"

const dir = process.argv[2]
const crf = process.argv[3] ?? "16"
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

// The frames are 16-bit; each encode keeps as many of those bits as its browsers can decode, because
// the room is mostly dark gradients and 8 bits band there: HEVC Main 10 (Safari, Chrome on a Mac),
// VP9 profile 2 (other Chromes), and 8-bit H.264, error-diffused, for everything else.
const color = ["-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv"]
const input = ["-framerate", "30", "-i", join(dir, "f%04d.png")]
const run = (name: string, args: string[]) => {
  const p = Bun.spawnSync(["ffmpeg", "-y", "-loglevel", "error", ...input, ...args, ...color, "-an", join(out, name)], { stdout: "inherit", stderr: "inherit" })
  if (p.exitCode) throw new Error("ffmpeg failed: " + name)
}
const yuv = (fmt: string, dither = "none") => ["-vf", `scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int:sws_dither=${dither},format=${fmt}`]
run("opening-hevc.mp4", [...yuv("yuv420p10le"), "-c:v", "libx265", "-preset", "slow", "-crf", String(+crf + 2), "-tag:v", "hvc1",
  "-x265-params", "log-level=error:keyint=15:min-keyint=15:bframes=2:aq-mode=3:no-sao=1:colorprim=bt709:transfer=bt709:colormatrix=bt709:range=limited",
  "-movflags", "+faststart"])
run("opening.webm", [...yuv("yuv420p10le"), "-c:v", "libvpx-vp9", "-profile:v", "2", "-crf", String(+crf + 8), "-b:v", "0", "-row-mt", "1", "-g", "15",
  "-deadline", "good", "-cpu-used", "2"])
run("opening.mp4", [...yuv("yuv420p", "ed"), "-c:v", "libx264", "-preset", "slow", "-tune", "film", "-crf", crf, "-g", "15", "-bf", "2",
  "-x264-params", "aq-mode=3", "-movflags", "+faststart"])

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
const mb = (f: string) => (Bun.file(join(out, f)).size / 1e6).toFixed(2) + " MB"
console.log(`plate: ${n} frames ${w}x${h} -> opening-hevc.mp4 ${mb("opening-hevc.mp4")}, opening.webm ${mb("opening.webm")}, opening.mp4 ${mb("opening.mp4")}`)
