// The man's Mac: workspaces, tiled windows, the bar, the cursor and the menu bar, in logical screen
// units. draw() appends rounded-rect primitives to a display list that both renderers consume.
import { APPS, FONT, iconCanvas } from "./apps.js"
import { paintWindow, BAR_H } from "./content.js"
import { plates, blurOf, coverUV } from "./plates.js"
import { sp, to, jump, step, clamp } from "./springs.js"

export const MENU_H = 26
const GAP = 8
const EDGE_PAD = 10
// The Grow track: { w: 50, ht: 32, small: 0.62, gap: 8, pad: 3 }
const BAR = { w: 50, ht: 32, small: 0.62, gap: 8, pad: 3, edge: 12, inner: 42 }
// A tiler's moves are short: windows land in about a third of a second, a beat apart.
const TILE_D = 0.34
const STAGGER = 0.024

export function tile(n, a) {
  const out = []
  if (!n) return out
  const portrait = a.h > a.w * 1.05
  const T = (x, y, w, h) => (portrait ? { x: a.x + y * a.w, y: a.y + x * a.h, w: h * a.w, h: w * a.h } : { x: a.x + x * a.w, y: a.y + y * a.h, w: w * a.w, h: h * a.h })
  // fractions in a "landscape" frame, transposed for portrait areas
  const fr = []
  if (n === 1) fr.push([0, 0, 1, 1])
  else if (n === 2) fr.push([0, 0, 0.5, 1], [0.5, 0, 0.5, 1])
  else if (n === 3) fr.push([0, 0, 0.5, 1], [0.5, 0, 0.5, 0.5], [0.5, 0.5, 0.5, 0.5])
  else {
    const aspect = portrait ? a.h / a.w : a.w / a.h
    // rows as even as they can be, the fuller rows last: 7 is 3 over 4, never 3, 3 and a lone 1
    const rows = n === 4 ? 2 : Math.max(1, Math.round(Math.sqrt(n / (aspect * 0.8))))
    let k = 0
    for (let r = 0; r < rows; r++) {
      const inRow = Math.floor(n / rows) + (r >= rows - (n % rows) ? 1 : 0)
      for (let c = 0; c < inRow; c++, k++) fr.push([c / inRow, r / rows, 1 / inRow, 1 / rows])
    }
  }
  for (const [x, y, w, h] of fr) {
    const r = T(x, y, w, h)
    out.push({ x: r.x + GAP / 2, y: r.y + GAP / 2, w: r.w - GAP, h: r.h - GAP })
  }
  return out
}

export class World {
  constructor(SW, SH, opts = {}) {
    this.SW = SW
    this.SH = SH
    this.area = { x: EDGE_PAD - GAP / 2, y: MENU_H + EDGE_PAD - GAP / 2, w: SW - 2 * EDGE_PAD + GAP, h: SH - MENU_H - 2 * EDGE_PAD - 80 + GAP }
    this.wins = new Map()
    this.order = new Map() // ws -> [ids] in tile order
    this.active = 1
    this.focus = null
    this.tiling = false
    this.res = 1
    this.z = 0
    this.clock = "Tue 11:48 PM"
    this.status = false
    this.frontApp = "Jot"
    this.menuCanvas = null
    this.menuKey = ""
    this.bar = { shown: sp(0, 0.42), scale: sp(1, 0.4), cells: [], left: sp(0, 0.3), right: sp(0, 0.3), count: 0 }
    this.cursor = { x: SW * 0.62, y: SH * 0.55, vx: 0, vy: 0, path: null, a: sp(1, 0.3), press: sp(0, 0.09) }
    this.keys = { label: null, a: sp(0, 0.2), until: 0 }
    this.time = 0
    this.lastFocus = {}
    this.reduced = !!opts.reduced
    this.repaints = 0
  }

  get wall() {
    return plates.wall || plates["wall-s"] || fallbackWall()
  }

  add(def) {
    const w = {
      ws: 1,
      float: false,
      ...def,
      x: sp(def.at?.[0] ?? 100, TILE_D),
      y: sp(def.at?.[1] ?? 100, TILE_D),
      w: sp(def.at?.[2] ?? 480, TILE_D),
      h: sp(def.at?.[3] ?? 340, TILE_D),
      a: sp(def.hidden ? 0 : 1, 0.2),
      s: sp(1, 0.26),
      off: sp(0, 0.2),
      fo: sp(0, 0.24),
      z: ++this.z,
      canvas: null,
      dirty: true,
      cw: 0,
      ch: 0,
      leaving: false,
    }
    this.wins.set(w.id, w)
    if (!w.float) this.listOf(w.ws).push(w.id)
    return w
  }

