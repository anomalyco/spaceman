// Checks the film's opening against the plate it is built from: the room must move as smoothly as the
// plate does, from the first frame. Compares consecutive-frame differences (mean absolute, grey,
// 360×225) and, until the landing, which plate frame each film frame matches best.
// usage: bun plate-check.ts <film.mp4> [plate.mp4]   (exits 1 on a flash, a jump or a freeze)
import { join } from "node:path"

const [film, plateArg] = process.argv.slice(2)
if (!film) throw new Error("usage: plate-check.ts <film.mp4> [plate.mp4]")
const plate = plateArg ?? join(import.meta.dir, "../../public/show/plate/opening-hevc.mp4")
const meta = await Bun.file(join(plate, "../opening.json")).json()
const W = 360, H = 225, N = W * H
const n = meta.frames as number
const land = Math.round(meta.land * meta.fps)

function frames(path: string) {
  const p = Bun.spawnSync(["ffmpeg", "-v", "error", "-i", path, "-frames:v", String(n), "-vf", `scale=${W}:${H}:flags=area,format=gray`, "-f", "rawvideo", "-"], { stdout: "pipe", stderr: "inherit" })
  if (p.exitCode) throw new Error("ffmpeg failed: " + path)
  const b = new Uint8Array(p.stdout.buffer)
  return Array.from({ length: Math.floor(b.length / N) }, (_, i) => b.subarray(i * N, (i + 1) * N))
}
// The monitor is deliberately replaced with the live desktop, so it cannot identify a room frame.
// Exclude its bounds (plus the downsampling filter's edge) in both compared frames.
const screen = meta.corners.map((corners: number[][]) => [
  Math.floor(Math.min(...corners.map((c) => c[0])) * W) - 3,
  Math.ceil(Math.max(...corners.map((c) => c[0])) * W) + 3,
  Math.floor(Math.min(...corners.map((c) => c[1])) * H) - 3,
  Math.ceil(Math.max(...corners.map((c) => c[1])) * H) + 3,
])
const diff = (a: Uint8Array, b: Uint8Array, room: number[] = []) => {
  let s = 0, count = 0
  const masks = room.map((i) => screen[i])
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    if (masks.some(([l, r, t, b]) => x >= l && x <= r && y >= t && y <= b)) continue
    const i = y * W + x
    s += Math.abs(a[i] - b[i])
    count++
  }
  return s / count
}
const P = frames(plate), F = frames(film)
if (P.length !== n || F.length !== n) throw new Error(`incomplete opening: expected ${n} frames, plate=${P.length}, film=${F.length}`)
const m = Math.min(P.length, F.length)
const dp = Array.from({ length: m - 1 }, (_, i) => diff(P[i], P[i + 1], i < land ? [i, i + 1] : []))
const df = Array.from({ length: m - 1 }, (_, i) => diff(F[i], F[i + 1], i < land ? [i, i + 1] : []))
// Match geometry by edges: the film applies a vignette and grade to the room. Raw luminance
// would mistake those intentional lighting differences for a different camera position.
const edges = (frame: Uint8Array) => {
  const out = new Uint8Array(N)
  for (let y = 1; y < H - 1; y++) for (let x = 1; x < W - 1; x++) {
    const i = y * W + x
    out[i] = Math.min(255, Math.abs(frame[i + 1] - frame[i - 1]) + Math.abs(frame[i + W] - frame[i - W]))
  }
  return out
}
const PE = P.map(edges), FE = F.map(edges)
// before the landing the film frame is the plate frame (the screen aside): the best match says which
const off: number[] = []
for (let i = 0; i < Math.min(m, land); i++) {
  let best = i, bd = Infinity
  for (let j = Math.max(0, i - 8); j < Math.min(P.length, i + 9); j++) {
    const d = diff(FE[i], PE[j], [i, j])
    if (d < bd) (bd = d), (best = j)
  }
  off.push(best - i)
}
// until the landing the film moves as the plate does; after it the film adds its own move onto the
// flat desktop, so there a step is checked against the median of the steps around it instead
const moving = (i: number) => dp[i] > 0.3
const around = (i: number) => {
  const v = [-3, -2, -1, 1, 2, 3].map((k) => df[i + k]).filter((d) => d !== undefined).sort((a, b) => a - b)
  return v[v.length >> 1]
}
const freezes: number[] = [], jumps: number[] = []
for (let i = 1; i < df.length; i++) {
  const md = around(i)
  const still = i < land ? moving(i) && df[i] < dp[i] * 0.35 : md > 0.3 && df[i] < md * 0.35
  const jump = df[i] > 1 && (i < land ? moving(i) && df[i] > dp[i] * 2.2 : df[i] > md * 2.2)
  if (still) freezes.push(i)
  if (jump) jumps.push(i)
}
const offs = off.map((o, i) => (o !== 0 && moving(Math.max(0, i - 1)) ? i + ":" + o : "")).filter(Boolean)
const flash = df[0] > Math.max(1, dp[0] * 3)
const r = (v: number) => Math.round(v * 100) / 100
console.log(JSON.stringify({
  frames: m,
  first: { film: r(df[0]), plate: r(dp[0]) },
  freezes, jumps, offsets: offs,
  worst: df.map((d, i) => [i, r(d), r(dp[i])]).sort((a, b) => Math.abs((b[1] as number) - (b[2] as number)) - Math.abs((a[1] as number) - (a[2] as number))).slice(0, 6),
}))
process.exit(flash || freezes.length || jumps.length || offs.length ? 1 : 0)
