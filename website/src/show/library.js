// His photo library: two pictures from before, then No. 14's hedge every morning from June 11, from
// the same spot, and from September, more and more often, No. 14's window at night. The pictures are
// phone photos (public/show/photos); every morning reuses one of eight, a hand's width off.
import { FONT, roundRect } from "./apps.js"
import { plates, coverUV } from "./plates.js"
import { BAR_H } from "./content.js"

export const PHOTOS = ["h-jun", "h-jul", "h-rain", "h-rain2", "h-fog", "h-oct", "h-nov", "h-tape", "drill", "sixteen", "night", "night-zoom", "plastic", "label", "night-look"]
// the hedge's top in the two photos that get measured (fraction of the photo's height)
const TOP = { "h-jun": 0.393, "h-tape": 0.41 }
export const MORNINGS = 160
export const PICK = 64 // Aug 14

const hash = (n) => {
  n = (n ^ 61) ^ (n >>> 16)
  n = Math.imul(n, 9) >>> 0
  n ^= n >>> 4
  n = Math.imul(n, 0x27d4eb2d) >>> 0
  return ((n ^ (n >>> 15)) >>> 0) / 4294967296
}
const h01 = (i, k) => hash(i * 131 + k * 7919)

const MONTHS = [["Jun", 30], ["Jul", 31], ["Aug", 31], ["Sep", 30], ["Oct", 31], ["Nov", 30]]
const WD = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"] // Jun 11 was a Thursday
export function dateOf(i) {
  let d = i + 11
  for (const [m, n] of MONTHS) {
    if (d <= n) return [m, d]
    d -= n
  }
  return ["Nov", 30]
}
export const day = (i) => dateOf(i).join(" ")
export const weekday = (i) => WD[((i % 7) + 7) % 7]
export const heightAt = (i) => {
  const quarter = Math.round(8 + Math.min(i, 120) * 0.03 + hash(i) * 0.8) // quarter inches above 6′
  const inch = quarter / 4
  return "6′ " + (inch % 1 === 0 ? inch : inch.toFixed(2).replace(/0$/, "")) + "″"
}

// that morning's weather, as the months go
function morningPlate(i) {
  if (i === PICK) return "h-tape"
  const r = h01(i, 4)
  const pick = (table) => {
    let a = 0
    for (const [p, w] of table) if (r < (a += w)) return p
    return table[0][0]
  }
  if (i < 20) return pick([["h-jun", 0.7], ["h-jul", 0.3]])
  if (i < 51) return pick([["h-jul", 0.5], ["h-jun", 0.3], ["h-rain2", 0.2]])
  if (i < 82) return pick([["h-jun", 0.35], ["h-rain", 0.3], ["h-rain2", 0.2], ["h-jul", 0.15]])
  if (i < 112) return pick([["h-fog", 0.45], ["h-jul", 0.35], ["h-rain2", 0.2]])
  if (i < 143) return pick([["h-oct", 0.55], ["h-fog", 0.25], ["h-rain", 0.2]])
  return pick([["h-nov", 0.6], ["h-oct", 0.25], ["h-fog", 0.15]])
}

// Oldest first. m: the morning it belongs after (it shows once that many mornings exist).
export const LIB = (() => {
  // he goes out at about the same time, not the same minute: earlier in summer, a late run in winter
  const morningTime = (i) => {
    const m = Math.round(1 + h01(i, 12) * 6 + h01(i, 13) * h01(i, 14) * 9 - (i > 40 && i < 110 ? 2 : 0) + (i > 120 ? 3 : 0))
    return "6:" + String(Math.max(0, m)).padStart(2, "0") + " AM"
  }
  const L = [{ plate: "sixteen", date: "Mar 3", time: "4:12 PM", m: -1 }]
  for (let i = 0; i < MORNINGS; i++) {
    L.push({ plate: morningPlate(i), date: day(i), time: morningTime(i), m: i, morning: i })
    if (i === 0) L.push({ plate: "drill", date: "Jun 11", time: "9:40 PM", m: 0 })
    if (i === 154) L.push({ plate: "label", date: "Nov 12", time: "10:18 PM", m: i, dark: 1 })
    if (i === 156) L.push({ plate: "plastic", date: "Nov 14", time: "6:31 PM", m: i, dark: 1 })
    // his light, at night: from September, then most nights
    const p = i < 82 ? 0 : Math.min(0.9, 0.18 + (i - 82) * 0.011)
    if (h01(i, 9) < p) L.push({ plate: i > 131 && h01(i, 10) < 0.6 ? "night-zoom" : "night", date: day(i), time: "11:4" + Math.floor(h01(i, 11) * 10) + " PM", m: i, night: 1 })
  }
  return L
})()
export const shown = (n) => {
  let k = 0
  while (k < LIB.length && LIB[k].m < n) k++
  return k
}
export const indexOf = (morning) => LIB.findIndex((e) => e.morning === morning)

