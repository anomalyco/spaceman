// Runs the director's timeline against the world model with no browser (no drawing), to catch
// cue errors and dump a motion trace quickly. usage: bun sim.ts [out.json] [--tall]
import { World } from "../../src/show/world.js"
import { build, pile, END } from "../../src/show/director.js"
import { POST_DEFAULT } from "../../src/show/gpu.js"

const tall = process.argv.includes("--tall")
const out = process.argv.slice(2).find((a) => !a.startsWith("--"))
const SW = tall ? 600 : 1440
const SH = tall ? 1300 : 900
const w = new World(SW, SH, {})
for (const d of pile(SW, SH)) w.add(d)
w.setFocus("notes")
let t = 0
const S: any = {
  world: w, SW, SH, reduced: false,
  get t() { return t },
  cam: { D: 1, cx: SW / 2, cy: SH / 2 },
  grade: [0, 0, 0], fx: { ...POST_DEFAULT }, scene: {}, look: { focus: null, dof: 0.6 },
  three: { on: false, a: "CamChair", b: "CamChair", u: 0, glow: 1, black: 0, drift: 0.1, after: false },
  shot(a: string, b: string, u: number) { Object.assign(S.three, { on: true, a, b, u }) },
  flat() { S.three.on = false },
  sfx() {}, bed() {}, hint() {}, end() { ended = true },
}
let ended = false
const { cues, spans } = build(S)
let ci = 0
const trace: any[] = []
const h = 1 / 60
let n = 0
while (t < END + 0.1 && !ended) {
  t += h
  while (ci < cues.length && cues[ci].t <= t) {
    try { cues[ci].fn() } catch (e) { console.error("cue at", cues[ci].t.toFixed(2), e); process.exit(1) }
    ci++
  }
  for (const s of spans) if (t >= s.t0 && t - h < s.t1) s.fn(Math.min(1, Math.max(0, (t - s.t0) / (s.t1 - s.t0))), t)
  w.update(h)
  if (++n % 2 === 0) trace.push({ t: +t.toFixed(4), cue: ci, cam: [S.cam.cx, S.cam.cy, S.cam.D], three: S.three.on ? [S.three.a, S.three.b, +S.three.u.toFixed(4)] : null, active: w.active, ...w.sample() })
}
console.log("ok", t.toFixed(2), "cues", ci, "/", cues.length, "frames", trace.length)
if (out) await Bun.write(out, JSON.stringify(trace))
