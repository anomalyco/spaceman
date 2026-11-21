// Window painters. Each window owns a canvas; paint() draws its chrome and contents at logical size
// (the context is already scaled). Contents are redrawn only when a window is marked dirty.
import { APPS, FONT, MONO, iconCanvas, roundRect } from "./apps.js"
import { plates, coverUV } from "./plates.js"
import { photos as libraryPhotos } from "./library.js"

export const BAR_H = 30
const INK = "#2b2a28"
const SOFT = "#7b776f"
const RULE = "rgba(0,0,0,.08)"
const PAGE = "#f7f5f1"

const hash = (n) => {
  n = (n ^ 61) ^ (n >>> 16)
  n = Math.imul(n, 9) >>> 0
  n ^= n >>> 4
  n = Math.imul(n, 0x27d4eb2d) >>> 0
  return ((n ^ (n >>> 15)) >>> 0) / 4294967296
}

function fit(ctx, text, max) {
  if (ctx.measureText(text).width <= max) return text
  let lo = 0
  let hi = text.length
  while (lo < hi) {
    const m = (lo + hi + 1) >> 1
    if (ctx.measureText(text.slice(0, m) + "…").width <= max) lo = m
    else hi = m - 1
  }
  return text.slice(0, lo) + "…"
}

function text(ctx, s, x, y, { size = 14, weight = 400, color = INK, align = "left", font = FONT, max } = {}) {
  ctx.font = `${weight} ${size}px ${font}`
  ctx.fillStyle = color
  ctx.textAlign = align
  ctx.textBaseline = "alphabetic"
  ctx.fillText(max ? fit(ctx, s, max) : s, x, y)
}

// The title bar is translucent: the frosted desktop under the window shows through it.
function chrome(ctx, w, win, focused) {
  const dark = win.dark
  ctx.clearRect(0, 0, w, BAR_H)
  ctx.fillStyle = win.kind === "sticky" ? "rgba(244,222,124,.97)" : dark ? (focused ? "rgba(40,42,47,.84)" : "rgba(34,36,40,.86)") : focused ? "rgba(240,237,231,.74)" : "rgba(236,233,228,.84)"
  ctx.fillRect(0, 0, w, BAR_H)
  ctx.fillStyle = dark ? "rgba(255,255,255,.07)" : "rgba(0,0,0,.1)"
  ctx.fillRect(0, BAR_H - 0.5, w, 0.5)
  const lights = ["#ec6a5e", "#f4bf4f", "#61c554"]
  lights.forEach((col, i) => {
    const cx = 16 + i * 19
    const cy = BAR_H / 2
    ctx.beginPath()
    ctx.arc(cx, cy, 6, 0, Math.PI * 2)
    if (focused) {
      const g = ctx.createLinearGradient(0, cy - 6, 0, cy + 6)
      g.addColorStop(0, col)
      g.addColorStop(1, shade(col, 0.88))
      ctx.fillStyle = g
    } else ctx.fillStyle = dark ? "#4a4d52" : "#d3cfc8"
    ctx.fill()
    ctx.strokeStyle = focused ? "rgba(0,0,0,.18)" : dark ? "rgba(255,255,255,.06)" : "rgba(0,0,0,.1)"
    ctx.lineWidth = 0.6
    ctx.stroke()
  })
  const ic = 16
  const label = win.title
  ctx.font = `600 13px ${FONT}`
  const tw = Math.min(ctx.measureText(label).width, w - 150)
  const x0 = Math.max(76, w / 2 - (tw + ic + 6) / 2)
  if (w > 170) ctx.drawImage(iconCanvas(win.app, ic * 3), x0, BAR_H / 2 - ic / 2, ic, ic)
  text(ctx, label, x0 + (w > 170 ? ic + 6 : 0), BAR_H / 2 + 4.5, {
    size: 13,
    weight: 600,
    color: dark ? (focused ? "#e2dfd9" : "#9b9892") : focused ? INK : "#8a867e",
    max: w - x0 - 12 - ic,
  })
}

function shade(hex, k) {
  const n = parseInt(hex.slice(1), 16)
  const f = (v) => Math.round(v * k)
  return `rgb(${f(n >> 16)},${f((n >> 8) & 255)},${f(n & 255)})`
}

// Wrap words into lines no wider than max; returns the lines.
function wrap(ctx, s, max) {
  const out = []
  let line = ""
  for (const word of s.split(" ")) {
    const t = line ? line + " " + word : word
    if (ctx.measureText(t).width > max && line) {
      out.push(line)
      line = word
    } else line = t
  }
  if (line) out.push(line)
  return out
}

