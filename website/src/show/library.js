// His album "Hedge": 4,812 phone photos of No. 14 and its hedge, in three days' sections, drawn as
// the Photos grid: square thumbnails edge to edge with 2 px gaps, a header per day that sticks while
// its photos scroll under it, thumbnails that load in as their rows come into view. Daylight
// for most of it, then afternoons and dusk; only the last row is at night, ending on his window with
// him in it, looking back. Photos are in public/show/photos (show/notes/photos.md).
import { FONT, roundRect } from "./apps.js"
import { plates, coverUV } from "./plates.js"
import { BAR_H } from "./content.js"

const DAY = ["d-sun", "d-angle", "d-frost", "d-drizzle", "d-tape", "d-road", "d-shoots", "d-corner", "d-fog", "d-leaves", "d-snow", "d-car"]
const DUSK = ["e-gold", "e-blue", "e-zoom"]
const NIGHT = ["n-rain", "n-light", "n-alan"]
const ALL = [...DAY, ...DUSK, ...NIGHT]
export const PHOTOS = ALL

export const ALBUM = 4812
// the days, as Photos groups them (count of photos in each)
const SECTIONS = [
  { date: "Sunday, Dec 14", short: "Dec 14", n: 1604 },
  { date: "Thursday, Jan 8", short: "Jan 8", n: 1890 },
  { date: "Saturday, Jan 31", short: "Jan 31", n: ALBUM - 1604 - 1890 },
]
const START = SECTIONS.reduce((a, s) => [...a, a.at(-1) + s.n], [0])
const PLACE = "Pinecrest"

const hash = (n) => {
  n = (n ^ 61) ^ (n >>> 16)
  n = Math.imul(n, 9) >>> 0
  n ^= n >>> 4
  n = Math.imul(n, 0x27d4eb2d) >>> 0
  return ((n ^ (n >>> 15)) >>> 0) / 4294967296
}
const h01 = (i, k) => hash(i * 131 + k * 7919)

// Which photo each tile shows, for a grid this many columns wide. Never the same picture as any
// neighbour (left, the row above, two rows up), so no two nearby thumbnails look like copies.
const assigned = new Map()
function assign(cols) {
  let a = assigned.get(cols)
  if (a) return a
  a = new Int8Array(ALBUM)
  const tail = SECTIONS.at(-1).n % cols || cols
  const lastRow = ALBUM - tail
  const night = { [ALBUM - 1]: "n-alan", [ALBUM - 2]: "n-light" }
  for (let k = 0; k < ALBUM; k++) {
    if (night[k]) {
      a[k] = ALL.indexOf(night[k])
      continue
    }
    const f = k / ALBUM
    let pool
    if (k >= lastRow) pool = ["n-rain", "e-zoom"]
    else if (f < 0.75) pool = DAY
    else {
      // the afternoons get later: more and more of the evening ones toward the end
      const e = (f - 0.75) / 0.25
      pool = h01(k, 21) < 0.15 + e * 0.7 ? ["e-gold", "e-blue", "e-gold", "e-blue", "d-corner", "d-sun", "e-zoom"] : DAY
    }
    const near = new Set([k - 1, k - 2, k - cols, k - cols - 1, k - cols + 1, k - 2 * cols].filter((j) => j >= 0).map((j) => a[j]))
    const ok = pool.map((p) => ALL.indexOf(p)).filter((i) => !near.has(i))
    const from = ok.length ? ok : pool.map((p) => ALL.indexOf(p))
    a[k] = from[Math.floor(h01(k, 3) * from.length)]
  }
  assigned.set(cols, a)
  return a
}
const AWD = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
export function albumAt(k, cols) {
  const s = SECTIONS.findIndex((_, i) => k < START[i + 1])
  const plate = ALL[assign(cols)[k]]
  const t = k === ALBUM - 1 ? "11:52 PM" : null
  return { plate, section: s, date: SECTIONS[s].short, wd: SECTIONS[s].date.split(",")[0].slice(0, 3), time: t, night: plate.startsWith("n-") }
}
void AWD