  listOf(ws) {
    let l = this.order.get(ws)
    if (!l) this.order.set(ws, (l = []))
    return l
  }

  rectsOf(ws) {
    const ids = this.listOf(ws).filter((id) => !this.wins.get(id).leaving)
    const rs = tile(ids.length, this.area)
    return new Map(ids.map((id, i) => [id, rs[i]]))
  }

  // Retarget every tiled window on the active workspace to its tile, a beat apart in tile order.
  reflow({ snap = false, delay = 0 } = {}) {
    if (!this.tiling) return
    let i = 0
    for (const [id, r] of this.rectsOf(this.active)) {
      const w = this.wins.get(id)
      if (snap) for (const k of ["x", "y", "w", "h"]) jump(w[k], r[k])
      else if (Math.abs(w.x.t - r.x) + Math.abs(w.y.t - r.y) + Math.abs(w.w.t - r.w) + Math.abs(w.h.t - r.h) > 0.5) {
        w.target = r
        w.delay = delay + i++ * STAGGER
      }
    }
  }

  tileNow() {
    this.tiling = true
    this.reflow()
  }

  setFocus(id) {
    const prev = this.wins.get(this.focus)
    if (prev && prev.id !== id) {
      prev.dirty = true
      to(prev.fo, 0)
    }
    this.focus = id
    const w = this.wins.get(id)
    if (w) {
      w.z = ++this.z
      w.dirty = true
      to(w.fo, 1)
      this.frontApp = APPS[w.app]?.name || "Jot"
      this.lastFocus[w.ws] = id
    }
  }

  // Change a window's title or contents; it repaints on the next frame.
  set(id, patch) {
    const w = this.wins.get(id)
    if (!w) return
    if (patch.data) w.data = { ...w.data, ...patch.data }
    if (patch.title != null) w.title = patch.title
    w.dirty = true
  }

  open(def, { focus = true } = {}) {
    const w = this.add({ ...def, hidden: true })
    const r = (!w.float && this.tiling && this.rectsOf(w.ws).get(w.id)) || { x: def.at[0], y: def.at[1], w: def.at[2], h: def.at[3] }
    for (const k of ["x", "y", "w", "h"]) jump(w[k], r[k])
    jump(w.s, 0.95)
    to(w.s, 1)
    to(w.a, 1)
    if (w.ws === this.active) this.reflow()
    if (focus) this.setFocus(w.id)
    return w
  }

  // Several windows at once (a workspace being filled): each goes straight to its final tile and they
  // appear a beat apart, so nothing already on screen is moved twice.
  openMany(defs, { gap = 0.16 } = {}) {
    const ws = defs[0].ws
    const made = defs.map((d) => this.add({ ...d, hidden: true }))
    const rs = this.rectsOf(ws)
    made.forEach((m, i) => {
      const r = (this.tiling && rs.get(m.id)) || { x: m.at?.[0] ?? 100, y: m.at?.[1] ?? 100, w: m.at?.[2] ?? 480, h: m.at?.[3] ?? 340 }
      for (const k of ["x", "y", "w", "h"]) jump(m[k], r[k])
      jump(m.s, 0.95)
      m.revealAt = this.time + i * gap
    })
    if (ws === this.active) this.reflow()
    return made
  }

  // The window fades and shrinks a little; its neighbours wait a beat, then close the gap.
  close(id) {
    const w = this.wins.get(id)
    if (!w) return
    w.leaving = true
    to(w.s, 0.95, 0.22)
    to(w.a, 0, 0.18)
    w.removeAt = this.time + 0.32
    if (this.focus === id) this.focus = null
    this.reflow({ delay: 0.07 })
  }

  // Option+Shift+n: the window leaves for workspace n, shrinking into its little layout in the bar.
  move(id, ws) {
    const w = this.wins.get(id)
    const from = this.listOf(w.ws)
    if (from.includes(id)) from.splice(from.indexOf(id), 1)
    w.float = false
    this.ensureCell(ws)
    w.ws = ws
    this.listOf(ws).push(id)
    const c = this.cellRect(ws)
    w.target = null
    if (this.reduced) {
      to(w.a, 0, 0.24)
    } else {
      // one duration for every edge, so no edge doubles back on its way into the bar
      for (const k of ["x", "y", "w", "h"]) to(w[k], c[k], 0.34)
      to(w.a, 0, 0.3)
    }
    w.flying = this.time + 0.42
    if (this.focus === id) {
      const next = [...this.rectsOf(this.active).keys()].pop()
      if (next) this.setFocus(next)
    }
    this.reflow({ delay: 0.05 })
  }

