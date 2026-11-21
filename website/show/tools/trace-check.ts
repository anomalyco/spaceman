// Checks a motion trace (from capture's trace.json or sim.ts) for what reads as a glitch:
//   jumps   a visible thing moves far more in one frame than in the frames either side of it
//   pops    a window appears or disappears while it is visible (no enter or exit)
//   kinks   a moving thing changes velocity abruptly mid-move (a retarget that dropped velocity,
//           a hard stop, a reversal); starts from rest are springs and are allowed
//   reverse a path that turns back on itself by more than a pixel inside one move (overshoot)
// usage: bun trace-check.ts <trace.json> [--json out.json]
type Frame = { t: number; cue?: number; cam: number[]; active: number; cursor: number[]; bar: number[]; wins: Record<string, number[]> }

const [file] = process.argv.slice(2).filter((a) => !a.startsWith("--"))
const jsonOut = process.argv.includes("--json") ? process.argv[process.argv.indexOf("--json") + 1] : null
const tr: Frame[] = await Bun.file(file).json()
const issues: { kind: string; what: string; t: number; detail: string }[] = []
const add = (kind: string, what: string, t: number, detail: string) => issues.push({ kind, what, t, detail })

// A named 1D or 2D signal per frame, null when absent or invisible.
type Sig = (f: Frame) => number[] | null
const signals: [string, Sig, number][] = [
  ["cursor", (f) => f.cursor, 1],
  ["camera", (f) => [f.cam[0], f.cam[1]], 1],
  ["camera zoom", (f) => [Math.log(f.cam[2]) * 900], 1],
  ["bar marker", (f) => ((f.bar[3] ?? 1) > 0.05 ? [f.bar[0], f.bar[1]] : null), 1],
  ["bar scale", (f) => ((f.bar[3] ?? 1) > 0.05 ? [f.bar[2] * 100] : null), 1],
]
const ids = new Set<string>()
for (const f of tr) for (const id in f.wins) ids.add(id)
for (const id of ids) {
  signals.push([id, (f) => { const w = f.wins[id]; return w && w[4] > 0.05 ? [w[0], w[1], w[0] + w[2], w[1] + w[3]] : null }, 0])
}

// whether a cue fired between frame i-1 and i (or within a range): a retarget, not a glitch
const cued = (i: number) => tr[i].cue != null && tr[i].cue !== tr[i - 1]?.cue
const cuedIn = (a: number, b: number) => tr[b].cue != null && tr[b].cue !== tr[Math.max(0, a - 1)].cue
const len = (v: number[]) => Math.hypot(...v)
const sub = (a: number[], b: number[]) => a.map((x, i) => x - b[i])

for (const [name, sig] of signals) {
  const P = tr.map(sig)
  let peak = 0
  const V: (number[] | null)[] = P.map((p, i) => (i && p && P[i - 1] ? sub(p, P[i - 1]!) : null))
  for (let i = 1; i < tr.length; i++) {
    const v = V[i]
    if (!v) continue
    const s = len(v)
    const a = V[i - 1] ? len(V[i - 1]!) : 0
    const b = V[i + 1] ? len(V[i + 1]!) : 0
    // a lone spike: far bigger than both neighbours
    if (s > 24 && s > 3 * Math.max(a, b, 2)) add("jump", name, tr[i].t, `${s.toFixed(1)} px in one frame (neighbours ${a.toFixed(1)}, ${b.toFixed(1)})`)
    // velocity kink mid-move
    const u = V[i - 1]
    peak = s < 0.5 ? 0 : Math.max(peak, s)
    if (u) {
      const su = len(u)
      const dv = len(sub(v, u))
      // speeding up is how a spring starts; slowing hard, stopping dead or turning sharply is not
      const cos = (u[0] * v[0] + (u[1] ?? 0) * (v[1] ?? 0) + (u[2] ?? 0) * (v[2] ?? 0) + (u[3] ?? 0) * (v[3] ?? 0)) / (su * s || 1)
      const why = cued(i) ? " (a cue retargeted it)" : ""
      if (su > 12 && su > 0.6 * peak && s < 0.35 * su) add(cued(i) ? "retarget" : "kink", name, tr[i].t, `speed ${su.toFixed(1)} → ${s.toFixed(1)} px/frame${why}`)
      else if (su > 4 && s > 4 && cos < 0.5) add(cued(i) ? "retarget" : "kink", name, tr[i].t, `turns ${((Math.acos(Math.max(-1, Math.min(1, cos))) * 180) / Math.PI).toFixed(0)}° at ${su.toFixed(1)} px/frame${why}`)
    }
  }
  // reversals inside one move: per axis, within a run of frames that are all moving
  // a hand moves in curves, so the cursor is only checked for kinks
  const dims = name === "cursor" ? 0 : (P.find((p) => p)?.length ?? 0)
  for (let d = 0; d < dims; d++) {
    let dir = 0
    let peak = 0
    let since = 0
    for (let i = 1; i < tr.length; i++) {
      const v = V[i]
      if (!v || Math.abs(v[d]) < 0.05) {
        dir = 0
        continue
      }
      const sg = Math.sign(v[d])
      if (!dir) {
        dir = sg
        peak = P[i]![d]
        since = i
      } else if (sg === dir) peak = P[i]![d]
      else {
        // how far it comes back before stopping or turning again
        let j = i
        let back = 0
        while (j < tr.length && V[j] && Math.sign(V[j]![d]) === sg && Math.abs(V[j]![d]) >= 0.05) back = Math.abs(P[j]![d] - peak), j++
        if (back > 1.5) add(cuedIn(since, i) ? "retarget" : "reverse", name + (dims > 1 ? "[" + d + "]" : ""), tr[i].t, `turns back ${back.toFixed(1)} px after ${((i - since) / 30).toFixed(2)} s`)
        dir = sg
        peak = P[i]![d]
        since = i
      }
    }
  }
}

// windows that appear or vanish while visible
for (const id of ids) {
  for (let i = 1; i < tr.length; i++) {
    const a = tr[i - 1].wins[id]
    const b = tr[i].wins[id]
    if (!a && b && b[4] > 0.5 && tr[i - 1].active === tr[i].active) add("pop", id, tr[i].t, `appears at alpha ${b[4]}`)
    if (a && !b && a[4] > 0.5 && tr[i - 1].active === tr[i].active) add("pop", id, tr[i].t, `vanishes at alpha ${a[4]}`)
  }
}

const by: Record<string, number> = {}
for (const x of issues) by[x.kind] = (by[x.kind] || 0) + 1
console.log(`${tr.length} frames, ${(tr.length / 30).toFixed(1)} s, ${ids.size} windows`)
console.log("issues:", JSON.stringify(by))
for (const kind of ["jump", "pop", "kink", "reverse", "retarget"]) {
  const xs = issues.filter((x) => x.kind === kind)
  if (!xs.length) continue
  console.log("\n" + kind)
  for (const x of xs.slice(0, 40)) console.log(`  ${x.t.toFixed(2).padStart(7)}  ${x.what.padEnd(14)} ${x.detail}`)
  if (xs.length > 40) console.log(`  … ${xs.length - 40} more`)
}
if (jsonOut) await Bun.write(jsonOut, JSON.stringify({ frames: tr.length, by, issues }, null, 1))