const GAP = 2
const HEAD = 46
const TOOL = 40
// The grid, for the painter and the director alike.
export function albumGrid(w, h) {
  const top = BAR_H + TOOL
  const cols = Math.max(4, Math.round(w / 112))
  const tw = (w - (cols - 1) * GAP) / cols
  const cell = tw + GAP
  const secTop = []
  let y = 0
  for (const s of SECTIONS) {
    secTop.push(y)
    y += HEAD + Math.ceil(s.n / cols) * cell
  }
  const total = y
  const view = h - top
  const maxScroll = Math.max(0, total - view)
  const sectionOf = (k) => SECTIONS.findIndex((_, i) => k < START[i + 1])
  const rowY = (k) => {
    const s = sectionOf(k)
    return secTop[s] + HEAD + Math.floor((k - START[s]) / cols) * cell
  }
  const tile = (k, scroll) => {
    const s = sectionOf(k)
    const i = k - START[s]
    return { x: (i % cols) * cell, y: top + rowY(k) - scroll, w: tw, h: tw }
  }
  // the scroll that puts entry k's row a fraction f of the way down the view
  const scrollTo = (k, f = 0.42) => Math.max(0, Math.min(maxScroll, rowY(k) + tw / 2 - view * f))
  return { top, cols, tw, cell, secTop, total, view, maxScroll, tile, scrollTo, rowY }
}

// His scroll, as a pure function of show time: each throw coasts to its stop (velocity decaying
// exponentially after the finger's push gets going); a step with no length is a jump (⌘↓).
export function scrollOn(path, t, reduced = false) {
  let s = 0
  for (const f of path || []) {
    if (t < f.t0) break
    const from = s
    if (f.t1 <= f.t0 || reduced) {
      s = f.to
      continue
    }
    const c = 0.07
    const e = (x) => x - c * (1 - Math.exp(-x / c))
    const k = (1 - Math.exp(-e(Math.min(t, f.t1) - f.t0) / f.tau)) / (1 - Math.exp(-e(f.t1 - f.t0) / f.tau))
    s = from + (f.to - from) * k
  }
  return s
}

// when a row first came into view: the album opening, the scroll bringing it up from below, or a jump
// (the scroll only ever goes down, so a bisection finds it)
function arrival(G, d, y) {
  if (y < scrollOn(d.path, d.open0) + G.view) return { at: d.open0, from: y - scrollOn(d.path, d.open0) }
  let lo = d.open0
  let hi = d.now
  if (scrollOn(d.path, hi) + G.view <= y) return null
  for (let i = 0; i < 22; i++) {
    const m = (lo + hi) / 2
    if (scrollOn(d.path, m) + G.view > y) hi = m
    else lo = m
  }
  return { at: hi, from: y - scrollOn(d.path, hi) }
}

// per photo: a small copy for the grid (whole frame, 300 px high) and a tiny one, blurred, for the
// moment before a thumbnail arrives
const small = new Map()
function smallOf(p) {
  let c = small.get(p)
  if (c) return c
  const mk = (H) => {
    const k = document.createElement("canvas")
    k.height = H
    k.width = Math.max(1, Math.round((p.width / p.height) * H))
    const x = k.getContext("2d")
    x.imageSmoothingQuality = "high"
    x.drawImage(p, 0, 0, k.width, k.height)
    return k
  }
  c = { sharp: mk(300), soft: mk(9) }
  small.set(p, c)
  return c
}