  switchTo(ws, { quiet } = {}) {
    if (ws === this.active) return
    const dir = this.reduced ? 0 : Math.sign(ws - this.active)
    this.ensureCell(ws)
    for (const w of this.wins.values()) {
      if (w.float || w.leaving) continue
      if (w.ws === this.active) {
        to(w.off, -dir * 32 || -0.001, 0.16)
        to(w.a, 0, 0.11)
      }
    }
    const prev = this.active
    this.active = ws
    for (const [id, r] of this.rectsOf(ws)) {
      const w = this.wins.get(id)
      w.target = null
      for (const k of ["x", "y", "w", "h"]) jump(w[k], r[k])
      jump(w.off, dir * 32)
      to(w.off, 0, 0.22)
      jump(w.a, 0)
      to(w.a, 1, 0.15)
      jump(w.s, 1)
      if (Math.abs(w.cw - r.w) > 0.5 || Math.abs(w.ch - r.h) > 0.5) w.dirty = true
    }
    const ids = [...this.rectsOf(ws).keys()]
    const keep = this.lastFocus?.[ws]
    if (ids.length) this.setFocus(ids.includes(keep) ? keep : ids[ids.length - 1])
    this.barGo(prev, ws)
    if (!quiet) this.keyCast(["⌥", String(ws % 10)])
  }

  keyCast(label, hold = 1.0) {
    const same = this.keys.label && this.keys.a.t > 0 && this.keys.label.length === label.length
    this.keys.label = label
    this.keys.until = this.time + hold
    to(this.keys.a, 1)
    this.keys.canvas = null
    if (!same) this.keys.pop = this.time
  }

  // ─── Bar ────────────────────────────────────────────────────────────────
  ensureCell(ws) {
    while (this.bar.cells.length < ws) {
      const n = this.bar.cells.length + 1
      this.bar.cells.push({ n, s: sp(0, 0.3), o: sp(0, 0.3) })
      const c = this.bar.cells[n - 1]
      to(c.s, n === this.active ? 1 : BAR.small)
      to(c.o, n === this.active ? 1 : 0.5)
    }
  }

  barLayout(active) {
    let x = BAR.edge
    return this.bar.cells.map((c, i) => {
      const width = BAR.w * (i + 1 === active ? 1 : BAR.small)
      const r = { x, width }
      x += width + BAR.gap
      return r
    })
  }

  // The marker: its leading edge 0.26 s, its trailing edge 0.42 s.
  barGo(from, ws) {
    const dir = Math.sign(ws - from)
    this.bar.cells.forEach((c, j) => {
      to(c.s, j + 1 === ws ? 1 : BAR.small)
      to(c.o, j + 1 === ws ? 1 : 0.5)
    })
    const r = this.barLayout(ws)[ws - 1]
    to(this.bar.left, r.x - BAR.pad, dir > 0 ? 0.42 : dir < 0 ? 0.26 : 0.3)
    to(this.bar.right, r.x + r.width + BAR.pad, dir > 0 ? 0.26 : dir < 0 ? 0.42 : 0.3)
  }

  showBar() {
    this.ensureCell(Math.max(1, this.active))
    to(this.bar.shown, 1)
    const r = this.barLayout(this.active)[this.active - 1]
    jump(this.bar.left, r.x - BAR.pad)
    jump(this.bar.right, r.x + r.width + BAR.pad)
  }

  barGeom() {
    const sc = this.bar.scale.x
    let width = BAR.edge * 2 + 2
    this.bar.cells.forEach((c, i) => (width += BAR.w * c.s.x + (i ? BAR.gap * Math.min(1, c.s.x / BAR.small) : 0)))
    const h = (BAR.inner + 2) * sc
    const w = width * sc
    const x = this.SW / 2 - w / 2
    const y = this.SH - 12 - h + (1 - this.bar.shown.x) * 70
    return { x, y, w, h, sc }
  }

  cellRect(ws) {
    const g = this.barGeom()
    const r = this.barLayout(this.active)[ws - 1] || { x: g.w / g.sc - 40, width: BAR.w * BAR.small }
    const s = ws === this.active ? 1 : BAR.small
    return { x: g.x + r.x * g.sc, y: g.y + (BAR.inner / 2 - (BAR.ht * s) / 2 + 1) * g.sc, w: r.width * g.sc, h: BAR.ht * s * g.sc }
  }