function lines(ctx, list, x, y, w, { size = 15, gap = 25, check = false } = {}) {
  for (const item of list) {
    const s = typeof item === "string" ? { t: item } : item
    if (s.gap) {
      y += gap * 0.6
      continue
    }
    let tx = x
    if (check && s.nobox) tx += size + 9
    else if (check) {
      ctx.strokeStyle = s.done ? "#5b9a5b" : "#b3aea5"
      ctx.lineWidth = 1.5
      roundRect(ctx, x, y - size + 2, size - 1, size - 1, 3)
      ctx.stroke()
      if (s.done) {
        ctx.strokeStyle = "#4f8f4f"
        ctx.lineWidth = 2
        ctx.beginPath()
        ctx.moveTo(x + 3, y - size / 2 + 1)
        ctx.lineTo(x + size / 2 - 1, y - 2)
        ctx.lineTo(x + size - 3, y - size + 4)
        ctx.stroke()
      }
      tx += size + 9
    }
    text(ctx, s.t, tx, y, { size: s.size || size, weight: s.b ? 600 : 400, color: s.done || s.soft ? SOFT : s.color || INK, max: w - (tx - x), font: s.mono ? MONO : FONT })
    if (s.strike) {
      ctx.font = `400 ${size}px ${FONT}`
      const sw = Math.min(ctx.measureText(s.t).width, w - (tx - x))
      ctx.fillStyle = "#8a857c"
      ctx.fillRect(tx, y - size * 0.32, sw, 1.4)
    }
    y += s.size ? s.size * 1.6 : gap
  }
  return y
}

function paperBody(ctx, w, h, d) {
  ctx.fillStyle = PAGE
  ctx.fillRect(0, BAR_H, w, h - BAR_H)
  let y = BAR_H + 40
  if (d.heading) {
    const hs = d.hsize || 21
    y += hs - 21
    text(ctx, d.heading, 26, y, { size: hs, weight: 650, max: w - 52 })
    y += 14
    if (d.sub) {
      y += 14
      text(ctx, d.sub, 26, y, { size: 13, color: SOFT, max: w - 52 })
    }
    y += 26
  }
  const end = lines(ctx, d.lines || [], 26, y, w - 52, { size: d.size || 15, gap: d.gap || 25, check: d.check })
  if (d.caret) {
    // the insertion point, after the last line
    const last = d.lines?.[d.lines.length - 1]
    const s = typeof last === "string" ? last : last?.t || ""
    ctx.font = `400 ${d.size || 15}px ${FONT}`
    const lx = 26 + Math.min(ctx.measureText(s).width, w - 52) + 1.5
    const ly = end - (d.gap || 25)
    ctx.fillStyle = "#3d6fd0"
    ctx.fillRect(lx, ly - (d.size || 15) + 1, 1.5, (d.size || 15) + 3)
  }
}

// A doorbell camera across the street at night. The lit window is No. 14's.
const DOOR_WIN = [0.4709, 0.4226, 0.495, 0.4658]
export function street(ctx, x, y, w, h, t, lit = 1) {
  const p = plates.door
  if (p) {
    const [u0, v0, u1, v1] = coverUV(p, w, h, 0.55, 0.4)
    ctx.drawImage(p, u0 * p.width, v0 * p.height, (u1 - u0) * p.width, (v1 - v0) * p.height, x, y, w, h)
    if (lit < 1) {
      const X = (fx) => x + ((fx - u0) / (u1 - u0)) * w
      const Y = (fy) => y + ((fy - v0) / (v1 - v0)) * h
      ctx.fillStyle = `rgba(40,38,33,${1 - lit})`
      ctx.fillRect(X(DOOR_WIN[0]), Y(DOOR_WIN[1]), X(DOOR_WIN[2]) - X(DOOR_WIN[0]), Y(DOOR_WIN[3]) - Y(DOOR_WIN[1]))
    }
  } else {
    ctx.fillStyle = "#14181c"
    ctx.fillRect(x, y, w, h)
  }
  const fs = Math.max(9, Math.min(13, w * 0.022))
  ctx.fillStyle = "rgba(0,0,0,.35)"
  ctx.fillRect(x, y, w, fs + 18)
  text(ctx, "FRONT  ·  " + t, x + 12, y + 9 + fs, { size: fs, weight: 500, color: "rgba(235,238,240,.86)", font: MONO })
  ctx.fillStyle = "#d2463b"
  ctx.beginPath()
  ctx.arc(x + w - 16, y + 9 + fs * 0.62, 3.6, 0, Math.PI * 2)
  ctx.fill()
  text(ctx, "LIVE", x + w - 24, y + 9 + fs, { size: fs * 0.92, weight: 600, color: "rgba(235,238,240,.86)", font: MONO, align: "right" })
}

// A translucent sidebar: the frosted desktop shows through (the window canvas stays clear here).
function sidebar(ctx, w, h, sw, dark) {
  ctx.clearRect(0, BAR_H, sw, h - BAR_H)
  ctx.fillStyle = dark ? "rgba(30,32,36,.78)" : "rgba(232,229,223,.7)"
  ctx.fillRect(0, BAR_H, sw, h - BAR_H)
  ctx.fillStyle = dark ? "rgba(255,255,255,.06)" : "rgba(0,0,0,.09)"
  ctx.fillRect(sw - 0.5, BAR_H, 0.5, h - BAR_H)
}

function sideItem(ctx, label, y, sw, { sel, count, icon, focused = true } = {}) {
  if (sel) {
    ctx.fillStyle = focused ? "rgba(0,0,0,.09)" : "rgba(0,0,0,.06)"
    roundRect(ctx, 8, y - 18, sw - 16, 26, 6)
    ctx.fill()
  }
  if (icon) {
    ctx.fillStyle = "#4b7fd0"
    ctx.globalAlpha = 0.85
    roundRect(ctx, 18, y - 11, 12, 12, 3)
    ctx.fill()
    ctx.globalAlpha = 1
  }
  text(ctx, label, icon ? 38 : 20, y, { size: 13, weight: sel ? 600 : 400, max: sw - 70 })
  if (count) text(ctx, count, sw - 16, y, { size: 12, color: SOFT, align: "right" })
}