// A tile: its own crop of the photo (each frames the house a little differently). Before it loads
// it's a grey placeholder; then the tiny soft copy, then the sharp one over it (soft, sharp: 0..1).
function drawTile(ctx, e, k, r, soft01, a) {
  const p = plates[e.plate]
  if (soft01 < 1) {
    ctx.fillStyle = "#2c2c2f"
    ctx.fillRect(r.x, r.y, r.w, r.h)
  }
  if (soft01 <= 0) return
  if (!p) {
    ctx.fillStyle = e.night ? "#1c1c20" : "#3d4438"
    ctx.fillRect(r.x, r.y, r.w, r.h)
    return
  }
  const { sharp, soft } = smallOf(p)
  // a square window into the frame: zoomed a little and slid along it, per tile
  const last = e.plate === "n-alan" || e.plate === "n-light"
  const z = last ? 1.15 : 1 + h01(k, 1) * 0.5
  const src = (c) => {
    const side = Math.min(c.width, c.height) / z
    const u = last ? 0.5 : h01(k, 2)
    const v = last ? 0.45 : 0.25 + h01(k, 4) * 0.5
    return [(c.width - side) * u, (c.height - side) * v, side, side]
  }
  ctx.save()
  ctx.beginPath()
  ctx.rect(r.x, r.y, r.w, r.h)
  ctx.clip()
  if (a < 1) {
    ctx.imageSmoothingEnabled = true
    ctx.globalAlpha = soft01
    const [sx, sy, sw, sh] = src(soft)
    ctx.drawImage(soft, sx, sy, sw, sh, r.x - 2, r.y - 2, r.w + 4, r.h + 4)
    ctx.globalAlpha = 1
  }
  if (a > 0) {
    ctx.globalAlpha = a
    const [sx, sy, sw, sh] = src(sharp)
    ctx.drawImage(sharp, sx, sy, sw, sh, r.x, r.y, r.w, r.h)
    // that moment's exposure, a touch over or under
    const ex = h01(k, 5) - 0.5
    ctx.fillStyle = ex < 0 ? "rgba(20,24,30," + (-ex * 0.18).toFixed(3) + ")" : "rgba(255,244,228," + (ex * 0.1).toFixed(3) + ")"
    ctx.fillRect(r.x, r.y, r.w, r.h)
  }
  ctx.restore()
}

function label(ctx, s, x, y, size, weight, color, align = "left") {
  ctx.font = weight + " " + size + "px " + FONT
  ctx.fillStyle = color
  ctx.textAlign = align
  ctx.textBaseline = "alphabetic"
  ctx.fillText(s, x, y)
}

function header(ctx, s, y, w, solid) {
  if (solid) {
    ctx.fillStyle = "rgba(30,30,32,.9)"
    ctx.fillRect(0, y, w, HEAD)
    ctx.fillStyle = "rgba(255,255,255,.06)"
    ctx.fillRect(0, y + HEAD - 1, w, 1)
  }
  label(ctx, SECTIONS[s].date, 14, y + 22, 15, 700, "#f2f2f4")
  label(ctx, PLACE + " · " + SECTIONS[s].n.toLocaleString("en-US") + " Photos", 14, y + 38, 12, 400, "#8e8e93")
}

