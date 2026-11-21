// Window painters. Each window owns a canvas; paint() draws its chrome and contents at logical size
// (the context is already scaled). Contents are redrawn only when a window is marked dirty.
import { APPS, FONT, MONO, iconCanvas, roundRect } from "./apps.js"

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

function chrome(ctx, w, win, focused) {
  ctx.fillStyle = win.kind === "sticky" ? "#f3dc7a" : win.dark ? "#26282c" : "#ebe8e2"
  ctx.fillRect(0, 0, w, BAR_H)
  ctx.fillStyle = win.dark ? "rgba(255,255,255,.08)" : "rgba(0,0,0,.09)"
  ctx.fillRect(0, BAR_H - 1, w, 1)
  const lights = focused ? ["#e0675d", "#e2b248", "#62b55a"] : ["#c9c5be", "#c9c5be", "#c9c5be"]
  lights.forEach((c, i) => {
    ctx.fillStyle = win.dark && !focused ? "#4a4d52" : c
    ctx.beginPath()
    ctx.arc(16 + i * 19, BAR_H / 2, 6, 0, Math.PI * 2)
    ctx.fill()
  })
  const ic = 16
  const label = win.title
  ctx.font = `600 13.5px ${FONT}`
  const tw = Math.min(ctx.measureText(label).width, w - 150)
  const x0 = Math.max(76, w / 2 - (tw + ic + 6) / 2)
  if (w > 170) ctx.drawImage(iconCanvas(win.app, ic * 2), x0, BAR_H / 2 - ic / 2, ic, ic)
  text(ctx, label, x0 + (w > 170 ? ic + 6 : 0), BAR_H / 2 + 5, {
    size: 13.5,
    weight: 600,
    color: win.dark ? "#d7d4ce" : focused ? INK : "#6f6b64",
    max: w - x0 - 12 - ic,
  })
}