const ARTICLE = [
  "Every autumn the same question comes up on the street: which kind of guard actually keeps a gutter clear? We put four types on the same stretch of roofline for a year and went up the ladder once a month.",
  "Mesh guards kept out the most leaves but needed brushing after every storm. Foam inserts were the cheapest and the quickest to fit, and the first to clog with seeds.",
  "Brush guards sat in the gutter like a bottle brush. They were easy to lift out and clean, but pine needles wove into them by spring.",
  "Reverse-curve guards worked best on the long run over the porch, where water comes off the roof fastest. They are also the most visible from the street.",
]

const painters = {
  jot(ctx, w, h, d, win, focused) {
    const side = w > 560 ? 180 : 0
    ctx.fillStyle = "#fbf8ef"
    ctx.fillRect(side, BAR_H, w - side, h - BAR_H)
    if (side) {
      sidebar(ctx, w, h, side)
      ;(d.list || ["notes", "return drill", "groceries", "passwords??"]).forEach((n, i) => sideItem(ctx, n, BAR_H + 34 + i * 30, side, { sel: i === 0, focused }))
    }
    let y = BAR_H + 38
    text(ctx, d.heading || "", side + 24, y, { size: 20, weight: 650, max: w - side - 48 })
    y += 14
    if (d.edited) text(ctx, d.edited, side + 24, y + 12, { size: 12, color: SOFT })
    y += d.edited ? 40 : 18
    lines(ctx, d.lines || [], side + 24, y, w - side - 48, { size: 15, gap: 25, check: d.check })
  },
  // A floor plan: the room downstairs, drawn to scale, the way he draws everything.
  plan(ctx, w, h, d, win, focused) {
    ctx.fillStyle = "#eef1f4"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const top = BAR_H + 34
    ctx.save()
    ctx.beginPath()
    ctx.rect(0, top, w, h - top)
    ctx.clip()
    for (let x = 0; x < w; x += 16) {
      ctx.fillStyle = x % 80 === 0 ? "rgba(60,90,130,.12)" : "rgba(60,90,130,.055)"
      ctx.fillRect(x, top, 1, h - top)
    }
    for (let y = top; y < h; y += 16) {
      ctx.fillStyle = (y - top) % 80 === 0 ? "rgba(60,90,130,.12)" : "rgba(60,90,130,.055)"
      ctx.fillRect(0, y, w, 1)
    }
    ctx.restore()
    // toolbar
    ctx.fillStyle = focused ? "#e2e6eb" : "#e7eaee"
    ctx.fillRect(0, BAR_H, w, 34)
    ctx.fillStyle = "rgba(0,0,0,.09)"
    ctx.fillRect(0, top - 1, w, 1)
    let tx = 16
    for (const [i, t] of ["Select", "Wall", "Room", "Label", "Measure"].entries()) {
      ctx.font = `500 12.5px ${FONT}`
      const tw = ctx.measureText(t).width + 18
      if (i === 0) {
        ctx.fillStyle = "rgba(0,0,0,.08)"
        roundRect(ctx, tx - 9, BAR_H + 7, tw, 20, 5)
        ctx.fill()
      }
      text(ctx, t, tx, BAR_H + 21.5, { size: 12.5, weight: 500, color: i === 0 ? INK : SOFT })
      tx += tw + 4
    }
    text(ctx, "Basement  ·  1:50", w - 16, BAR_H + 21.5, { size: 12, color: SOFT, align: "right" })
    text(ctx, "Downstairs", 28, top + 34, { size: 19, weight: 650 })
    text(ctx, d.edited || "Edited Jun 11", 28, top + 54, { size: 12, color: SOFT })
    // the room: 5.6 m by about 4 m, walls, the stairs, the boiler, a shelf
    const rx = 28
    const ry = top + 76
    const rw = w - 56
    const m = rw / 5.6
    const rh = Math.min(h - ry - 30, m * 4.4)
    const ink = "#2f3540"
    ctx.strokeStyle = ink
    ctx.lineWidth = 4.5
    ctx.lineJoin = "miter"
    ctx.beginPath()
    ctx.moveTo(rx + rw - m * 1.3, ry + rh)
    ctx.lineTo(rx, ry + rh)
    ctx.lineTo(rx, ry)
    ctx.lineTo(rx + rw, ry)
    ctx.lineTo(rx + rw, ry + rh)
    ctx.lineTo(rx + rw - m * 0.4, ry + rh)
    ctx.stroke()
    // door swing
    ctx.lineWidth = 1.2
    ctx.strokeStyle = "rgba(47,53,64,.5)"
    ctx.beginPath()
    ctx.arc(rx + rw - m * 0.4, ry + rh, m * 0.9, Math.PI, Math.PI * 1.5)
    ctx.stroke()
    // stairs
    const sx = rx + 3
    const sy = ry + 3
    const sw = m * 0.95
    const sh = m * 2.2
    ctx.strokeStyle = "rgba(47,53,64,.55)"
    ctx.lineWidth = 1
    ctx.strokeRect(sx + 0.5, sy + 0.5, sw, sh)
    for (let i = 1; i < 11; i++) {
      ctx.beginPath()
      ctx.moveTo(sx, sy + (sh * i) / 11 + 0.5)
      ctx.lineTo(sx + sw, sy + (sh * i) / 11 + 0.5)
      ctx.stroke()
    }
    text(ctx, "up", sx + sw / 2, sy + sh + 16, { size: 11, color: SOFT, align: "center" })
    // boiler and shelf
    ctx.strokeStyle = "rgba(47,53,64,.55)"
    ctx.strokeRect(rx + 3.5, ry + rh - m * 0.75 - 3.5, m * 0.7, m * 0.7)
    text(ctx, "boiler", rx + 3 + m * 0.35, ry + rh - m * 0.32, { size: 10.5, color: SOFT, align: "center" })
    ctx.strokeRect(rx + rw - m * 0.45 - 3.5, ry + m * 0.6, m * 0.45, m * 2.1)
    text(ctx, "shelf", rx + rw - m * 0.22 - 3.5, ry + m * 0.6 + m * 1.05 + 4, { size: 10.5, color: SOFT, align: "center" })
    // three places along the far wall, side by side, 0.8 by 2.0 m each
    const pw = m * 0.8
    const ph = m * 2.0
    const py = ry + m * 0.45
    const px0 = rx + m * 1.55
    const gap = m * 0.35
    const places = d.places || []
    for (let i = 0; i < 3; i++) {
      const p = places[i] || {}
      const x = Math.round(px0 + i * (pw + gap)) + 0.5
      const y = Math.round(py) + 0.5
      if (p.full) {
        ctx.fillStyle = "#d3d8df"
        ctx.fillRect(x, y, pw, ph)
        ctx.save()
        ctx.beginPath()
        ctx.rect(x, y, pw, ph)
        ctx.clip()
        ctx.strokeStyle = "rgba(47,53,64,.2)"
        ctx.lineWidth = 1
        for (let k = -ph; k < pw + ph; k += 9) {
          ctx.beginPath()
          ctx.moveTo(x + k, y)
          ctx.lineTo(x + k - ph, y + ph)
          ctx.stroke()
        }
        ctx.restore()
        ctx.strokeStyle = ink
        ctx.lineWidth = 1.6
        ctx.setLineDash([])
      } else {
        ctx.strokeStyle = "rgba(47,53,64,.6)"
        ctx.lineWidth = 1.3
        ctx.setLineDash([6, 5])
      }
      ctx.strokeRect(x, y, pw, ph)
      ctx.setLineDash([])
      ctx.fillStyle = p.full ? "rgba(238,241,244,.9)" : "rgba(238,241,244,0)"
      if (p.full) {
        roundRect(ctx, x + pw / 2 - 24, y + ph / 2 - 26, 48, 46, 6)
        ctx.fill()
      }
      const picked = d.sel && d.sel[0] === i
      text(ctx, p.n || "", x + pw / 2, y + ph / 2 - 2, { size: 17, weight: 650, color: p.full || picked ? INK : "#5d6470", align: "center" })
      text(ctx, p.when || "", x + pw / 2, y + ph / 2 + 14, { size: 11, weight: 500, color: "#4f5662", align: "center" })
    }
    // dimensions on the first place
    ctx.strokeStyle = "rgba(61,111,208,.75)"
    ctx.fillStyle = "rgba(61,111,208,.9)"
    ctx.lineWidth = 1
    const dx = Math.round(px0 - m * 0.28) + 0.5
    ctx.beginPath()
    ctx.moveTo(dx, py)
    ctx.lineTo(dx, py + ph)
    ctx.moveTo(dx - 4, py + 0.5)
    ctx.lineTo(dx + 4, py + 0.5)
    ctx.moveTo(dx - 4, py + ph - 0.5)
    ctx.lineTo(dx + 4, py + ph - 0.5)
    ctx.stroke()
    ctx.save()
    ctx.translate(dx - 6, py + ph / 2)
    ctx.rotate(-Math.PI / 2)
    text(ctx, "2.00 m", 0, 0, { size: 11, weight: 500, color: "rgba(61,111,208,.95)", align: "center" })
    ctx.restore()
    const dy = Math.round(py + ph + m * 0.22) + 0.5
    ctx.beginPath()
    ctx.moveTo(px0, dy)
    ctx.lineTo(px0 + pw, dy)
    ctx.moveTo(px0 + 0.5, dy - 4)
    ctx.lineTo(px0 + 0.5, dy + 4)
    ctx.moveTo(px0 + pw - 0.5, dy - 4)
    ctx.lineTo(px0 + pw - 0.5, dy + 4)
    ctx.stroke()
    text(ctx, "0.80", px0 + pw / 2, dy + 15, { size: 11, weight: 500, color: "rgba(61,111,208,.95)", align: "center" })
    // the selection, with its handles
    if (d.sel) {
      const [a, b] = d.sel
      const x0 = px0 + a * (pw + gap) - 5
      const x1 = px0 + b * (pw + gap) + pw + 5
      ctx.strokeStyle = "#3d7be0"
      ctx.lineWidth = 1.5
      ctx.strokeRect(Math.round(x0) + 0.5, Math.round(py - 5) + 0.5, Math.round(x1 - x0), Math.round(ph + 10))
      ctx.fillStyle = "#fff"
      for (const [hx, hy] of [[x0, py - 5], [x1, py - 5], [x0, py + ph + 5], [x1, py + ph + 5]]) {
        ctx.fillRect(hx - 3.5, hy - 3.5, 7, 7)
        ctx.strokeRect(hx - 3.5, hy - 3.5, 7, 7)
      }
    }
  },
  sticky(ctx, w, h, d) {
    ctx.fillStyle = "#f8e58c"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    text(ctx, d.text, 20, BAR_H + 38, { size: 19, weight: 500, color: "#4a3f12", max: w - 40, font: '"Marker Felt", "Comic Neue", ' + FONT })
    if (d.done) {
      ctx.fillStyle = "#6c5a14"
      ctx.font = `500 19px "Marker Felt", "Comic Neue", ${FONT}`
      ctx.fillRect(20, BAR_H + 31, Math.min(ctx.measureText(d.text).width, w - 40), 1.6)
    }
  },
  paper(ctx, w, h, d) {
    paperBody(ctx, w, h, d)
  },
  mail(ctx, w, h, d, win, focused) {
    const side = w > 760 ? 168 : 0
    const listW = w - side > 500 ? Math.min(320, (w - side) * 0.44) : w - side
    ctx.fillStyle = PAGE
    ctx.fillRect(side, BAR_H, w - side, h - BAR_H)
    if (side) {
      sidebar(ctx, w, h, side)
      text(ctx, "Mailboxes", 18, BAR_H + 26, { size: 11, weight: 600, color: SOFT })
      sideItem(ctx, "Inbox", BAR_H + 52, side, { sel: 1, count: d.count || "4,812", icon: 1, focused })
      sideItem(ctx, "Flagged", BAR_H + 82, side, { icon: 1 })
      sideItem(ctx, "Sent", BAR_H + 112, side, { icon: 1 })
      text(ctx, "Folders", 18, BAR_H + 150, { size: 11, weight: 600, color: SOFT })
      sideItem(ctx, "HOA", BAR_H + 176, side, { icon: 1, count: "212" })
      sideItem(ctx, "Receipts", BAR_H + 206, side, { icon: 1 })
    }
    // rows slide when a new message arrives at the top (d.slide: 0 → 1)
    const rowH = 66
    let y = BAR_H - (1 - (d.slide ?? 1)) * rowH
    ctx.save()
    ctx.beginPath()
    ctx.rect(side, BAR_H, listW, h - BAR_H)
    ctx.clip()
    for (const [i, m] of (d.list || []).entries()) {
      if (y > h) break
      const x0 = side
      ctx.fillStyle = i === d.sel ? (focused ? "#d9e5f7" : "#e6e6e4") : PAGE
      ctx.fillRect(x0, y, listW, rowH)
      if (m.unread) {
        ctx.fillStyle = "#4b7fd0"
        ctx.beginPath()
        ctx.arc(x0 + 13, y + 21, 4, 0, Math.PI * 2)
        ctx.fill()
      }
      text(ctx, m.from, x0 + 26, y + 25, { size: 14, weight: 650, max: listW - 100 })
      text(ctx, m.when || "", x0 + listW - 12, y + 25, { size: 12, color: SOFT, align: "right" })
      text(ctx, (m.clip ? "📎 " : "") + m.subject, x0 + 26, y + 47, { size: 13.5, color: "#55514a", max: listW - 40 })
      ctx.fillStyle = RULE
      ctx.fillRect(x0 + 26, y + rowH - 1, listW - 26, 1)
      y += rowH
    }
    ctx.restore()
    if (side + listW < w - 40) {
      const x0 = side + listW
      ctx.fillStyle = RULE
      ctx.fillRect(x0, BAR_H, 1, h)
      const m = d.open
      if (m) {
        const mw = w - x0 - 56
        text(ctx, m.subject, x0 + 28, BAR_H + 44, { size: 19, weight: 650, max: mw })
        text(ctx, m.from, x0 + 28, BAR_H + 68, { size: 13, color: SOFT, max: mw })
        if (m.when) text(ctx, m.when, w - 24, BAR_H + 68, { size: 12, color: SOFT, align: "right" })
        ctx.fillStyle = RULE
        ctx.fillRect(x0 + 28, BAR_H + 84, mw, 1)
        const end = lines(ctx, m.body || [], x0 + 28, BAR_H + 116, mw, { size: 15, gap: 24 })
        if (m.attach) {
          // the attachment, as a chip with a thumbnail
          const ax = x0 + 28
          const ay = end + 4
          ctx.fillStyle = m.attachSel ? "#d9e5f7" : "#efece6"
          roundRect(ctx, ax, ay, 230, 64, 10)
          ctx.fill()
          ctx.strokeStyle = "rgba(0,0,0,.1)"
          ctx.lineWidth = 1
          roundRect(ctx, ax + 0.5, ay + 0.5, 229, 63, 10)
          ctx.stroke()
          const a = plates.across
          if (a) {
            const [u0, v0, u1, v1] = coverUV(a, 64, 48)
            ctx.save()
            roundRect(ctx, ax + 8, ay + 8, 64, 48, 5)
            ctx.clip()
            ctx.drawImage(a, u0 * a.width, v0 * a.height, (u1 - u0) * a.width, (v1 - v0) * a.height, ax + 8, ay + 8, 64, 48)
            ctx.restore()
          }
          text(ctx, m.attach, ax + 84, ay + 28, { size: 13.5, weight: 600, max: 140 })
          text(ctx, "JPEG image · 2.4 MB", ax + 84, ay + 47, { size: 12, color: SOFT })
        }
      }
    }
  },
  browser(ctx, w, h, d) {
    // tabs live in the translucent title area's continuation
    ctx.clearRect(0, BAR_H, w, 34)
    ctx.fillStyle = "rgba(226,223,217,.8)"
    ctx.fillRect(0, BAR_H, w, 34)
    const n = d.tabs || 1
    // the open tab keeps its width; the rest crush down to their favicons, as a browser does with 38 of them
    const tw = Math.max(9, Math.min(170, (w - 20) / n))
    const aw = Math.max(tw, Math.min(150, (w - 20) * 0.34))
    const rw = n > 1 ? Math.max(9, (w - 20 - aw) / (n - 1)) : 0
    ctx.fillStyle = PAGE
    roundRect(ctx, 10, BAR_H + 5, aw - 2, 29, 7)
    ctx.fill()
    text(ctx, TAB_TITLES[0], 22, BAR_H + 24, { size: 12, color: INK, max: aw - 26 })
    for (let i = 1; i < n; i++) {
      const x = 10 + aw + (i - 1) * rw
      if (x + rw > w - 6) break
      if (i > 1) {
        ctx.fillStyle = "rgba(0,0,0,.1)"
        ctx.fillRect(x - 0.5, BAR_H + 13, 1, 13)
      }
      if (rw >= 60) text(ctx, TAB_TITLES[i % TAB_TITLES.length], x + 12, BAR_H + 24, { size: 12, color: "#6a665f", max: rw - 24 })
      else if (rw >= 11) {
        const f = Math.min(10, rw - 4)
        ctx.fillStyle = FAVICONS[i % FAVICONS.length]
        roundRect(ctx, x + (rw - f) / 2, BAR_H + 19.5 - f / 2, f, f, 2.5)
        ctx.fill()
      }
    }
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H + 34, w, h)
    ctx.fillStyle = "#ebe8e2"
    roundRect(ctx, 14, BAR_H + 43, w - 28, 26, 13)
    ctx.fill()
    text(ctx, d.url || "", 32, BAR_H + 61, { size: 12.5, color: "#5d5953", max: w - 64 })
    const cw = Math.min(w - 52, 620)
    const x0 = (w - cw) / 2
    let y = BAR_H + 108
    text(ctx, d.heading || "", x0, y, { size: 24, weight: 700, max: cw, font: SERIF })
    y += 24
    text(ctx, "By the Home Desk · 6 min read", x0, y, { size: 12.5, color: SOFT })
    y += 30
    ctx.font = `400 15px ${SERIF}`
    for (const para of ARTICLE) {
      for (const ln of wrap(ctx, para, cw)) {
        if (y > h - 8) return
        text(ctx, ln, x0, y, { size: 15, color: "#33312d", font: SERIF })
        y += 23
      }
      y += 12
    }
  },
  chat(ctx, w, h, d) {
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    text(ctx, d.who || "Dana", 20, BAR_H + 30, { size: 14, weight: 650 })
    text(ctx, d.when || "Today 11:52 PM", w - 20, BAR_H + 30, { size: 12, color: SOFT, align: "right" })
    ctx.fillStyle = RULE
    ctx.fillRect(0, BAR_H + 44, w, 1)
    let y = BAR_H + 62
    for (const m of d.msgs || []) {
      ctx.font = `400 15px ${FONT}`
      const bw = Math.min(w * 0.76, ctx.measureText(m.t).width + 28)
      const x = m.me ? w - bw - 18 : 18
      ctx.fillStyle = m.me ? "#7a64c4" : "#e8e4dd"
      roundRect(ctx, x, y, bw, 36, 18)
      ctx.fill()
      text(ctx, m.t, x + 14, y + 23, { size: 15, color: m.me ? "#fff" : INK, max: bw - 24 })
      if (m.note) text(ctx, m.note, m.me ? w - 22 : 22, y + 54, { size: 12, color: SOFT, align: m.me ? "right" : "left" })
      y += m.note ? 70 : 46
    }
  },
  // A screenshot from March: his desktop as it was, every window on top of another.
  shot(ctx, w, h) {
    ctx.fillStyle = "#2a2c30"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const pw = w - 32
    const ph = Math.min(h - BAR_H - 32, pw * 0.5625)
    const x = 16
    const y = BAR_H + (h - BAR_H - ph) / 2
    const wall = plates["wall-s"] || plates.wall
    if (wall) {
      const [u0, v0, u1, v1] = coverUV(wall, pw, ph)
      ctx.drawImage(wall, u0 * wall.width, v0 * wall.height, (u1 - u0) * wall.width, (v1 - v0) * wall.height, x, y, pw, ph)
    } else {
      ctx.fillStyle = "#5b6a80"
      ctx.fillRect(x, y, pw, ph)
    }
    // the same kinds of windows as tonight, painted by the same painters, piled where they were in March
    ctx.drawImage(marchPile(), x, y, pw, ph)
    ctx.strokeStyle = "rgba(255,255,255,.12)"
    ctx.strokeRect(x + 0.5, y + 0.5, pw - 1, ph - 1)
  },
  daybook(ctx, w, h, d) {
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    text(ctx, d.date || "", 22, BAR_H + 34, { size: 17, weight: 650 })
    const top = BAR_H + 52
    const hour = 40
    for (let i = 0; i < 14; i++) {
      const y = top + i * hour
      if (y > h) break
      ctx.fillStyle = RULE
      ctx.fillRect(64, y, w - 64, 1)
      text(ctx, `${(i + 7) % 12 || 12} ${i + 7 < 12 ? "AM" : "PM"}`, 56, y + 5, { size: 11, color: SOFT, align: "right" })
    }
    for (const e of d.events || []) {
      const y = top + (e.at - 7) * hour
      ctx.fillStyle = e.color || "#e9c9c4"
      roundRect(ctx, 72, y + 2, w - 88, e.len * hour - 4, 6)
      ctx.fill()
      ctx.fillStyle = "rgba(0,0,0,.18)"
      ctx.fillRect(72, y + 4, 3, e.len * hour - 8)
      text(ctx, e.t, 84, y + 20, { size: 13.5, weight: 600, color: "#4a2a26", max: w - 110 })
    }
  },
  photos: (...a) => libraryPhotos(...a),
  cam(ctx, w, h, d) {
    street(ctx, 0, BAR_H, w, h - BAR_H, d.time || "11:47:07 PM", d.lit ?? 1)
  },
  // A photo, opened from a message.
  photo(ctx, w, h, d) {
    ctx.fillStyle = "#18191c"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const p = plates[d.plate || "across"]
    if (p) {
      const pw = w
      const ph = h - BAR_H
      const [u0, v0, u1, v1] = coverUV(p, pw, ph, 0.5, 0.42)
      ctx.drawImage(p, u0 * p.width, v0 * p.height, (u1 - u0) * p.width, (v1 - v0) * p.height, 0, BAR_H, pw, ph)
    }
  },
  sheet(ctx, w, h, d) {
    ctx.fillStyle = "#fbfbf9"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const cols = d.cols || []
    const widths = d.widths || cols.map(() => 1 / cols.length)
    const rowH = 30
    let y = BAR_H
    // sharing history sits at the top, like a document's share banner
    if (d.foot?.length) {
      const fh = d.foot.length * 22 + 12
      ctx.fillStyle = "#f3efe2"
      ctx.fillRect(0, y, w, fh)
      d.foot.forEach((f, i) => text(ctx, f, 12, y + 23 + i * 22, { size: 13.5, weight: i === d.foot.length - 1 ? 600 : 400, color: i === d.foot.length - 1 ? INK : SOFT, max: w - 24 }))
      y += fh
    }
    const top = y
    ctx.fillStyle = "#eef0ec"
    ctx.fillRect(0, y, w, rowH)
    let x = 0
    cols.forEach((c, i) => {
      text(ctx, c, x + 12, y + 20, { size: 12.5, weight: 650, color: "#4c5a4c", max: widths[i] * w - 16 })
      x += widths[i] * w
    })
    y += rowH
    for (const row of d.rows || []) {
      if (y > h) break
      x = 0
      row.cells.forEach((c, i) => {
        text(ctx, c, x + 12, y + 20, { size: 14, color: row.strike ? "#9a968e" : INK, max: widths[i] * w - 16 })
        x += widths[i] * w
      })
      ctx.fillStyle = "rgba(0,0,0,.06)"
      ctx.fillRect(0, y + rowH - 1, w, 1)
      y += rowH
    }
    x = 0
    for (let i = 0; i < cols.length - 1; i++) {
      x += widths[i] * w
      ctx.fillStyle = "rgba(0,0,0,.06)"
      ctx.fillRect(x, top, 1, h)
    }
  },
  settings(ctx, w, h, d, win, focused) {
    const side = 160
    ctx.fillStyle = "#f4f2ee"
    ctx.fillRect(side, BAR_H, w - side, h - BAR_H)
    sidebar(ctx, w, h, side)
    ;["General", "Rules", "Workspaces", "Shortcuts", "Activity"].forEach((s, i) => sideItem(ctx, s, BAR_H + 36 + i * 32, side, { sel: s === "Rules", focused }))
    const x0 = side + 24
    text(ctx, "Rules", x0, BAR_H + 38, { size: 18, weight: 650 })
    text(ctx, "Checked in order. The first rule that matches a new window applies.", x0, BAR_H + 60, { size: 12.5, color: SOFT, max: w - x0 - 20 })
    const cols = [["App", 0], ["Window title", 0.27], ["Action", 0.6], ["Workspace", 0.8]]
    const tw = w - x0 - 22
    let y = BAR_H + 90
    cols.forEach(([c, f]) => text(ctx, c, x0 + f * tw, y, { size: 12, weight: 600, color: SOFT }))
    y += 10
    for (const r of d.rules || []) {
      ctx.fillStyle = r.hl ? "#e1ebda" : "#ffffff"
      roundRect(ctx, x0 - 8, y, tw + 8, 34, 7)
      ctx.fill()
      ctx.strokeStyle = "rgba(0,0,0,.07)"
      ctx.lineWidth = 1
      roundRect(ctx, x0 - 7.5, y + 0.5, tw + 7, 33, 7)
      ctx.stroke()
      // an empty field being edited loses its dash: just the focus ring and the insertion point
      const editing = r.cursor === 3 && r.ws == null
      const cells = [r.app, r.title ? `“${r.title}”` : "Any", r.action, editing ? "" : (r.ws ?? "—")]
      if (editing) {
        ctx.strokeStyle = "rgba(61,111,208,.55)"
        ctx.lineWidth = 2.5
        roundRect(ctx, x0 + cols[3][1] * tw - 7, y + 5, 0.2 * tw - 6, 24, 5)
        ctx.stroke()
      }
      cells.forEach((c, i) => {
        if (c == null) return
        const f = cols[i][1]
        if (i === 0 && c) ctx.drawImage(iconCanvas(r.icon || "paper", 54), x0 + f * tw, y + 8, 18, 18)
        text(ctx, String(c), x0 + f * tw + (i === 0 ? 24 : 0), y + 22, { size: 13.5, color: (i === 1 && !r.title) || (i === 3 && r.ws == null) ? SOFT : INK, max: (i === 3 ? 0.2 : cols[i + 1][1] - f) * tw - 26 })
      })
      if (r.cursor != null) {
        ctx.fillStyle = "#3d6fd0"
        const f = cols[r.cursor][1]
        ctx.font = `400 13.5px ${FONT}`
        ctx.fillRect(x0 + f * tw + (r.cursor === 0 ? 24 : 0) + ctx.measureText(String(cells[r.cursor] || "")).width + 1.5, y + 9, 1.5, 17)
      }
      y += 40
    }
    if (d.add) text(ctx, "+  Add Rule", x0, y + 22, { size: 13.5, weight: 600, color: "#3d6fd0" })
  },
}

