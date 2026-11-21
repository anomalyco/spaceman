// The last shot. The camera pulls back from his screen through his dark study, out through the
// window behind him, until the house front is a photograph on someone else's screen.
// Each layer is a plane at depth z (in units of F, the distance at which his screen fills the frame).
import { FONT, MONO } from "./apps.js"
import { World } from "./world.js"
import { BAR_H, photoHole } from "./content.js"

const rgb = (h, a = 1) => [parseInt(h.slice(1, 3), 16) / 255, parseInt(h.slice(3, 5), 16) / 255, parseInt(h.slice(5, 7), 16) / 255, a]

export const Z_HEAD = 1.25
export const Z_FRONT = 3

let glowTex = null
export function glowCanvas() {
  if (glowTex) return glowTex
  const c = (glowTex = document.createElement("canvas"))
  c.width = c.height = 256
  const ctx = c.getContext("2d")
  const g = ctx.createRadialGradient(128, 128, 0, 128, 128, 128)
  g.addColorStop(0, "rgba(255,255,255,1)")
  g.addColorStop(0.35, "rgba(255,255,255,.45)")
  g.addColorStop(1, "rgba(255,255,255,0)")
  ctx.fillStyle = g
  ctx.fillRect(0, 0, 256, 256)
  c.version = 1
  return c
}

// The room around his monitor, on the plane of the screen (z = 0).
export function roomPrims(SW, SH, light) {
  const L = light
  const p = []
  p.push({ x: -40 * SW, y: -40 * SH, w: 80 * SW, h: 80 * SH, col: rgb("#0c0b0a") })
  p.push({ x: SW / 2 - 2.2 * SW, y: SH / 2 - 2.4 * SH, w: 4.4 * SW, h: 4.8 * SH, tex: glowCanvas(), col: [0.55, 0.62, 0.75, 0.2 * L], r: 0 })
  // desk
  const top = SH + 230
  p.push({ x: -2.6 * SW, y: top, w: 6.2 * SW, h: 3 * SH, col: rgb("#100e0c") })
  p.push({ x: -2.6 * SW, y: top, w: 6.2 * SW, h: 10, col: [0.2, 0.2, 0.22, 0.5 + 0.4 * L] })
  p.push({ x: SW / 2 - 1.6 * SW, y: top - 0.2 * SH, w: 3.2 * SW, h: 0.9 * SH, tex: glowCanvas(), col: [0.55, 0.62, 0.75, 0.12 * L], r: 0 })
  // monitor: stand, then body and bezel
  p.push({ x: SW / 2 - 70, y: SH + 10, w: 140, h: 210, col: rgb("#141416"), r: 6 })
  p.push({ x: SW / 2 - 230, y: top - 26, w: 460, h: 30, col: rgb("#18181b"), r: 14 })
  p.push({ x: -18, y: -18, w: SW + 36, h: SH + 36, col: rgb("#09090a"), r: 20 })
  p.push({ x: -18, y: -18, w: SW + 36, h: SH + 36, ring: 1.5, col: [1, 1, 1, 0.06], r: 20 })
  // a mug, a lamp that is off, a picture on the wall
  p.push({ x: SW + 160, y: top - 120, w: 86, h: 120, col: rgb("#0a0909"), r: 10 })
  p.push({ x: SW + 160, y: top - 120, w: 86, h: 120, ring: 2, col: [0.6, 0.66, 0.78, 0.12 * L], r: 10 })
  p.push({ x: -0.7 * SW, y: top - 0.9 * SH, w: 30, h: 0.9 * SH, col: rgb("#0a0a0a") })
  p.push({ x: -0.86 * SW, y: top - 1.18 * SH, w: 0.34 * SW, h: 0.32 * SH, col: rgb("#0b0b0b"), r: 0.14 * SH })
  p.push({ x: 1.45 * SW, y: -0.75 * SH, w: 0.55 * SW, h: 0.42 * SH, col: rgb("#121110") })
  p.push({ x: 1.45 * SW, y: -0.75 * SH, w: 0.55 * SW, h: 0.42 * SH, ring: 10, col: [0.3, 0.32, 0.36, 0.25 * L] })
  return p
}