function lines(ctx, list, x, y, w, { size = 15, gap = 25, check = false } = {}) {
  for (const item of list) {
    const s = typeof item === "string" ? { t: item } : item
    if (s.gap) {
      y += gap * 0.6
      continue
    }
    let tx = x
    if (check) {
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
    text(ctx, d.heading, 26, y, { size: 21, weight: 650, max: w - 52 })
    y += 14
    if (d.sub) {
      y += 14
      text(ctx, d.sub, 26, y, { size: 13, color: SOFT, max: w - 52 })
    }
    y += 26
  }
  lines(ctx, d.lines || [], 26, y, w - 52, { size: d.size || 15, gap: d.gap || 25, check: d.check })
}

// Hedge, as photographed every morning from the same spot. Same framing, different weather.
export function hedgePhoto(ctx, x, y, w, h, i, measured) {
  const r = hash(i * 7 + 3)
  const sky = r < 0.3 ? ["#8e9aa6", "#b9c1c7"] : r < 0.7 ? ["#9fb7cc", "#d7e1e6"] : ["#b7c2c9", "#e3e4df"]
  const g = ctx.createLinearGradient(0, y, 0, y + h)
  g.addColorStop(0, sky[0])
  g.addColorStop(1, sky[1])
  ctx.fillStyle = g
  ctx.fillRect(x, y, w, h)
  ctx.fillStyle = "#6f7d63"
  ctx.fillRect(x, y + h * 0.82, w, h * 0.18)
  const top = y + h * (0.24 + (hash(i * 13) - 0.5) * 0.02)
  ctx.fillStyle = "#2f4a2c"
  ctx.beginPath()
  ctx.moveTo(x, y + h * 0.84)
  ctx.lineTo(x, top + 4)
  for (let k = 0; k <= 12; k++) ctx.lineTo(x + (w * k) / 12, top + (hash(k * 31 + 5) - 0.5) * h * 0.05)
  ctx.lineTo(x + w, y + h * 0.84)
  ctx.fill()
  ctx.fillStyle = "rgba(255,255,255,.07)"
  for (let k = 0; k < 18; k++) ctx.fillRect(x + hash(k * 3 + i) * w, top + hash(k * 5 + 1) * h * 0.5, 2, 2)
  // the measuring pole
  ctx.fillStyle = "#e9e3d4"
  ctx.fillRect(x + w * 0.78, y + h * 0.16, Math.max(1.5, w * 0.02), h * 0.68)
  ctx.fillStyle = "#b33a2e"
  for (let k = 0; k < 7; k++) ctx.fillRect(x + w * 0.78, y + h * (0.16 + k * 0.1), Math.max(1.5, w * 0.02), h * 0.05)
  if (r < 0.3) {
    ctx.strokeStyle = "rgba(255,255,255,.18)"
    ctx.lineWidth = 1
    for (let k = 0; k < 14; k++) {
      const px = x + hash(k * 17 + i) * w
      const py = y + hash(k * 23 + i) * h * 0.8
      ctx.beginPath()
      ctx.moveTo(px, py)
      ctx.lineTo(px - 2, py + 6)
      ctx.stroke()
    }
  }
  if (measured) {
    ctx.fillStyle = "rgba(255,255,255,.85)"
    ctx.fillRect(x + w * 0.4, top - 1, w * 0.36, 1.2)
    ctx.font = `600 ${Math.max(8, w * 0.1)}px ${FONT}`
    ctx.textAlign = "left"
    ctx.fillText(measured, x + w * 0.42, top - 4)
  }
}

const day = (i) => {
  const months = [["Jun", 30], ["Jul", 31], ["Aug", 31], ["Sep", 30], ["Oct", 31], ["Nov", 30]]
  let d = i + 3
  for (const [m, n] of months) {
    if (d <= n) return `${m} ${d}`
    d -= n
  }
  return "Nov 30"
}
export const heightAt = (i) => {
  const quarter = Math.round(8 + Math.min(i, 120) * 0.03 + hash(i) * 0.8) // in quarter inches above 6′
  const inch = quarter / 4
  return `6′ ${inch % 1 === 0 ? inch : inch.toFixed(2).replace(/0$/, "")}″`
}

// A street at night, from a doorbell camera. Across the street, one window is lit.
export function street(ctx, x, y, w, h, t, lit = 1) {
  const g = ctx.createLinearGradient(0, y, 0, y + h)
  g.addColorStop(0, "#0d1116")
  g.addColorStop(1, "#1a1f22")
  ctx.fillStyle = g
  ctx.fillRect(x, y, w, h)
  // houses across the street
  const base = y + h * 0.62
  const houses = [0.02, 0.36, 0.7]
  houses.forEach((hx, k) => {
    const hw = w * 0.28
    const hh = h * 0.3
    const left = x + w * hx
    ctx.fillStyle = k === 1 ? "#1d2328" : "#181d21"
    ctx.fillRect(left, base - hh, hw, hh)
    ctx.beginPath()
    ctx.moveTo(left - hw * 0.05, base - hh)
    ctx.lineTo(left + hw / 2, base - hh - h * 0.12)
    ctx.lineTo(left + hw * 1.05, base - hh)
    ctx.fillStyle = "#14181c"
    ctx.fill()
    for (let j = 0; j < 2; j++) {
      const wx = left + hw * (0.18 + j * 0.44)
      const wy = base - hh * 0.75
      const on = k === 1 && j === 1 && lit
      ctx.fillStyle = on ? `rgba(236,196,120,${0.85 * lit})` : "#101417"
      ctx.fillRect(wx, wy, hw * 0.2, hh * 0.32)
      if (on) {
        const glow = ctx.createRadialGradient(wx + hw * 0.1, wy + hh * 0.16, 0, wx + hw * 0.1, wy + hh * 0.16, hw * 0.5)
        glow.addColorStop(0, `rgba(236,196,120,${0.18 * lit})`)
        glow.addColorStop(1, "rgba(236,196,120,0)")
        ctx.fillStyle = glow
        ctx.fillRect(wx - hw * 0.4, wy - hh * 0.5, hw, hh * 1.2)
      }
    }
    if (k === 1) {
      // No. 14's hedge, along the front
      ctx.fillStyle = "#0e1510"
      ctx.fillRect(left - hw * 0.1, base - hh * 0.32, hw * 1.2, hh * 0.32)
    }
  })
  ctx.fillStyle = "#121619"
  ctx.fillRect(x, base, w, h * 0.12)
  ctx.fillStyle = "#1c2124"
  ctx.fillRect(x, base + h * 0.12, w, h)
  // street lamp pool
  const lamp = ctx.createRadialGradient(x + w * 0.2, base + h * 0.2, 0, x + w * 0.2, base + h * 0.2, w * 0.3)
  lamp.addColorStop(0, "rgba(200,170,110,.12)")
  lamp.addColorStop(1, "rgba(200,170,110,0)")
  ctx.fillStyle = lamp
  ctx.fillRect(x, y, w, h)
  // fisheye vignette
  const v = ctx.createRadialGradient(x + w / 2, y + h / 2, Math.min(w, h) * 0.3, x + w / 2, y + h / 2, Math.max(w, h) * 0.75)
  v.addColorStop(0, "rgba(0,0,0,0)")
  v.addColorStop(1, "rgba(0,0,0,.55)")
  ctx.fillStyle = v
  ctx.fillRect(x, y, w, h)
  const fs = Math.max(9, Math.min(13, w * 0.025))
  text(ctx, "FRONT  ·  " + t, x + 12, y + 12 + fs, { size: fs, weight: 500, color: "rgba(230,235,240,.75)", font: MONO })
  ctx.fillStyle = "#c8443a"
  ctx.beginPath()
  ctx.arc(x + w - 18, y + 12 + fs * 0.6, 4, 0, Math.PI * 2)
  ctx.fill()
}

function silhouette(ctx, x, y, w, h, seed, off) {
  ctx.fillStyle = off ? "#2c2f33" : ["#4b5157", "#585049", "#4d5248", "#504a55"][seed % 4]
  ctx.fillRect(x, y, w, h)
  if (off) return
  const cx = x + w * (0.5 + (hash(seed) - 0.5) * 0.1)
  ctx.fillStyle = "rgba(20,20,22,.78)"
  ctx.beginPath()
  ctx.arc(cx, y + h * 0.44, Math.min(w, h) * 0.17, 0, Math.PI * 2)
  ctx.fill()
  ctx.beginPath()
  ctx.ellipse(cx, y + h * 1.02, Math.min(w, h) * 0.42, h * 0.36, 0, Math.PI, 0)
  ctx.fill()
}

const painters = {
  jot(ctx, w, h, d) {
    ctx.fillStyle = "#fbf8ef"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    let y = BAR_H + 38
    text(ctx, d.heading || "", 24, y, { size: 20, weight: 650, max: w - 48 })
    y += 32
    lines(ctx, d.lines || [], 24, y, w - 48, { size: 15, gap: 25, check: d.check })
  },
  sticky(ctx, w, h, d) {
    ctx.fillStyle = "#f8e58c"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    text(ctx, d.text, 20, BAR_H + 38, { size: 19, weight: 500, color: "#4a3f12", max: w - 40, font: '"Marker Felt", "Comic Neue", ' + FONT })
    if (d.done) {
      ctx.fillStyle = "#6c5a14"
      ctx.font = `500 19px ${FONT}`
      ctx.fillRect(20, BAR_H + 31, Math.min(ctx.measureText(d.text).width, w - 40), 1.6)
    }
  },
  paper(ctx, w, h, d) {
    paperBody(ctx, w, h, d)
  },
  mail(ctx, w, h, d) {
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const listW = w > 520 ? Math.min(330, w * 0.42) : w
    let y = BAR_H
    for (const [i, m] of (d.list || []).entries()) {
      if (y > h) break
      ctx.fillStyle = i === d.sel ? "#dfe8f6" : PAGE
      ctx.fillRect(0, y, listW, 66)
      if (m.unread) {
        ctx.fillStyle = "#4b7fd0"
        ctx.beginPath()
        ctx.arc(13, y + 21, 4, 0, Math.PI * 2)
        ctx.fill()
      }
      text(ctx, m.from, 26, y + 25, { size: 14, weight: 650, max: listW - 90 })
      text(ctx, m.when || "", listW - 12, y + 25, { size: 12, color: SOFT, align: "right" })
      text(ctx, m.subject, 26, y + 47, { size: 13.5, color: "#55514a", max: listW - 40 })
      ctx.fillStyle = RULE
      ctx.fillRect(26, y + 65, listW - 26, 1)
      y += 66
    }
    if (listW < w) {
      ctx.fillStyle = RULE
      ctx.fillRect(listW, BAR_H, 1, h)
      const m = d.open
      if (m) {
        text(ctx, m.subject, listW + 28, BAR_H + 44, { size: 19, weight: 650, max: w - listW - 56 })
        text(ctx, m.from, listW + 28, BAR_H + 70, { size: 13, color: SOFT, max: w - listW - 56 })
        lines(ctx, m.body || [], listW + 28, BAR_H + 112, w - listW - 56, { size: 15, gap: 24 })
      }
    }
  },
  browser(ctx, w, h, d) {
    ctx.fillStyle = "#e3e0da"
    ctx.fillRect(0, BAR_H, w, 34)
    const n = d.tabs || 1
    const tw = Math.max(9, Math.min(160, (w - 20) / n))
    for (let i = 0; i < n && 10 + i * tw < w; i++) {
      ctx.fillStyle = i === 0 ? PAGE : "#d6d2cb"
      roundRect(ctx, 10 + i * tw, BAR_H + 6, tw - 2, 28, 5)
      ctx.fill()
      if (tw > 60) text(ctx, d.tabTitle || "", 18 + i * tw, BAR_H + 25, { size: 12, color: "#5a564f", max: tw - 18 })
    }
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H + 34, w, h)
    ctx.fillStyle = "#ebe8e2"
    roundRect(ctx, 14, BAR_H + 44, w - 28, 26, 13)
    ctx.fill()
    text(ctx, d.url || "", 30, BAR_H + 62, { size: 12.5, color: SOFT, max: w - 60 })
    let y = BAR_H + 104
    text(ctx, d.heading || "", 26, y, { size: 22, weight: 700, max: w - 52 })
    y += 22
    for (let i = 0; i < 9 && y < h; i++) {
      ctx.fillStyle = i % 4 === 0 ? "#dcd7cf" : "#e8e4dd"
      ctx.fillRect(26, y, (w - 52) * (i % 4 === 3 ? 0.6 : 1), i % 4 === 0 ? 70 : 10)
      y += i % 4 === 0 ? 84 : 20
    }
  },
  chat(ctx, w, h, d) {
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    let y = BAR_H + 30
    for (const m of d.msgs || []) {
      ctx.font = `400 15px ${FONT}`
      const bw = Math.min(w * 0.72, ctx.measureText(m.t).width + 28)
      const x = m.me ? w - bw - 18 : 18
      ctx.fillStyle = m.me ? "#7a64c4" : "#e6e2db"
      roundRect(ctx, x, y, bw, 36, 17)
      ctx.fill()
      text(ctx, m.t, x + 14, y + 23, { size: 15, color: m.me ? "#fff" : INK, max: bw - 24 })
      if (m.note) text(ctx, m.note, m.me ? w - 20 : 20, y + 54, { size: 12, color: SOFT, align: m.me ? "right" : "left" })
      y += m.note ? 70 : 48
    }
  },
  shot(ctx, w, h) {
    ctx.fillStyle = "#3a3c40"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const pw = w - 40
    const ph = Math.min(h - BAR_H - 40, pw * 0.62)
    const x = 20
    const y = BAR_H + 20
    const g = ctx.createLinearGradient(0, y, 0, y + ph)
    g.addColorStop(0, "#4c5866")
    g.addColorStop(1, "#6d6a68")
    ctx.fillStyle = g
    ctx.fillRect(x, y, pw, ph)
    for (let i = 0; i < 14; i++) {
      ctx.fillStyle = i % 5 === 3 ? "#f3dc7a" : "#ece9e3"
      const rx = x + hash(i * 3) * pw * 0.7
      const ry = y + hash(i * 5 + 1) * ph * 0.6
      ctx.fillRect(rx, ry, pw * 0.25, ph * 0.3)
      ctx.strokeStyle = "rgba(0,0,0,.25)"
      ctx.strokeRect(rx, ry, pw * 0.25, ph * 0.3)
    }
  },
  daybook(ctx, w, h, d) {
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    text(ctx, d.date || "", 22, BAR_H + 34, { size: 17, weight: 650 })
    const top = BAR_H + 52
    const hour = 40
    for (let i = 0; i < 12; i++) {
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
      text(ctx, e.t, 82, y + 20, { size: 13.5, weight: 600, color: "#4a2a26", max: w - 110 })
    }
  },
  photos(ctx, w, h, d) {
    ctx.fillStyle = "#e9e6e0"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const n = d.count || 0
    const cols = Math.max(2, Math.round(w / 118))
    const cw = (w - 16) / cols
    const ch = cw * 0.86
    const rows = Math.ceil(n / cols)
    const visibleRows = Math.floor((h - BAR_H - 10) / ch)
    const first = Math.max(0, rows - visibleRows) * cols
    for (let i = first; i < n; i++) {
      const k = i - first
      const x = 8 + (k % cols) * cw
      const y = BAR_H + 8 + Math.floor(k / cols) * ch
      hedgePhoto(ctx, x + 3, y + 3, cw - 6, ch - 22, i, d.measured && i >= d.measuredFrom ? heightAt(i) : null)
      text(ctx, day(i), x + 4, y + ch - 6, { size: 11, color: "#6d6961" })
    }
  },
  cam(ctx, w, h, d) {
    street(ctx, 0, BAR_H, w, h - BAR_H, d.time || "11:48:07 PM", d.lit ?? 1)
  },
  call(ctx, w, h, d) {
    ctx.fillStyle = "#1c1d20"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const people = d.people || []
    const cols = w > h * 0.9 ? 2 : 1
    const rows = Math.ceil(people.length / cols)
    const gw = (w - 12 * (cols + 1)) / cols
    const gh = (h - BAR_H - 12 * (rows + 1)) / rows
    people.forEach((p, i) => {
      const x = 12 + (i % cols) * (gw + 12)
      const y = BAR_H + 12 + Math.floor(i / cols) * (gh + 12)
      ctx.save()
      roundRect(ctx, x, y, gw, gh, 10)
      ctx.clip()
      silhouette(ctx, x, y, gw, gh, i, p.off)
      if (p.off) text(ctx, p.name[0], x + gw / 2, y + gh / 2 + 14, { size: 40, weight: 600, color: "#8a8d92", align: "center" })
      ctx.restore()
      ctx.fillStyle = "rgba(0,0,0,.45)"
      roundRect(ctx, x + 10, y + gh - 34, 18 + p.name.length * 8.5, 24, 6)
      ctx.fill()
      text(ctx, p.name, x + 19, y + gh - 17, { size: 13, weight: 600, color: "#f0eee9" })
    })
  },
  sheet(ctx, w, h, d) {
    ctx.fillStyle = "#fbfbf9"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const cols = d.cols || []
    const widths = d.widths || cols.map(() => 1 / cols.length)
    const rowH = 30
    let y = BAR_H
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
        if (row.strike && i === 0) {
          ctx.font = `400 14px ${FONT}`
          ctx.fillStyle = "#8a857c"
          ctx.fillRect(x + 12, y + 15, Math.min(ctx.measureText(c).width, widths[i] * w - 16), 1.4)
        }
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
      ctx.fillRect(x, BAR_H, 1, h)
    }
  },
  // The stranger's photo viewer: the photo area stays transparent (the live scene shows through);
  // a strip of earlier nights runs underneath.
  viewer2(ctx, w, h) {
    ctx.fillStyle = "#2a2b2e"
    ctx.fillRect(0, h - 96, w, 96)
    const n = 7
    const tw = Math.min(110, (w - 24) / n - 8)
    const th = tw * 0.66
    for (let i = 0; i < n; i++) {
      const x = 12 + i * (tw + 8)
      const y = h - 96 + 12
      ctx.fillStyle = "#15181d"
      ctx.fillRect(x, y, tw, th)
      ctx.fillStyle = "#0c0e11"
      ctx.fillRect(x, y, tw, th * 0.18)
      ctx.fillStyle = i === n - 1 ? "rgba(150,170,205,.9)" : "rgba(150,170,205,.75)"
      ctx.fillRect(x + tw * 0.5, y + th * 0.36, tw * 0.13, th * 0.26)
      ctx.fillStyle = "#10140f"
      ctx.fillRect(x, y + th * 0.82, tw, th * 0.18)
      if (i === n - 1) {
        ctx.strokeStyle = "#e9e6df"
        ctx.lineWidth = 2
        ctx.strokeRect(x - 1, y - 1, tw + 2, th + 2)
      }
      text(ctx, `Nov ${14 + i}`, x + tw / 2, y + th + 18, { size: 11.5, color: "#b8b4ac", align: "center" })
    }
  },
  settings(ctx, w, h, d) {
    ctx.fillStyle = "#f2f0ec"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const side = 150
    ctx.fillStyle = "#e7e4de"
    ctx.fillRect(0, BAR_H, side, h - BAR_H)
    ;["General", "Rules", "Workspaces", "Shortcuts", "Activity"].forEach((s, i) => {
      if (s === "Rules") {
        ctx.fillStyle = "#d5d1ca"
        roundRect(ctx, 8, BAR_H + 14 + i * 32, side - 16, 28, 6)
        ctx.fill()
      }
      text(ctx, s, 20, BAR_H + 33 + i * 32, { size: 14, weight: s === "Rules" ? 600 : 400 })
    })
    const x0 = side + 22
    text(ctx, "Rules", x0, BAR_H + 38, { size: 18, weight: 650 })
    text(ctx, "Checked in order. The first rule that matches applies.", x0, BAR_H + 60, { size: 12.5, color: SOFT, max: w - x0 - 20 })
    const cols = [["App", 0], ["Window title", 0.27], ["Action", 0.6], ["Workspace", 0.8]]
    const tw = w - x0 - 20
    let y = BAR_H + 88
    cols.forEach(([c, f]) => text(ctx, c, x0 + f * tw, y, { size: 12, weight: 600, color: SOFT }))
    y += 10
    for (const r of d.rules || []) {
      ctx.fillStyle = r.hl ? "#e3ecdc" : "#fbfaf8"
      roundRect(ctx, x0 - 8, y, tw + 8, 34, 6)
      ctx.fill()
      const cells = [r.app, r.title ? `“${r.title}”` : "Any", r.action, r.ws ?? "—"]
      cells.forEach((c, i) => {
        if (c == null) return
        const f = cols[i][1]
        if (i === 0 && c) ctx.drawImage(iconCanvas(r.icon || "paper", 36), x0 + f * tw, y + 8, 18, 18)
        text(ctx, String(c), x0 + f * tw + (i === 0 ? 24 : 0), y + 22, { size: 13.5, color: i === 1 && !r.title ? SOFT : INK, max: (i === 3 ? 0.2 : cols[i + 1][1] - f) * tw - 26 })
      })
      if (r.cursor) {
        ctx.fillStyle = INK
        const f = cols[r.cursor][1]
        ctx.font = `400 13.5px ${FONT}`
        ctx.fillRect(x0 + f * tw + (r.cursor === 0 ? 24 : 0) + ctx.measureText(String(cells[r.cursor] || "")).width + 2, y + 9, 1.5, 17)
      }
      y += 40
    }
    if (d.add) text(ctx, "+  Add Rule", x0, y + 22, { size: 13.5, weight: 600, color: "#4b7fd0" })
  },
}

export function paintWindow(ctx, w, h, win, focused) {
  ctx.clearRect(0, 0, w, h)
  ;(painters[win.kind] || painters.paper)(ctx, w, h, win.data || {}, win)
  chrome(ctx, w, win, focused)
}

export { text, hash }