// The grid, for the painter and the director alike.
export function gridOf(w, h, n) {
  const top = BAR_H + 40
  const cols = Math.max(4, Math.round((w - 16) / 132))
  const cw = (w - 16) / cols
  const rowH = cw + 22
  const count = shown(n)
  const rows = Math.ceil(count / cols)
  const view = h - top
  const maxScroll = Math.max(0, rows * rowH + 16 - view)
  // where entry k's tile sits for a given scroll
  const tile = (k, scroll = maxScroll) => ({ x: 8 + (k % cols) * cw + 4, y: top + 8 + Math.floor(k / cols) * rowH - scroll + 4, w: cw - 8, h: cw - 8 })
  // the scroll that puts entry k's row a fraction f of the way down the view
  const scrollTo = (k, f = 0.42) => Math.max(0, Math.min(maxScroll, 8 + Math.floor(k / cols) * rowH + rowH / 2 - view * f))
  return { top, cols, cw, rowH, count, rows, view, maxScroll, tile, scrollTo }
}

// one square thumbnail per photo, made once
const thumbs = new Map()
function thumb(p) {
  let c = thumbs.get(p)
  if (c) return c
  const s = 232
  c = document.createElement("canvas")
  c.width = c.height = s
  const x = c.getContext("2d")
  x.imageSmoothingQuality = "high"
  const [u0, v0, u1, v1] = coverUV(p, 1, 1, 0.5, 0.5)
  x.drawImage(p, u0 * p.width, v0 * p.height, (u1 - u0) * p.width, (v1 - v0) * p.height, 0, 0, s, s)
  thumbs.set(p, c)
  return c
}

// A morning's tile: the photo a hand's width off, a degree off level, in that morning's exposure.
function drawTile(ctx, e, k, r) {
  const p = plates[e.plate]
  if (!p) {
    ctx.fillStyle = e.night || e.dark ? "#26262a" : "#5b6650"
    ctx.fillRect(r.x, r.y, r.w, r.h)
    return
  }
  const t = thumb(p)
  ctx.save()
  ctx.beginPath()
  ctx.rect(r.x, r.y, r.w, r.h)
  ctx.clip()
  const jx = (h01(k, 1) - 0.5) * 0.06
  const jy = (h01(k, 2) - 0.5) * 0.06
  const rot = e.morning != null ? (h01(k, 3) - 0.5) * 0.024 : 0
  const z = 1.1
  ctx.translate(r.x + r.w / 2, r.y + r.h / 2)
  ctx.rotate(rot)
  ctx.drawImage(t, (-0.5 * z + jx) * r.w, (-0.5 * z + jy) * r.h, r.w * z, r.h * z)
  ctx.restore()
  if (e.morning != null) {
    const ex = h01(k, 5) - 0.5
    ctx.fillStyle = ex < 0 ? "rgba(20,24,30," + (-ex * 0.22).toFixed(3) + ")" : "rgba(255,240,220," + (ex * 0.12).toFixed(3) + ")"
    ctx.fillRect(r.x, r.y, r.w, r.h)
  }
}

function label(ctx, s, x, y, size, weight, color, align = "left") {
  ctx.font = weight + " " + size + "px " + FONT
  ctx.fillStyle = color
  ctx.textAlign = align
  ctx.textBaseline = "alphabetic"
  ctx.fillText(s, x, y)
}

