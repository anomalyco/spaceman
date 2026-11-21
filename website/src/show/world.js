// The man's Mac: workspaces, tiled windows, the bar, the cursor and the menu bar, in logical screen
// units. draw() appends rounded-rect primitives to a display list that both renderers consume.
import { APPS, FONT, iconCanvas } from "./apps.js"
import { paintWindow, BAR_H } from "./content.js"
import { sp, to, jump, step, clamp } from "./springs.js"

export const MENU_H = 26
const GAP = 8
const EDGE_PAD = 10
// The Grow track: { w: 50, ht: 32, small: 0.62, gap: 8, pad: 3 }
const BAR = { w: 50, ht: 32, small: 0.62, gap: 8, pad: 3, edge: 12, inner: 42 }

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
    const cols = Math.max(2, Math.round(Math.sqrt(n * aspect * 0.8)))
    const rows = Math.ceil(n / cols)
    let k = 0
    for (let r = 0; r < rows; r++) {
      const inRow = Math.min(cols, n - k)
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
    this.full = new Set() // workspaces in Fullscreen Layout
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
    // bar
    this.bar = { shown: sp(0, 0.5), scale: sp(1, 0.45), cells: [], left: sp(0, 0.3), right: sp(0, 0.3), count: 0 }
    this.cursor = { x: sp(SW * 0.62, 0.6), y: sp(SH * 0.55, 0.6), a: sp(1, 0.3), press: sp(0, 0.08) }
    this.keys = { label: null, a: sp(0, 0.25), until: 0 }
    this.time = 0
    this.wall = wallpaper(opts.wall || "night")
    this.lastFocus = {}
    this.reduced = !!opts.reduced
  }

  add(def) {
    const w = {
      ws: 1,
      float: false,
      ...def,
      x: sp(def.at?.[0] ?? 100, 0.42),
      y: sp(def.at?.[1] ?? 100, 0.42),
      w: sp(def.at?.[2] ?? 480, 0.42),
      h: sp(def.at?.[3] ?? 340, 0.42),
      a: sp(def.hidden ? 0 : 1, 0.22),
      s: sp(1, 0.3),
      off: sp(0, 0.2),
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
    if (this.full.has(ws)) return new Map(ids.map((id) => [id, { x: this.area.x + GAP / 2, y: this.area.y + GAP / 2, w: this.area.w - GAP, h: this.area.h - GAP }]))
    const rs = tile(ids.length, this.area)
    return new Map(ids.map((id, i) => [id, rs[i]]))
  }

  // Retarget every tiled window on the active workspace to its tile.
  reflow(snap = false) {
    if (!this.tiling) return
    for (const [id, r] of this.rectsOf(this.active)) {
      const w = this.wins.get(id)
      for (const k of ["x", "y", "w", "h"]) (snap ? jump : to)(w[k], r[k])
      if (Math.abs(w.cw - r.w) > 0.5 || Math.abs(w.ch - r.h) > 0.5) w.dirty = true
    }
  }

  tileNow() {
    this.tiling = true
    let i = 0
    for (const [id, r] of this.rectsOf(this.active)) {
      const w = this.wins.get(id)
      // a cascade: each window leaves a beat after the one before it
      w.delay = i++ * 0.035
      w.target = r
    }
  }

  setFocus(id) {
    const prev = this.wins.get(this.focus)
    if (prev) prev.dirty = true
    this.focus = id
    const w = this.wins.get(id)
    if (w) {
      w.z = ++this.z
      w.dirty = true
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

  open(def, { route } = {}) {
    const w = this.add({ ...def, hidden: true })
    if (!w.float) {
      const l = this.listOf(w.ws)
      // opened on the active workspace first, then routed
      if (route != null && w.ws !== this.active) {
        l.splice(l.indexOf(w.id), 1)
        w.ws = this.active
        this.listOf(this.active).push(w.id)
      }
    }
    const rs = this.rectsOf(this.active)
    const r = rs.get(w.id) || { x: def.at[0], y: def.at[1], w: def.at[2], h: def.at[3] }
    for (const k of ["x", "y", "w", "h"]) jump(w[k], r[k])
    jump(w.s, 0.94)
    to(w.s, 1)
    to(w.a, 1)
    this.reflow()
    this.setFocus(w.id)
    return w
  }

  close(id) {
    const w = this.wins.get(id)
    w.leaving = true
    to(w.s, 0.94)
    to(w.a, 0)
    w.removeAt = this.time + 0.3
    this.reflow()
  }

  // Option+Shift+n: the window leaves for workspace n, shrinking into its little layout in the bar.
  move(id, ws) {
    const w = this.wins.get(id)
    const from = this.listOf(w.ws)
    from.splice(from.indexOf(id), 1)
    this.ensureCell(ws)
    w.ws = ws
    this.listOf(ws).push(id)
    const c = this.cellRect(ws)
    if (this.reduced) {
      to(w.a, 0, 0.3)
    } else {
      to(w.x, c.x, 0.5)
      to(w.y, c.y, 0.5)
      to(w.w, c.w, 0.5)
      to(w.h, c.h, 0.5)
      to(w.a, 0, 0.45)
    }
    w.flying = this.time + 0.55
    this.reflow()
  }

  switchTo(ws, { quiet } = {}) {
    if (ws === this.active) return
    const dir = this.reduced ? 0 : Math.sign(ws - this.active)
    this.ensureCell(ws)
    for (const w of this.wins.values()) {
      if (w.float || w.leaving) continue
      if (w.ws === this.active) {
        to(w.off, -dir * 36 || -0.001, 0.16)
        to(w.a, 0, 0.12)
      }
    }
    const prev = this.active
    this.active = ws
    for (const [id, r] of this.rectsOf(ws)) {
      const w = this.wins.get(id)
      for (const k of ["x", "y", "w", "h"]) jump(w[k], r[k])
      jump(w.off, dir * 36)
      to(w.off, 0, 0.24)
      jump(w.a, 0)
      to(w.a, 1, 0.16)
      if (Math.abs(w.cw - r.w) > 0.5 || Math.abs(w.ch - r.h) > 0.5) w.dirty = true
    }
    const top = [...this.rectsOf(ws).keys()].pop()
    if (top) this.setFocus(this.lastFocus?.[ws] ?? top)
    this.barGo(prev, ws)
    if (!quiet) this.keyCast(["⌥", String(ws % 10)])
  }

  keyCast(label, hold = 1.1) {
    this.keys.label = label
    this.keys.until = this.time + hold
    to(this.keys.a, 1)
    this.keys.canvas = null
  }

  // ─── Bar ────────────────────────────────────────────────────────────────
  ensureCell(ws) {
    while (this.bar.cells.length < ws) {
      const n = this.bar.cells.length + 1
      this.bar.cells.push({ n, s: sp(0, 0.35), o: sp(0, 0.35) })
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
          for (const k of ["x", "y", "w", "h"]) to(w[k], w.target[k], 0.46)
          w.dirty = true
          w.target = null
        }
      }
      for (const k of ["x", "y", "w", "h", "a", "s", "off"]) step(w[k], dt)
      if (w.removeAt && this.time >= w.removeAt) {
        this.wins.delete(w.id)
        const l = this.listOf(w.ws)
        if (l.includes(w.id)) l.splice(l.indexOf(w.id), 1)
      }
      if (w.flying && this.time >= w.flying) w.flying = 0
    }
    const b = this.bar
    for (const k of ["shown", "scale", "left", "right"]) step(b[k], dt)
    for (const c of b.cells) {
      step(c.s, dt)
      step(c.o, dt)
    }
    const cu = this.cursor
    for (const k of ["x", "y", "a", "press"]) step(cu[k], dt)
    if (this.keys.label && this.time > this.keys.until) to(this.keys.a, 0)
    step(this.keys.a, dt)
  }

  moveCursor(x, y, d) {
    const dist = Math.hypot(x - this.cursor.x.t, y - this.cursor.y.t)
    const dur = d ?? clamp(0.35 + dist / 2200, 0.35, 0.9)
    to(this.cursor.x, x, dur)
    to(this.cursor.y, y, dur)
  }

  click() {
    jump(this.cursor.press, 1)
    to(this.cursor.press, 0)
  }

  // ─── Drawing ────────────────────────────────────────────────────────────
  ensureCanvas(w) {
    const tw = Math.max(40, w.w.t)
    const th = Math.max(40, w.h.t)
    // the world's resolution, capped so no window canvas passes about 2.2 megapixels
    const res = Math.max(0.5, Math.floor(Math.min(this.res, Math.sqrt(2.2e6 / (tw * th))) * 4) / 4)
    const sizeChanged = Math.abs(w.cw - tw) > 0.5 || Math.abs(w.ch - th) > 0.5 || w.res !== res
    if (!w.dirty && !sizeChanged && w.canvas) return
    if (w.flying && w.canvas) return
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
    const key = this.clock + this.frontApp + this.status + this.active + this.res
    if (this.menuCanvas && key === this.menuKey) return this.menuCanvas
    this.menuKey = key
    const c = this.menuCanvas || (this.menuCanvas = document.createElement("canvas"))
    const res = this.res
    c.width = Math.ceil(this.SW * res)
    c.height = Math.ceil(MENU_H * res)
    const ctx = c.getContext("2d")
    ctx.setTransform(res, 0, 0, res, 0, 0)
    ctx.clearRect(0, 0, this.SW, MENU_H)
    ctx.fillStyle = "rgba(236,233,227,.86)"
    ctx.fillRect(0, 0, this.SW, MENU_H)
    ctx.fillStyle = "#2b2a28"
    ctx.textBaseline = "middle"
    ctx.font = `700 13.5px ${FONT}`
    ctx.textAlign = "left"
    ctx.fillText(this.frontApp, 16, MENU_H / 2 + 1)
    let x = 24 + ctx.measureText(this.frontApp).width
    ctx.font = `400 13.5px ${FONT}`
    if (this.SW > 700)
      for (const m of ["File", "Edit", "View", "Window", "Help"]) {
        ctx.fillText(m, x, MENU_H / 2 + 1)
        x += ctx.measureText(m).width + 18
      }
    ctx.textAlign = "right"
    ctx.fillText(this.clock, this.SW - 14, MENU_H / 2 + 1)
    if (this.status) {
      const cw = ctx.measureText(this.clock).width
      const sx = this.SW - 30 - cw - 22
      ctx.fillStyle = "#2b2a28"
      ctx.beginPath()
      ctx.roundRect(sx, 5, 22, 16, 4)
      ctx.fill()
      ctx.fillStyle = "#ece9e3"
      ctx.font = `700 11px ${FONT}`
      ctx.textAlign = "center"
      ctx.fillText(String(this.active % 10), sx + 11, MENU_H / 2 + 1)
    }
    c.version = (c.version || 0) + 1
    return c
  }

  keyCanvas() {
    if (this.keys.canvas) return this.keys.canvas
    const c = (this.keys.canvas = document.createElement("canvas"))
    const res = this.res
    const label = this.keys.label || []
    const kw = 44
    const W = label.length * (kw + 8) + 16
    c.width = Math.ceil(W * res)
    c.height = Math.ceil(60 * res)
    c.lw = W
    c.lh = 60
    const ctx = c.getContext("2d")
    ctx.setTransform(res, 0, 0, res, 0, 0)
    ctx.fillStyle = "rgba(24,24,27,.82)"
    ctx.beginPath()
    ctx.roundRect(0, 0, W, 60, 14)
    ctx.fill()
    label.forEach((k, i) => {
      const x = 8 + i * (kw + 8)
      ctx.fillStyle = "rgba(255,255,255,.12)"
      ctx.beginPath()
      ctx.roundRect(x, 8, kw, 44, 9)
      ctx.fill()
      ctx.fillStyle = "#f3f1ec"
      ctx.font = `500 ${k.length > 1 ? 15 : 22}px ${FONT}`
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
    const P = (x, y, w, h, o) => list.push({ g, ...o, x: x * T.k + T.ox, y: y * T.k + T.oy, w: w * T.k, h: h * T.k, clip, a: (o.a ?? 1) * am, r: (o.r ?? 0) * T.k, ring: o.ring ? o.ring * T.k : 0, shadow: o.shadow ? o.shadow * T.k : 0 })
    if (part !== "top") this.drawBase(P)
    if (part !== "base") this.drawTop(P)
  }

  drawBase(P) {
    P(0, 0, this.SW, this.SH, { tex: this.wall, r: 0 })
    const wins = [...this.wins.values()].sort((a, b) => (a.float ? 1e6 : 0) + a.z - ((b.float ? 1e6 : 0) + b.z))
    for (const w of wins) {
      if (w.a.x < 0.003) continue
      if (!w.float && w.ws !== this.active && !w.flying && w.off.x === 0) continue
      this.ensureCanvas(w)
      const s = w.s.x
      const ww = w.w.x * s
      const hh = w.h.x * s
      const x = w.x.x + (w.w.x - ww) / 2 + w.off.x
      const y = w.y.x + (w.h.x - hh) / 2
      const r = Math.min(10, ww * 0.12)
      P(x, y, ww, hh, { shadow: w.float ? 34 : 18, col: [0, 0, 0, w.float ? 0.5 : 0.32], r, a: w.a.x })
      P(x, y, ww, hh, { tex: w.canvas, r, a: w.a.x })
      P(x, y, ww, hh, { ring: 0.8, col: [0, 0, 0, 0.22], r, a: w.a.x })
    }
  }

  drawTop(P) {
    P(0, 0, this.SW, MENU_H, { tex: this.menu(), r: 0 })
    this.drawBar(P)
    // key cast
    if (this.keys.a.x > 0.01 && this.keys.label) {
      const c = this.keyCanvas()
      const bg = this.barGeom()
      P(this.SW / 2 - c.lw / 2, bg.y - c.lh - 18, c.lw, c.lh, { tex: c, r: 14, a: this.keys.a.x, g: 0 })
    }
    // cursor
    const cu = this.cursor
    if (cu.a.x > 0.01) {
      const s = 1 - cu.press.x * 0.15
      P(cu.x.x - 3 * s, cu.y.x - 2 * s, 26 * s, 34 * s, { tex: cursorCanvas(), r: 0, a: cu.a.x })
    }
  }

  drawBar(P) {
    const b = this.bar
    if (b.shown.x < 0.01 || !b.cells.length) return
    const g = this.barGeom()
    const sc = g.sc
    const a = clamp(b.shown.x * 1.4)
    P(g.x, g.y, g.w, g.h, { shadow: 16, col: [0, 0, 0, 0.35], r: g.h / 2, a })
    P(g.x, g.y, g.w, g.h, { col: [0.188, 0.188, 0.212, 1], r: g.h / 2, a })
    P(g.x + sc, g.y + sc, g.w - 2 * sc, g.h - 2 * sc, { col: [0.086, 0.086, 0.094, 1], r: g.h / 2 - sc, a })
    const mh = (BAR.ht + BAR.pad * 2) * sc
    const my = g.y + sc + (BAR.inner * sc - mh) / 2
    P(g.x + b.left.x * sc, my, Math.max(0, b.right.x - b.left.x) * sc, mh, { col: [1, 1, 1, 0.1], r: 9 * sc, a })
    P(g.x + b.left.x * sc, my, Math.max(0, b.right.x - b.left.x) * sc, mh, { ring: 0.5 * sc, col: [1, 1, 1, 0.2], r: 9 * sc, a })
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
        P(tx, ty, tw, th, { col: [1, 1, 1, 0.11], r: 3 * s * sc, a: ca })
        const size = Math.min(18 * s * sc, Math.min(tw, th) - 3 * s * sc)
        if (size > 3) P(tx + tw / 2 - size / 2, ty + th / 2 - size / 2, size, size, { tex: iconCanvas(w.app, 48), r: 0, a: ca })
      }
      x += cw + BAR.gap
    }
  }
}