  // ─── Simulation ─────────────────────────────────────────────────────────
  update(dt) {
    this.time += dt
    for (const w of [...this.wins.values()]) {
      if (w.target) {
        w.delay -= dt
        if (w.delay <= 0) {
          for (const k of ["x", "y", "w", "h"]) to(w[k], w.target[k], TILE_D)
          w.dirty = true
          w.target = null
        }
      }
      for (const k of ["x", "y", "w", "h", "a", "s", "off", "fo"]) step(w[k], dt)
      if (w.removeAt && this.time >= w.removeAt) {
        this.wins.delete(w.id)
        const l = this.listOf(w.ws)
        if (l.includes(w.id)) l.splice(l.indexOf(w.id), 1)
      }
      if (w.flying && this.time >= w.flying) w.flying = 0
      if (w.revealAt !== undefined && this.time >= w.revealAt) {
        w.revealAt = undefined
        to(w.s, 1)
        to(w.a, 1)
        if (w.ws === this.active) this.setFocus(w.id)
      }
      // once a workspace's windows have faded out, they rest where they are
      if (!w.float && w.ws !== this.active && w.a.t === 0 && w.a.x < 0.002 && w.off.x !== 0) jump(w.off, 0)
    }
    const b = this.bar
    for (const k of ["shown", "scale", "left", "right"]) step(b[k], dt)
    for (const c of b.cells) {
      step(c.s, dt)
      step(c.o, dt)
    }
    this.stepCursor(dt)
    if (this.keys.label && this.time > this.keys.until) to(this.keys.a, 0)
    step(this.keys.a, dt)
  }

  // The cursor travels on a slight arc, speeding up and settling (a Hermite curve that starts with
  // whatever velocity it had), and takes longer for longer trips, the way a hand does.
  moveCursor(x, y, d) {
    const c = this.cursor
    const dx = x - c.x
    const dy = y - c.y
    const dist = Math.hypot(dx, dy)
    if (dist < 0.5) return
    const dur = d ?? cursorTime(dist)
    let nx = -dy / dist
    let ny = dx / dist
    if (ny > 0 || (Math.abs(ny) < 0.2 && nx * dx < 0)) {
      nx = -nx
      ny = -ny
    }
    c.path = { x0: c.x, y0: c.y, x1: x, y1: y, mx: c.vx * dur, my: c.vy * dur, t0: this.time, dur, bx: nx * dist * 0.06, by: ny * dist * 0.06 }
  }

  // Set off in time to arrive at arriveAt (show time), or now if the trip needs longer.
  cursorTo(x, y, arriveAt) {
    const c = this.cursor
    const dist = Math.hypot(x - c.x, y - c.y)
    if (dist < 0.5) return
    // a trip that would land late is made quicker, so the click never lands on a moving cursor
    const dur = Math.max(0.22, Math.min(cursorTime(dist), arriveAt - this.time))
    // a hand already moving doesn't stop to wait: it carries on to the new point
    const t0 = c.path ? this.time : Math.max(this.time, arriveAt - dur)
    this.moveCursor(x, y, dur)
    const p = this.cursor.path
    if (t0 > this.time + 1e-4) {
      // waiting to move: no carried velocity
      p.t0 = t0
      p.mx = p.my = 0
    }
  }

  stepCursor(dt) {
    const c = this.cursor
    step(c.a, dt)
    step(c.press, dt)
    const p = c.path
    if (!p) {
      c.vx = c.vy = 0
      return
    }
    const s = clamp((this.time - p.t0) / p.dur)
    const s2 = s * s
    const s3 = s2 * s
    const h00 = 2 * s3 - 3 * s2 + 1
    const h10 = s3 - 2 * s2 + s
    const h01 = -2 * s3 + 3 * s2
    const bulge = Math.sin(Math.PI * s) ** 2
    const x = h00 * p.x0 + h10 * p.mx + h01 * p.x1 + p.bx * bulge
    const y = h00 * p.y0 + h10 * p.my + h01 * p.y1 + p.by * bulge
    c.vx = (x - c.x) / dt
    c.vy = (y - c.y) / dt
    c.x = x
    c.y = y
    if (s >= 1) {
      c.path = null
      c.vx = c.vy = 0
    }
  }

  click() {
    jump(this.cursor.press, 1)
    to(this.cursor.press, 0)
  }