// One photo, full size, with the hedge's top measured and the limit under it.
function drawOpen(ctx, e, R, d, a) {
  const p = plates[e.plate]
  ctx.save()
  ctx.beginPath()
  ctx.rect(R.x, R.y, R.w, R.h)
  ctx.clip()
  if (!p) {
    ctx.fillStyle = "#4d5a45"
    ctx.fillRect(R.x, R.y, R.w, R.h)
    ctx.restore()
    return
  }
  const [u0, v0, u1, v1] = coverUV(p, R.w, R.h, 0.5, 0.5)
  ctx.drawImage(p, u0 * p.width, v0 * p.height, (u1 - u0) * p.width, (v1 - v0) * p.height, R.x, R.y, R.w, R.h)
  const top = TOP[e.plate]
  if (top != null && d.measured && a > 0.98) {
    const Y = (fy) => R.y + ((fy - v0) / (v1 - v0)) * R.h
    const X = (fx) => R.x + ((fx - u0) / (u1 - u0)) * R.w
    const yt = Y(top)
    const yl = Y(top + 0.03)
    const fs = Math.max(11, Math.min(17, R.w * 0.022))
    const x0 = X(e.plate === "h-tape" ? 0.2 : 0.3)
    const fsz = Math.max(11, Math.min(17, R.w * 0.022))
    // leave room for the "+2″" bracket on narrow (phone) crops
    const x1 = Math.min(X(e.plate === "h-tape" ? 0.62 : 0.76), R.x + R.w - fsz * 2.4 - 14)
    ctx.fillStyle = "rgba(255,255,255,.94)"
    ctx.fillRect(x0, yt - 0.7, x1 - x0, 1.5)
    label(ctx, d.measure || "6′ 2″", x0 + 4, yt - 6, fs, 600, "rgba(255,255,255,.96)")
    if (d.limit) {
      ctx.strokeStyle = "rgba(255,120,100,.95)"
      ctx.setLineDash([5, 4])
      ctx.lineWidth = 1.5
      ctx.beginPath()
      ctx.moveTo(x0, yl)
      ctx.lineTo(x1, yl)
      ctx.stroke()
      ctx.setLineDash([])
      label(ctx, "6′ 0″ limit", x0 + 4, yl + fs + 4, fs, 600, "rgba(255,150,130,.98)")
      // the two inches over, bracketed at the end of the lines
      const bx = x1 + 6
      ctx.fillStyle = "rgba(255,150,130,.98)"
      ctx.fillRect(bx, yt - 0.7, 1.5, yl - yt + 1.4)
      ctx.fillRect(bx - 4, yt - 0.7, 5.5, 1.5)
      ctx.fillRect(bx - 4, yl - 0.7, 5.5, 1.5)
      label(ctx, "+2″", bx + 6, (yt + yl) / 2 + fs * 0.36, fs, 700, "rgba(255,150,130,.98)")
    }
  }
  ctx.restore()
}