export function album(ctx, w, h, d) {
  const G = albumGrid(w, h)
  const scroll = Math.max(0, Math.min(G.maxScroll, d.scroll || 0))
  ctx.fillStyle = "#1e1e20"
  ctx.fillRect(0, BAR_H, w, h - BAR_H)
  ctx.save()
  ctx.beginPath()
  ctx.rect(0, G.top, w, h - G.top)
  ctx.clip()
  for (let s = 0; s < SECTIONS.length; s++) {
    const y0 = G.top + G.secTop[s] - scroll
    const rows = Math.ceil(SECTIONS[s].n / G.cols)
    if (y0 > h || y0 + HEAD + rows * G.cell < G.top) continue
    header(ctx, s, y0, w, false)
    const r0 = Math.max(0, Math.floor((G.top - y0 - HEAD) / G.cell))
    const r1 = Math.min(rows, Math.ceil((h - y0 - HEAD) / G.cell) + 1)
    for (let row = r0; row < r1; row++)
      for (let c = 0; c < G.cols; c++) {
        const k = START[s] + row * G.cols + c
        if (k >= START[s + 1]) break
        const r = G.tile(k, scroll)
        // the thumbnails load as their row comes into view, in a quick wave: left to right with a
        // little scatter, rows further down a touch later
        let soft01 = 1
        let a = 1
        if (d.path) {
          const arr = arrival(G, d, G.rowY(k))
          const c = (k - START[s]) % G.cols
          const d0 = arr ? arr.at + 0.06 + c * 0.03 + h01(k, 7) * 0.07 + Math.min(4, Math.max(0, arr.from / G.cell)) * 0.05 : Infinity
          soft01 = Math.max(0, Math.min(1, (d.now - d0) / 0.07))
          a = Math.max(0, Math.min(1, (d.now - d0 - 0.06) / 0.2))
        }
        drawTile(ctx, albumAt(k, G.cols), k, r, soft01, a)
        if (d.sel === k) {
          ctx.strokeStyle = "#0a84ff"
          ctx.lineWidth = 3
          ctx.strokeRect(r.x + 1.5, r.y + 1.5, r.w - 3, r.h - 3)
        }
      }
  }
  // the day you're in stays pinned under the toolbar
  const cur = G.secTop.findLastIndex((y) => y <= scroll)
  if (cur >= 0 && scroll > G.secTop[cur] + 0.5) {
    const next = G.secTop[cur + 1]
    const push = next != null ? Math.min(0, next - scroll - HEAD) : 0
    header(ctx, cur, G.top + push, w, true)
  }
  if (d.bar > 0 && G.maxScroll > 0) {
    const vh = 34
    const y = G.top + 6 + (scroll / G.maxScroll) * (G.view - 12 - vh)
    ctx.fillStyle = "rgba(255,255,255," + (0.42 * d.bar).toFixed(3) + ")"
    roundRect(ctx, w - 9, y, 5, vh, 2.5)
    ctx.fill()
  }
  ctx.restore()
  // the toolbar: the album and its count
  ctx.fillStyle = "#2a2a2c"
  ctx.fillRect(0, BAR_H, w, TOOL)
  ctx.fillStyle = "rgba(0,0,0,.5)"
  ctx.fillRect(0, BAR_H + TOOL - 1, w, 1)
  label(ctx, "Hedge", 16, BAR_H + 26, 16, 700, "#f2f2f4")
  ctx.font = "700 16px " + FONT
  label(ctx, ALBUM.toLocaleString("en-US") + " Photos", 16 + ctx.measureText("Hedge").width + 9, BAR_H + 26, 13, 400, "#98989d")
  // one photo, opening out of its tile (a = 0: its tile; 1: the whole window)
  const a = d.openT || 0
  if (d.open != null && a > 0) {
    const e = albumAt(d.open, G.cols)
    const t = G.tile(d.open, scroll)
    const F = { x: 0, y: G.top, w, h: h - G.top - 34 }
    const R = { x: t.x + (F.x - t.x) * a, y: t.y + (F.y - t.y) * a, w: t.w + (F.w - t.w) * a, h: t.h + (F.h - t.h) * a }
    ctx.fillStyle = "rgba(12,12,14," + Math.min(1, a * 1.4).toFixed(3) + ")"
    ctx.fillRect(0, G.top, w, h - G.top)
    const p = plates[e.plate]
    if (p) {
      ctx.save()
      ctx.beginPath()
      ctx.rect(R.x, R.y, R.w, R.h)
      ctx.clip()
      const [u0, v0, u1, v1] = coverUV(p, R.w, R.h, 0.5, 0.45)
      ctx.drawImage(p, u0 * p.width, v0 * p.height, (u1 - u0) * p.width, (v1 - v0) * p.height, R.x, R.y, R.w, R.h)
      ctx.restore()
    }
    if (a > 0.6) {
      ctx.globalAlpha = Math.min(1, (a - 0.6) / 0.3)
      label(ctx, SECTIONS[e.section].date + (e.section ? ", 2026 · " : ", 2025 · ") + (e.time || ""), 14, h - 12, 13.5, 600, "#d9d5ce")
      label(ctx, (d.open + 1).toLocaleString("en-US") + " of " + ALBUM.toLocaleString("en-US"), w - 14, h - 12, 13, 400, "#9d988f", "right")
      ctx.globalAlpha = 1
    }
  }
}