  // ─── Drawing ────────────────────────────────────────────────────────────
  // Window canvases follow the camera's zoom (device pixels x zoom), so text stays sharp in a
  // push. A canvas is only re-rasterised when its scale is 20% off, at most two per frame for
  // zoom alone, the focused window first.
  ensureCanvas(w) {
    const tw = Math.max(40, w.w.t)
    const th = Math.max(40, w.h.t)
    const cap = w.id === this.focus || w.float ? 7e6 : 2.6e6
    const want = Math.max(0.5, Math.min(this.res, Math.sqrt(cap / (tw * th))))
    const sizeChanged = Math.abs(w.cw - tw) > 0.5 || Math.abs(w.ch - th) > 0.5
    const resOff = !w.res || want > w.res * 1.2 || want < w.res / 1.6
    if (!w.dirty && !sizeChanged && w.canvas && !resOff) return
    if (w.flying && w.canvas) return
    if (!w.dirty && !sizeChanged && w.canvas && resOff) {
      if (this.repaints >= 2) return
      this.repaints++
    }
    const res = Math.round(want * 8) / 8
    w.cw = tw
    w.ch = th
    w.res = res
    if (!w.canvas) w.canvas = document.createElement("canvas")
    const pw = Math.ceil(tw * res)
    const ph = Math.ceil(th * res)
    if (w.canvas.width !== pw || w.canvas.height !== ph) {
      w.canvas.width = pw
      w.canvas.height = ph
    }
    const ctx = w.canvas.getContext("2d")
    ctx.setTransform(res, 0, 0, res, 0, 0)
    paintWindow(ctx, tw, th, w, w.id === this.focus)
    w.canvas.version = (w.canvas.version || 0) + 1
    w.dirty = false
  }

  menu() {
    const res = Math.min(4, this.res)
    const key = this.clock + this.frontApp + this.status + this.active + res
    if (this.menuCanvas && key === this.menuKey) return this.menuCanvas
    this.menuKey = key
    const c = this.menuCanvas || (this.menuCanvas = document.createElement("canvas"))
    c.width = Math.ceil(this.SW * res)
    c.height = Math.ceil(MENU_H * res)
    const ctx = c.getContext("2d")
    ctx.setTransform(res, 0, 0, res, 0, 0)
    ctx.clearRect(0, 0, this.SW, MENU_H)
    // a light frosted strip: the blurred desktop shows through underneath
    ctx.fillStyle = "rgba(246,243,238,.58)"
    ctx.fillRect(0, 0, this.SW, MENU_H)
    ctx.fillStyle = "rgba(0,0,0,.12)"
    ctx.fillRect(0, MENU_H - 1 / res, this.SW, 1 / res)
    const ink = "#1f1e1c"
    ctx.fillStyle = ink
    ctx.textBaseline = "middle"
    const mid = MENU_H / 2 + 0.5
    ctx.font = `700 13px ${FONT}`
    ctx.textAlign = "left"
    ctx.fillText(this.frontApp, 16, mid)
    let x = 30 + ctx.measureText(this.frontApp).width
    ctx.font = `400 13px ${FONT}`
    if (this.SW > 760)
      for (const m of ["File", "Edit", "View", "Window", "Help"]) {
        ctx.fillText(m, x, mid)
        x += ctx.measureText(m).width + 19
      }
    ctx.textAlign = "right"
    ctx.font = `500 13px ${FONT}`
    let r = this.SW - 14
    ctx.fillText(this.clock, r, mid)
    r -= ctx.measureText(this.clock).width + 16
    // status items, right to left: battery, wifi, then Spaceman's workspace number
    battery(ctx, r - 24, mid, ink)
    r -= 40
    wifi(ctx, r - 9, mid + 5, ink)
    r -= 30
    if (this.status) {
      const sx = r - 22
      ctx.fillStyle = ink
      ctx.beginPath()
      ctx.roundRect(sx, 5.5, 22, 15, 4)
      ctx.fill()
      ctx.fillStyle = "#f4f1ec"
      ctx.font = `700 11px ${FONT}`
      ctx.textAlign = "center"
      ctx.fillText(String(this.active % 10), sx + 11, mid)
    }
    c.version = (c.version || 0) + 1
    return c
  }