export function photos(ctx, w, h, d) {
  const n = d.count || 1
  const G = gridOf(w, h, n)
  const scroll = d.scroll == null ? G.maxScroll : Math.max(-40, Math.min(G.maxScroll + 40, d.scroll))
  ctx.fillStyle = "#1f1f21"
  ctx.fillRect(0, BAR_H, w, h - BAR_H)
  // the grid, newest at the bottom
  const first = Math.max(0, Math.floor((scroll - 8) / G.rowH) * G.cols)
  const last = Math.min(G.count, Math.ceil((scroll + G.view) / G.rowH + 1) * G.cols)
  ctx.save()
  ctx.beginPath()
  ctx.rect(0, G.top, w, h - G.top)
  ctx.clip()
  for (let k = first; k < last; k++) {
    const r = G.tile(k, scroll)
    if (r.y > h || r.y + G.rowH < G.top) continue
    const e = LIB[k]
    drawTile(ctx, e, k, r)
    if (d.sel === k) {
      ctx.strokeStyle = "#4f8cff"
      ctx.lineWidth = 3
      ctx.strokeRect(r.x - 1.5, r.y - 1.5, r.w + 3, r.h + 3)
    }
    const fs = Math.max(10, Math.min(12, G.cw / 11))
    label(ctx, e.date, r.x + 1, r.y + r.w + fs + 5, fs, 600, e.night || e.dark ? "#a7a29a" : "#d9d5ce")
    label(ctx, e.time, r.x + r.w - 1, r.y + r.w + fs + 5, fs - 1, 400, e.night || e.dark ? "#8b867e" : "#8f8b84", "right")
  }
  // the overlay scroll bar, while it moves
  if (d.bar > 0 && G.maxScroll > 0) {
    const vh = Math.max(30, (G.view / (G.view + G.maxScroll)) * (G.view - 12))
    const y = G.top + 6 + (Math.max(0, Math.min(G.maxScroll, scroll)) / G.maxScroll) * (G.view - 12 - vh)
    ctx.fillStyle = "rgba(255,255,255," + (0.42 * d.bar).toFixed(3) + ")"
    roundRect(ctx, w - 9, y, 5, vh, 2.5)
    ctx.fill()
  }
  ctx.restore()
  // the toolbar: the count
  ctx.fillStyle = "rgba(38,38,40,.96)"
  ctx.fillRect(0, BAR_H, w, 40)
  ctx.fillStyle = "rgba(255,255,255,.07)"
  ctx.fillRect(0, BAR_H + 39, w, 1)
  label(ctx, "Library", 16, BAR_H + 25, 14, 700, "#e9e6e0")
  label(ctx, G.count + " photos", 80, BAR_H + 25, 13, 400, "#9d988f")
  // one photo, opening out of its tile (a = 0: its tile; 1: the whole window)
  const a = d.openT || 0
  if (d.open != null && a > 0) {
    const e = LIB[d.open]
    const t = G.tile(d.open, scroll)
    const F = { x: 0, y: G.top, w, h: h - G.top - 34 }
    const R = { x: t.x + (F.x - t.x) * a, y: t.y + (F.y - t.y) * a, w: t.w + (F.w - t.w) * a, h: t.h + (F.h - t.h) * a }
    ctx.fillStyle = "rgba(18,18,20," + Math.min(1, a * 1.4).toFixed(3) + ")"
    ctx.fillRect(0, G.top, w, h - G.top)
    drawOpen(ctx, e, R, d, a)
    if (a > 0.6) {
      const al = Math.min(1, (a - 0.6) / 0.3)
      ctx.globalAlpha = al
      const i = e.morning ?? 0
      label(ctx, weekday(i) + ", " + e.date + " · " + e.time, 14, h - 12, 13.5, 600, "#d9d5ce")
      if (d.measured) label(ctx, d.measure || "6′ 2″", w - 14, h - 12, 13.5, 700, "#ff8f7a", "right")
      ctx.globalAlpha = 1
    }
  }
}