// Him, from behind: head, shoulders and the back of the chair (z = Z_HEAD).
export function headPrims(SW, SH, light) {
  const rim = [0.62, 0.7, 0.84, 0.32 * light]
  const dark = rgb("#050506")
  const p = []
  const cx = SW * 0.5
  p.push({ x: cx - 0.5 * SW, y: SH * 1.1, w: 1.0 * SW, h: 2 * SH, col: rgb("#08080a"), r: 0.12 * SW })
  p.push({ x: cx - 0.5 * SW, y: SH * 1.1, w: 1.0 * SW, h: 2 * SH, ring: 3, col: [0.5, 0.56, 0.7, 0.1 * light], r: 0.12 * SW })
  p.push({ x: cx - 0.42 * SW, y: SH * 0.98, w: 0.84 * SW, h: 1.4 * SH, col: dark, r: 0.2 * SW })
  p.push({ x: cx - 0.42 * SW, y: SH * 0.98, w: 0.84 * SW, h: 1.4 * SH, ring: 3, col: rim, r: 0.2 * SW })
  p.push({ x: cx - 0.06 * SW, y: SH * 0.82, w: 0.12 * SW, h: 0.3 * SH, col: dark, r: 0.04 * SW })
  const hr = 0.115 * SW
  p.push({ x: cx - hr, y: SH * 0.82 - hr * 1.75, w: hr * 2, h: hr * 2.3, col: dark, r: hr })
  p.push({ x: cx - hr, y: SH * 0.82 - hr * 1.75, w: hr * 2, h: hr * 2.3, ring: 3, col: rim, r: hr })
  return p
}