const SERIF = 'Charter, "Iowan Old Style", Georgia, serif'
const FAVICONS = ["#4f7fbf", "#c4553e", "#6a9a5b", "#d0a03a", "#7d6bb0", "#3f9a96", "#8a8580", "#c97a3a"]
const TAB_TITLES = ["Gutter guards: which kind?", "Hedge height bylaws", "Laser measure reviews", "Pinecrest HOA", "How to take minutes", "Cordless drills compared"]

// His desktop on March 2, at a 1440×810 screen: real windows, each on top of another. Painted once.
let pile = null
let pileWall = false
function marchPile() {
  // painted again once the wallpaper plate has arrived
  if (pile && (pileWall || !(plates["wall-s"] || plates.wall))) return pile
  const W = 1440
  const H = 810
  pile = document.createElement("canvas")
  pile.width = W
  pile.height = H
  const ctx = pile.getContext("2d")
  const wall = plates["wall-s"] || plates.wall
  pileWall = !!wall
  if (wall) {
    const [u0, v0, u1, v1] = coverUV(wall, W, H)
    ctx.drawImage(wall, u0 * wall.width, v0 * wall.height, (u1 - u0) * wall.width, (v1 - v0) * wall.height, 0, 0, W, H)
  } else {
    ctx.fillStyle = "#5b6a80"
    ctx.fillRect(0, 0, W, H)
  }
  const jot = (title, heading, lines, edited) => ({ kind: "jot", app: "jot", title, data: { heading, lines, edited } })
  const wins = [
    [{ kind: "browser", app: "browsy", title: "Hedge height bylaws — Browsy", data: { tabs: 21, url: "pinecrest.example/bylaws/4-12", heading: "Hedge height bylaws" } }, 40, 60, 620, 470],
    [{ kind: "daybook", app: "daybook", title: "Daybook — March", data: { date: "Monday, March 2", events: [{ at: 8, len: 1, t: "Bins out" }] } }, 520, 40, 420, 520],
    [jot("Untitled 44", "Untitled 44", ["hedge — measure?", "ask Mark re: drill"], "Edited February 3"), 300, 230, 380, 330],
    [{ kind: "browser", app: "browsy", title: "Cordless drills compared — Browsy", data: { tabs: 21, url: "homedesk.example/cordless-drills", heading: "Cordless drills compared" } }, 820, 120, 580, 440],
    [jot("notes", "notes", ["call about the gutters", "drill → return", "passwords??"], "Edited March 2"), 120, 380, 360, 330],
    [jot("cameras — No. 16", "About the cameras", ["We'd like the two cameras on No. 12", "that face our houses taken down."], "Signed Feb 26"), 640, 330, 420, 300],
    [jot("Untitled 45", "Untitled 45", ["minutes — June?"], "Edited February 21"), 980, 420, 330, 280],
    [{ kind: "sticky", app: "jot", title: "", data: { text: "organize notes" } }, 430, 500, 300, 260],
  ]
  for (const [win, wx, wy, ww, wh] of wins) {
    const c = document.createElement("canvas")
    c.width = ww
    c.height = wh
    paintWindow(c.getContext("2d"), ww, wh, win, false)
    ctx.save()
    ctx.shadowColor = "rgba(0,0,0,.35)"
    ctx.shadowBlur = 24
    ctx.shadowOffsetY = 8
    ctx.drawImage(c, wx, wy)
    ctx.restore()
  }
  return pile
}

export function paintWindow(ctx, w, h, win, focused) {
  ctx.clearRect(0, 0, w, h)
  ;(painters[win.kind] || painters.paper)(ctx, w, h, win.data || {}, win, focused)
  chrome(ctx, w, win, focused)
}

export { text, hash }