  keyCanvas() {
    if (this.keys.canvas) return this.keys.canvas
    const c = (this.keys.canvas = document.createElement("canvas"))
    const res = Math.min(4, this.res)
    const label = this.keys.label || []
    const kw = 44
    const W = label.length * (kw + 8) + 16
    c.width = Math.ceil(W * res)
    c.height = Math.ceil(60 * res)
    c.lw = W
    c.lh = 60
    const ctx = c.getContext("2d")
    ctx.setTransform(res, 0, 0, res, 0, 0)
    ctx.fillStyle = "rgba(22,22,25,.62)"
    ctx.beginPath()
    ctx.roundRect(0, 0, W, 60, 15)
    ctx.fill()
    label.forEach((k, i) => {
      const x = 8 + i * (kw + 8)
      const g = ctx.createLinearGradient(0, 8, 0, 52)
      g.addColorStop(0, "rgba(255,255,255,.17)")
      g.addColorStop(1, "rgba(255,255,255,.09)")
      ctx.fillStyle = g
      ctx.beginPath()
      ctx.roundRect(x, 8, kw, 44, 9)
      ctx.fill()
      ctx.fillStyle = "rgba(255,255,255,.22)"
      ctx.fillRect(x + 6, 8, kw - 12, 1 / res)
      ctx.fillStyle = "#f5f3ee"
      ctx.font = `500 ${k.length > 1 ? 15 : 21}px ${FONT}`
      ctx.textAlign = "center"
      ctx.textBaseline = "middle"
      ctx.fillText(k, x + kw / 2, 31)
    })
    c.version = 1
    return c
  }

  // T maps logical units to CSS px: px = x * T.k + T.ox. clip is in CSS px; g grades; am scales
  // alpha. part is "base" (wallpaper and windows), "top" (menu bar, bar, keys, cursor) or "all".
  draw(list, T, { clip = null, g = 1, am = 1, part = "all" } = {}) {
    this.repaints = 0
    const P = (x, y, w, h, o) => list.push({ g, ...o, x: x * T.k + T.ox, y: y * T.k + T.oy, w: w * T.k, h: h * T.k, clip, a: (o.a ?? 1) * am, r: (o.r ?? 0) * T.k, ring: o.ring ? o.ring * T.k : 0, shadow: o.shadow ? o.shadow * T.k : 0 })
    this.hair = 1 / Math.max(1, this.res)
    this.wallUV = coverUV(this.wall, this.SW, this.SH, 0.5, 0.42)
    if (part !== "top") this.drawBase(P)
    if (part !== "base") this.drawTop(P)
  }

  // the blurred desktop under a rect, for frosted materials
  frost(P, x, y, w, h, o) {
    const [u0, v0, u1, v1] = this.wallUV
    const U = (px) => u0 + (px / this.SW) * (u1 - u0)
    const V = (py) => v0 + (py / this.SH) * (v1 - v0)
    P(x, y, w, h, { ...o, tex: blurOf(this.wall), uv: [U(x), V(y), U(x + w), V(y + h)] })
  }

  drawBase(P) {
    P(0, 0, this.SW, this.SH, { tex: this.wall, uv: this.wallUV, r: 0 })
    const wins = [...this.wins.values()].sort((a, b) => (a.float ? 1e6 : 0) + a.z - ((b.float ? 1e6 : 0) + b.z))
    const hair = this.hair
    for (const w of wins) {
      if (w.a.x < 0.003) continue
      if (!w.float && w.ws !== this.active && !w.flying && w.off.x === 0) continue
      this.ensureCanvas(w)
      const s = w.s.x
      const ww = w.w.x * s
      const hh = w.h.x * s
      const x = w.x.x + (w.w.x - ww) / 2 + w.off.x
      const y = w.y.x + (w.h.x - hh) / 2
      const r = Math.min(11, ww * 0.1, hh * 0.1)
      const a = w.a.x
      const f = w.fo.x
      const lift = w.float ? 1 : 0
      // two shadows: a wide soft one that grows with focus and height, and a tight contact one
      P(x, y, ww, hh, { shadow: 22 + 16 * f + 26 * lift, col: [0, 0, 0, 0.16 + 0.12 * f + 0.12 * lift], r, a })
      P(x, y, ww, hh, { shadow: 4 + 2 * f, col: [0, 0, 0, 0.26 + 0.06 * f], r, a })
      this.frost(P, x, y, ww, hh, { r, a })
      P(x, y, ww, hh, { tex: w.canvas, r, a })
      // device-pixel hairlines: a dark edge outside, a faint light edge inside
      P(x - hair, y - hair, ww + 2 * hair, hh + 2 * hair, { ring: hair, col: [0, 0, 0, 0.3 + 0.08 * f], r: r + hair, a })
      P(x, y, ww, hh, { ring: hair, col: [1, 1, 1, w.dark ? 0.1 : 0.42], r, a })
    }
  }