// The front of his house at night, with the study window open onto the room (z = Z_FRONT).
// photo is the part of this plane that ends up in the photograph; s is the plane's current scale,
// used to fade in details that would otherwise arrive as giant shapes.
export function frontPrims(SW, SH, photo, light, s = 0.2) {
  const o = { x: -0.5 * SW, y: -0.62 * SH, w: 2 * SW, h: 2.15 * SH }
  const B = { x: photo.x - 0.4 * SW, y: photo.y - 0.4 * SH, w: photo.w + 0.8 * SW, h: photo.h + 0.8 * SH }
  const wall = rgb("#1a1e24")
  const siding = rgb("#15181d")
  const p = []
  const fine = clamp01((0.7 - s) / 0.35)
  // the wall, as four pieces around the window opening
  p.push({ x: B.x, y: B.y, w: B.w, h: o.y - B.y, col: wall })
  p.push({ x: B.x, y: o.y + o.h, w: B.w, h: B.y + B.h - o.y - o.h, col: wall })
  p.push({ x: B.x, y: o.y, w: o.x - B.x, h: o.h, col: wall })
  p.push({ x: o.x + o.w, y: o.y, w: B.x + B.w - o.x - o.w, h: o.h, col: wall })
  // siding boards
  const roofY = o.y - 0.78 * SH
  const ground = o.y + o.h + 0.55 * SH
  for (let y = roofY + 0.16 * SH; y < ground; y += 0.16 * SH) {
    if (y > o.y - 0.1 * SH && y < o.y + o.h + 0.12 * SH) {
      p.push({ x: B.x, y, w: o.x - B.x - 0.1 * SW, h: 0.012 * SH, col: siding, a: fine })
      p.push({ x: o.x + o.w + 0.1 * SW, y, w: B.x + B.w - o.x - o.w - 0.1 * SW, h: 0.012 * SH, col: siding, a: fine })
    } else p.push({ x: B.x, y, w: B.w, h: 0.012 * SH, col: siding, a: fine })
  }
  // the screen's light, spilling onto the siding around the window
  p.push({ x: o.x - 1.1 * SW, y: o.y - 1.0 * SH, w: o.w + 2.2 * SW, h: o.h + 2.0 * SH, tex: glowCanvas(), col: [0.5, 0.58, 0.72, 0.12 * light], r: 0 })
  // roof edge and gutter, downspout
  p.push({ x: B.x, y: B.y, w: B.w, h: roofY - B.y, col: rgb("#0a0c0f") })
  p.push({ x: B.x, y: roofY - 0.03 * SH, w: B.w, h: 0.07 * SH, col: rgb("#22262c") })
  p.push({ x: o.x - 1.05 * SW, y: roofY, w: 0.05 * SW, h: ground - roofY, col: rgb("#1f2329"), a: fine })
  // casing, sill, the meeting rail of the sash
  const trim = rgb("#2c3036")
  const tw = 0.07 * SW
  p.push({ x: o.x - tw, y: o.y - tw, w: o.w + 2 * tw, h: tw, col: trim })
  p.push({ x: o.x - tw, y: o.y, w: tw, h: o.h, col: trim })
  p.push({ x: o.x + o.w, y: o.y, w: tw, h: o.h, col: trim })
  p.push({ x: o.x - tw * 1.6, y: o.y + o.h, w: o.w + tw * 3.2, h: 0.08 * SH, col: rgb("#34383f") })
  p.push({ x: o.x, y: o.y + o.h * 0.3 - 0.02 * SH, w: o.w, h: 0.04 * SH, col: trim, a: fine })
  // a curtain, half drawn, inside
  p.push({ x: o.x, y: o.y, w: 0.22 * SW, h: o.h, col: [0.07, 0.07, 0.08, 0.94] })
  p.push({ x: o.x + o.w - 0.1 * SW, y: o.y, w: 0.1 * SW, h: o.h, col: [0.07, 0.07, 0.08, 0.94] })
  // foundation, a shrub under the window, the lawn
  p.push({ x: B.x, y: ground - 0.1 * SH, w: B.w, h: 0.1 * SH, col: rgb("#202327") })
  p.push({ x: o.x - 0.3 * SW, y: ground - 0.42 * SH, w: o.w + 0.6 * SW, h: 0.5 * SH, col: rgb("#0c120d"), r: 0.2 * SH })
  p.push({ x: B.x, y: ground, w: B.w, h: B.y + B.h - ground, col: rgb("#0f130f") })
  // two trash cans by the side of the house, put away by 8 PM
  for (const [k, cx] of [[0, 1.85], [1, 2.2]]) {
    p.push({ x: cx * SW, y: ground - 0.62 * SH, w: 0.3 * SW, h: 0.66 * SH, col: rgb(k ? "#141a16" : "#161a1e"), r: 0.03 * SW })
    p.push({ x: cx * SW - 0.015 * SW, y: ground - 0.66 * SH, w: 0.33 * SW, h: 0.07 * SH, col: rgb("#1c2126"), r: 0.02 * SW })
  }
  // house number by the door frame
  p.push({ x: -1.55 * SW, y: o.y + 0.2 * SH, w: 0.22 * SW, h: 0.3 * SH, tex: numberCanvas(), r: 0, a: fine })
  return { prims: p, opening: o }
}

const clamp01 = (x) => (x < 0 ? 0 : x > 1 ? 1 : x)

let numTex = null
function numberCanvas() {
  if (numTex) return numTex
  const c = (numTex = document.createElement("canvas"))
  c.width = 220
  c.height = 300
  const ctx = c.getContext("2d")
  ctx.fillStyle = "#1e2228"
  ctx.fillRect(0, 0, 220, 300)
  ctx.fillStyle = "rgba(200,200,190,.42)"
  ctx.font = `600 190px ${FONT}`
  ctx.textAlign = "center"
  ctx.textBaseline = "middle"
  ctx.fillText("12", 110, 160)
  c.version = 1
  return c
}