let cursorTex = null
function cursorCanvas() {
  if (cursorTex) return cursorTex
  const c = (cursorTex = document.createElement("canvas"))
  c.width = 52
  c.height = 68
  const ctx = c.getContext("2d")
  ctx.scale(2, 2)
  ctx.beginPath()
  ctx.moveTo(4, 3)
  ctx.lineTo(4, 26)
  ctx.lineTo(9.5, 20.5)
  ctx.lineTo(13.5, 30)
  ctx.lineTo(17, 28.5)
  ctx.lineTo(13, 19.5)
  ctx.lineTo(20.5, 19.5)
  ctx.closePath()
  ctx.fillStyle = "#111"
  ctx.strokeStyle = "#fff"
  ctx.lineWidth = 1.6
  ctx.lineJoin = "round"
  ctx.stroke()
  ctx.fill()
  c.version = 1
  return c
}

const walls = {}
// A quiet gradient desktop. night: his; tidy: the other desktop.
function wallpaper(kind) {
  if (walls[kind]) return walls[kind]
  const c = (walls[kind] = document.createElement("canvas"))
  c.width = 320
  c.height = 200
  const ctx = c.getContext("2d")
  const g = ctx.createLinearGradient(0, 0, 320, 200)
  if (kind === "tidy") {
    g.addColorStop(0, "#2f3a3a")
    g.addColorStop(1, "#1d2324")
  } else {
    g.addColorStop(0, "#4a5868")
    g.addColorStop(0.6, "#38424f")
    g.addColorStop(1, "#262c35")
  }
  ctx.fillStyle = g
  ctx.fillRect(0, 0, 320, 200)
  const r = ctx.createRadialGradient(220, 60, 0, 220, 60, 220)
  r.addColorStop(0, kind === "tidy" ? "rgba(140,170,160,.18)" : "rgba(200,190,170,.16)")
  r.addColorStop(1, "rgba(0,0,0,0)")
  ctx.fillStyle = r
  ctx.fillRect(0, 0, 320, 200)
  c.version = 1
  return c
}