  drawTop(P) {
    this.frost(P, 0, 0, this.SW, MENU_H, { r: 0 })
    P(0, 0, this.SW, MENU_H, { tex: this.menu(), r: 0 })
    this.drawBar(P)
    // key cast, beside the bar, so the keys and what they did are one glance
    if (this.keys.a.x > 0.01 && this.keys.label) {
      const c = this.keyCanvas()
      const bg = this.barGeom()
      const k = this.keys.a.x
      const s = 0.94 + 0.06 * k
      const kx = Math.max(EDGE_PAD, bg.x - c.lw - 14)
      const ky = bg.y + bg.h / 2 - c.lh / 2
      const kw = c.lw * s
      const kh = c.lh * s
      const ox = kx + (c.lw - kw) / 2
      const oy = ky + (c.lh - kh) / 2
      P(ox, oy, kw, kh, { shadow: 18, col: [0, 0, 0, 0.3], r: 15 * s, a: k, g: 0 })
      this.frost(P, ox, oy, kw, kh, { r: 15 * s, a: k, g: 0 })
      P(ox, oy, kw, kh, { tex: c, r: 15 * s, a: k, g: 0 })
      P(ox, oy, kw, kh, { ring: this.hair, col: [1, 1, 1, 0.16], r: 15 * s, a: k, g: 0 })
    }
    const cu = this.cursor
    if (cu.a.x > 0.01) {
      const s = 1 - cu.press.x * 0.14
      P(cu.x - 3 * s, cu.y - 2 * s, 26 * s, 34 * s, { tex: cursorCanvas(), r: 0, a: cu.a.x })
    }
  }

  drawBar(P) {
    const b = this.bar
    if (b.shown.x < 0.01 || !b.cells.length) return
    const g = this.barGeom()
    const sc = g.sc
    const a = clamp(b.shown.x * 1.4)
    const R = g.h / 2
    P(g.x, g.y, g.w, g.h, { shadow: 26, col: [0, 0, 0, 0.3], r: R, a })
    P(g.x, g.y, g.w, g.h, { shadow: 5, col: [0, 0, 0, 0.3], r: R, a })
    this.frost(P, g.x, g.y, g.w, g.h, { r: R, a })
    P(g.x, g.y, g.w, g.h, { col: [0.075, 0.075, 0.085, 0.74], r: R, a })
    P(g.x, g.y, g.w, g.h, { ring: this.hair * 1.2, col: [1, 1, 1, 0.16], r: R, a })
    P(g.x - this.hair, g.y - this.hair, g.w + 2 * this.hair, g.h + 2 * this.hair, { ring: this.hair, col: [0, 0, 0, 0.45], r: R + this.hair, a })
    const mh = (BAR.ht + BAR.pad * 2) * sc
    const my = g.y + sc + (BAR.inner * sc - mh) / 2
    const mw = Math.max(0, b.right.x - b.left.x) * sc
    P(g.x + b.left.x * sc, my, mw, mh, { col: [1, 1, 1, 0.1], r: 9 * sc, a })
    P(g.x + b.left.x * sc, my, mw, mh, { ring: 0.5 * sc, col: [1, 1, 1, 0.24], r: 9 * sc, a })
    let x = BAR.edge
    for (const c of b.cells) {
      const s = c.s.x
      const cw = BAR.w * s
      const ch = BAR.ht * s
      const cx = g.x + x * sc
      const cy = g.y + sc + ((BAR.inner - ch) / 2) * sc
      const ca = a * c.o.x
      P(cx, cy, cw * sc, ch * sc, { col: [1, 1, 1, 0.05], r: 5 * s * sc, a: ca })
      P(cx, cy, cw * sc, ch * sc, { ring: 0.5 * sc, col: [1, 1, 1, 0.22], r: 5 * s * sc, a: ca })
      const rs = this.rectsOf(c.n)
      const inset = 3 * s
      const iw = cw - 1 - inset * 2
      const ih = ch - 1 - inset * 2
      if (!rs.size) P(cx + (cw / 2 - 2 * s) * sc, cy + (ch / 2 - 2 * s) * sc, 4 * s * sc, 4 * s * sc, { col: [1, 1, 1, 0.28], r: 2 * s * sc, a: ca })
      for (const [id, r] of rs) {
        const w = this.wins.get(id)
        if (w.flying) continue
        const fx = (r.x - this.area.x) / this.area.w
        const fy = (r.y - this.area.y) / this.area.h
        const fw = (r.w + GAP) / this.area.w
        const fh = (r.h + GAP) / this.area.h
        const tx = cx + (inset + fx * iw + 0.8 * s) * sc
        const ty = cy + (inset + fy * ih + 0.8 * s) * sc
        const tw = (fw * iw - 1.6 * s) * sc
        const th = (fh * ih - 1.6 * s) * sc
        const wa = ca * clamp(w.a.x * 1.5)
        P(tx, ty, tw, th, { col: [1, 1, 1, 0.11], r: 3 * s * sc, a: wa })
        const size = Math.min(18 * s * sc, Math.min(tw, th) - 3 * s * sc)
        if (size > 3) P(tx + tw / 2 - size / 2, ty + th / 2 - size / 2, size, size, { tex: iconCanvas(w.app, 48), r: 0, a: wa })
      }
      x += cw + BAR.gap
    }
  }