// The outer desktop. Its own Mac, its own Spaceman, its own bar.
export function makeOuter(SW, SH, res) {
  const o = new World(SW, SH, { wall: "tidy" })
  o.res = res
  o.tiling = true
  o.status = true
  o.clock = "Fri 11:48 PM"
  o.frontApp = "Viewer"
  const viewer = o.add({ id: "o-photo", app: "viewer", kind: "viewer2", title: "No. 12 — Study", ws: 5, data: {} })
  o.add({ id: "o-height", app: "sheets", kind: "sheet", title: "Hedge — Height", ws: 5, data: { cols: ["Date", "Height"], widths: [0.55, 0.45], rows: ["Nov 14", "Nov 15", "Nov 16", "Nov 17", "Nov 18", "Nov 19", "Nov 20"].map((d) => ({ cells: [d, "6′ 0″"] })) } })
  o.add({ id: "o-min", app: "paper", kind: "paper", title: "Minutes — Corrections", ws: 5, data: { heading: "Corrections from No. 14", lines: ["Item 2. I did not agree to this.", "Item 3. Measured from the", "street side, not the path.", "Item 4. I was present.", { gap: 1 }, { t: "Not sent", soft: 1 }] } })
  const others = { 1: ["postbox", "mail", "Postbox"], 2: ["browsy", "browser", "Browsy"], 3: ["jot", "jot", "Jot"], 4: ["daybook", "daybook", "Daybook"] }
  for (const [n, [app, kind, title]] of Object.entries(others)) o.add({ id: "o-x" + n, app, kind, title, ws: +n, data: kind === "daybook" ? { date: "Saturday, November 21", events: [] } : { heading: title } })
  o.active = 5
  o.ensureCell(6)
  for (const c of o.bar.cells) {
    c.s.x = c.s.t = c.n === 5 ? 1 : 0.62
    c.o.x = c.o.t = c.n === 5 ? 1 : 0.5
  }
  o.bar.shown.x = o.bar.shown.t = 1
  o.showBar()
  o.full.clear()
  // the photograph takes the left two thirds; the notes stack on the right
  const a = o.area
  const tall = SH > SW * 1.05
  const rects = tall
    ? {
        "o-photo": { x: a.x + 4, y: a.y + 4, w: a.w - 8, h: a.h * 0.56 - 8 },
        "o-height": { x: a.x + 4, y: a.y + a.h * 0.56 + 4, w: a.w * 0.5 - 8, h: a.h * 0.44 - 8 },
        "o-min": { x: a.x + a.w * 0.5 + 4, y: a.y + a.h * 0.56 + 4, w: a.w * 0.5 - 8, h: a.h * 0.44 - 8 },
      }
    : {
        "o-photo": { x: a.x + 4, y: a.y + 4, w: a.w * 0.64 - 8, h: a.h - 8 },
        "o-height": { x: a.x + a.w * 0.64 + 4, y: a.y + 4, w: a.w * 0.36 - 8, h: a.h * 0.5 - 8 },
        "o-min": { x: a.x + a.w * 0.64 + 4, y: a.y + a.h * 0.5 + 4, w: a.w * 0.36 - 8, h: a.h * 0.5 - 8 },
      }
  o.rectsOf = (ws) => (ws === 5 ? new Map(Object.entries(rects)) : new Map(o.wins.has("o-x" + ws) ? [["o-x" + ws, { x: a.x + 4, y: a.y + 4, w: a.w - 8, h: a.h - 8 }]] : []))
  for (const [id, r] of Object.entries(rects)) {
    const w = o.wins.get(id)
    for (const k of ["x", "y", "w", "h"]) w[k].x = w[k].t = r[k]
  }
  o.setFocus("o-photo")
  o.cursor.x.x = o.cursor.x.t = SW * 0.5
  o.cursor.y.x = o.cursor.y.t = SH - 120
  const pr = rects["o-photo"]
  const hole = photoHole(pr.w, pr.h, SW / SH)
  o.photoRect = { x: pr.x + hole.x, y: pr.y + hole.y, w: hole.w, h: hole.h }
  viewer.data = { aspect: SW / SH }
  void viewer
  return o
}

// The stamp a camera burns into the corner of a photo.
let stampTex = null
export function stampCanvas(text) {
  if (stampTex?.text === text) return stampTex
  const c = (stampTex = document.createElement("canvas"))
  c.text = text
  c.width = 520
  c.height = 64
  const ctx = c.getContext("2d")
  ctx.font = `600 34px ${MONO}`
  ctx.fillStyle = "rgba(240,150,60,.9)"
  ctx.textBaseline = "middle"
  ctx.fillText(text, 8, 34)
  c.version = 1
  return c
}

export { rgb, FONT }