// The album "Hedge": 4,812 pictures of No. 14. Every morning at first; then every hour, day and night;
// then night after night, a burst every few minutes; and last, his window, with him in it, looking back.
export const ALBUM = 4812
const HOURLY = 92 // where every morning becomes every hour
const NIGHTS = HOURLY + 24 * 38 // where every hour becomes every night
const p2 = (n) => String(n).padStart(2, "0")
const ampm = (h, m) => `${h % 12 || 12}:${p2(m)} ${h < 12 ? "AM" : "PM"}`
export function albumAt(k) {
  if (k >= ALBUM - 1) return { plate: "night-look", date: "Nov 19", time: "11:52 PM", night: 1, last: 1 }
  if (k < HOURLY) return { plate: morningPlate(k), date: day(k), time: ampm(6, Math.round(1 + h01(k, 12) * 7)), morning: k }
  if (k < NIGHTS) {
    const j = k - HOURLY
    const i = HOURLY + Math.floor(j / 24)
    const hr = (j + 7) % 24
    const night = hr >= 20 || hr < 6
    return { plate: night ? (h01(k, 10) < 0.5 ? "night" : "night-zoom") : morningPlate(i), date: day(Math.min(i, 172)), time: ampm(hr, Math.floor(h01(k, 11) * 6)), night, morning: night ? null : i }
  }
  const j = k - NIGHTS
  const per = (ALBUM - 1 - NIGHTS) / 31 // a month of nights, to the last one
  const i = 130 + Math.floor(j / per)
  const m = 23 * 60 + 2 + Math.floor((j % per) * 2.4)
  return { plate: h01(k, 10) < 0.55 ? "night-zoom" : "night", date: day(Math.min(i, 172)), time: ampm(Math.floor(m / 60) % 24, m % 60), night: 1 }
}
export function albumGrid(w, h) {
  const top = BAR_H + 40
  const cols = Math.max(5, Math.round((w - 16) / 112))
  const cw = (w - 16) / cols
  const rowH = cw + 20
  const rows = Math.ceil(ALBUM / cols)
  const view = h - top
  const maxScroll = Math.max(0, rows * rowH + 16 - view)
  const tile = (k, scroll) => ({ x: 8 + (k % cols) * cw + 3, y: top + 8 + Math.floor(k / cols) * rowH - scroll + 3, w: cw - 6, h: cw - 6 })
  const scrollTo = (k, f = 0.42) => Math.max(0, Math.min(maxScroll, 8 + Math.floor(k / cols) * rowH + rowH / 2 - view * f))
  return { top, cols, cw, rowH, rows, view, maxScroll, tile, scrollTo }
}
const MONTH = { Jun: "June", Jul: "July", Aug: "August", Sep: "September", Oct: "October", Nov: "November" }
export function album(ctx, w, h, d) {
  const G = albumGrid(w, h)
  const scroll = Math.max(0, Math.min(G.maxScroll, d.scroll || 0))
  ctx.fillStyle = "#1f1f21"
  ctx.fillRect(0, BAR_H, w, h - BAR_H)
  const first = Math.max(0, Math.floor((scroll - 8) / G.rowH) * G.cols)
  const last = Math.min(ALBUM, Math.ceil((scroll + G.view) / G.rowH + 1) * G.cols)
  ctx.save()
  ctx.beginPath()
  ctx.rect(0, G.top, w, h - G.top)
  ctx.clip()
  for (let k = first; k < last; k++) {
    const r = G.tile(k, scroll)
    if (r.y > h || r.y + G.rowH < G.top) continue
    const e = albumAt(k)
    drawTile(ctx, e, k, r)
    if (d.sel === k) {
      ctx.strokeStyle = "#4f8cff"
      ctx.lineWidth = 3
      ctx.strokeRect(r.x - 1.5, r.y - 1.5, r.w + 3, r.h + 3)
    }
    const fs = Math.max(9.5, Math.min(11.5, G.cw / 10))
    label(ctx, e.date, r.x + 1, r.y + r.w + fs + 4, fs, 600, e.night ? "#a7a29a" : "#d9d5ce")
    label(ctx, e.time, r.x + r.w - 1, r.y + r.w + fs + 4, fs - 1, 400, e.night ? "#8b867e" : "#8f8b84", "right")
  }
  if (d.bar > 0 && G.maxScroll > 0) {
    const vh = 34
    const y = G.top + 6 + (scroll / G.maxScroll) * (G.view - 12 - vh)
    ctx.fillStyle = "rgba(255,255,255," + (0.42 * d.bar).toFixed(3) + ")"
    roundRect(ctx, w - 9, y, 5, vh, 2.5)
    ctx.fill()
  }
  ctx.restore()
  // the toolbar: the album, its count, and the month under the top row
  ctx.fillStyle = "rgba(38,38,40,.96)"
  ctx.fillRect(0, BAR_H, w, 40)
  ctx.fillStyle = "rgba(255,255,255,.07)"
  ctx.fillRect(0, BAR_H + 39, w, 1)
  label(ctx, "Hedge", 16, BAR_H + 25, 14, 700, "#e9e6e0")
  label(ctx, ALBUM.toLocaleString("en-US") + " items", 70, BAR_H + 25, 13, 400, "#9d988f")
  const topK = Math.min(ALBUM - 1, Math.max(0, Math.floor((scroll + 8) / G.rowH) * G.cols))
  const mo = albumAt(topK).date.split(" ")[0]
  label(ctx, (MONTH[mo] || mo) + " 2026", w - 16, BAR_H + 25, 13, 600, "#d9d5ce", "right")
  const a = d.openT || 0
  if (d.open != null && a > 0) {
    const e = albumAt(d.open)
    const t = G.tile(d.open, scroll)
    const F = { x: 0, y: G.top, w, h: h - G.top - 34 }
    const R = { x: t.x + (F.x - t.x) * a, y: t.y + (F.y - t.y) * a, w: t.w + (F.w - t.w) * a, h: t.h + (F.h - t.h) * a }
    ctx.fillStyle = "rgba(18,18,20," + Math.min(1, a * 1.4).toFixed(3) + ")"
    ctx.fillRect(0, G.top, w, h - G.top)
    drawOpen(ctx, e, R, d, a)
    if (a > 0.6) {
      ctx.globalAlpha = Math.min(1, (a - 0.6) / 0.3)
      label(ctx, "Thu, " + e.date + " · " + e.time, 14, h - 12, 13.5, 600, "#d9d5ce")
      label(ctx, "4,812 of 4,812", w - 14, h - 12, 13, 400, "#9d988f", "right")
      ctx.globalAlpha = 1
    }
  }
}