  // Positions of everything that moves, for the motion trace.
  sample() {
    const o = { cursor: [+this.cursor.x.toFixed(2), +this.cursor.y.toFixed(2)], bar: [+this.bar.left.x.toFixed(2), +this.bar.right.x.toFixed(2), +this.bar.scale.x.toFixed(4), +this.bar.shown.x.toFixed(3)], wins: {} }
    for (const w of this.wins.values()) {
      const vis = w.float || w.ws === this.active || w.flying || w.off.x !== 0
      if (!vis) continue
      o.wins[w.id] = [+(w.x.x + w.off.x).toFixed(2), +w.y.x.toFixed(2), +(w.w.x * w.s.x).toFixed(2), +(w.h.x * w.s.x).toFixed(2), +w.a.x.toFixed(3)]
    }
    return o
  }
}

// Fitts-like: longer trips take longer, but not proportionally.
const cursorTime = (dist) => clamp(0.26 + 0.1 * Math.log2(1 + dist / 24), 0.3, 0.95)

function wifi(ctx, x, y, ink) {
  ctx.strokeStyle = ink
  ctx.lineWidth = 1.7
  ctx.lineCap = "round"
  for (const r of [4.5, 8.5, 12.5]) {
    ctx.beginPath()
    ctx.arc(x, y, r, Math.PI * 1.25, Math.PI * 1.75)
    ctx.stroke()
  }
  ctx.fillStyle = ink
  ctx.beginPath()
  ctx.arc(x, y, 1.6, 0, Math.PI * 2)
  ctx.fill()
}

function battery(ctx, x, y, ink) {
  ctx.strokeStyle = ink
  ctx.globalAlpha = 0.55
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.roundRect(x + 0.5, y - 5.5, 22, 11, 3)
  ctx.stroke()
  ctx.fillStyle = ink
  ctx.fillRect(x + 23.5, y - 2, 1.6, 4)
  ctx.globalAlpha = 1
  ctx.beginPath()
  ctx.roundRect(x + 2.5, y - 3.5, 15, 7, 1.5)
  ctx.fill()
}

let cursorTex = null
function cursorCanvas() {
  if (cursorTex) return cursorTex
  const c = (cursorTex = document.createElement("canvas"))
  c.width = 104
  c.height = 136
  const ctx = c.getContext("2d")
  ctx.scale(4, 4)
  ctx.shadowColor = "rgba(0,0,0,.35)"
  ctx.shadowBlur = 2
  ctx.shadowOffsetY = 0.8
  ctx.beginPath()
  ctx.moveTo(4, 3)
  ctx.lineTo(4, 26)
  ctx.lineTo(9.5, 20.5)
  ctx.lineTo(13.5, 30)
  ctx.lineTo(17, 28.5)
  ctx.lineTo(13, 19.5)
  ctx.lineTo(20.5, 19.5)
  ctx.closePath()
  ctx.fillStyle = "#fff"
  ctx.fill()
  ctx.shadowColor = "transparent"
  ctx.beginPath()
  ctx.moveTo(5.6, 6.6)
  ctx.lineTo(5.6, 22.2)
  ctx.lineTo(9.9, 18)
  ctx.lineTo(13.9, 27.6)
  ctx.lineTo(15.1, 27.1)
  ctx.lineTo(11, 18.1)
  ctx.lineTo(17, 18.1)
  ctx.closePath()
  ctx.fillStyle = "#111"
  ctx.fill()
  c.version = 1
  return c
}

let wallTex = null
// Until the photograph arrives: a gradient in its colours (dusk over hills).
function fallbackWall() {
  if (wallTex) return wallTex
  const c = (wallTex = document.createElement("canvas"))
  c.width = 320
  c.height = 180
  const ctx = c.getContext("2d")
  const g = ctx.createLinearGradient(0, 0, 0, 180)
  g.addColorStop(0, "#5b6a80")
  g.addColorStop(0.55, "#8a8393")
  g.addColorStop(0.75, "#4b4d5a")
  g.addColorStop(1, "#23252d")
  ctx.fillStyle = g
  ctx.fillRect(0, 0, 320, 180)
  c.version = 1
  return c
}

export { BAR_H }
