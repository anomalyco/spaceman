// Window painters. Each window owns a canvas; paint() draws its chrome and contents at logical size
// (the context is already scaled). Contents are redrawn only when a window is marked dirty.
import { APPS, FONT, MONO, iconCanvas, roundRect } from "./apps.js"
import { favTile } from "./favicons.js"
import { downloadIndicator, sitePage } from "./sitepage.js"
import { plates, coverUV } from "./plates.js"
import { album as albumPainter } from "./library.js"
import { feed as paintFeed } from "./feeds.js"

export const BAR_H = 30
// macOS label colours: primary, secondary, separators, and a white content background
const INK = "#1d1d1f"
const SOFT = "#76767b"
const RULE = "rgba(0,0,0,.09)"
const PAGE = "#ffffff"
// the system accent (blue) and Notes' yellow
const ACCENT = "#0a84ff"
const SEL = "#2a6fdb"
const NOTES_YELLOW = "#e6b600"

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

// The title bar, Sequoia style: an opaque bar a shade darker than the content when the window is
// key, lighter when it isn't; the three lights 12 pt across with 8 pt between, grey when inactive.
// Apps with a unified toolbar (Safari, Messages, Mail) put their own controls in the same row.
const LIGHTS = ["#ff5f57", "#febc2e", "#28c840"]
// where each kind's title row starts its own content: past the sidebar, if it has one
function chrome(ctx, w, win, focused) {
  const dark = win.dark
  const kind = win.kind
  // Spaceman's own window has no title bar: its sidebar and panel run to the top, under the lights
  const bare = kind === "settings"
  if (!bare) ctx.clearRect(0, 0, w, BAR_H)
  ctx.fillStyle = bare ? "rgba(0,0,0,0)" : kind === "sticky" ? "#f6e08a" : dark ? (focused ? "#2c2c2e" : "#323234") : focused ? "#ebebeb" : "#f5f5f5"
  ctx.fillRect(0, 0, w, BAR_H)
  // a sidebar runs up under the lights, in its own translucent material
  const side = bare ? 0 : sideWidth(win, w)
  if (side) {
    ctx.clearRect(0, 0, side, BAR_H)
    ctx.fillStyle = dark ? "rgba(30,30,32,.8)" : focused ? "rgba(232,232,234,.78)" : "rgba(242,242,244,.86)"
    ctx.fillRect(0, 0, side, BAR_H)
    ctx.fillStyle = dark ? "rgba(255,255,255,.07)" : "rgba(0,0,0,.1)"
    ctx.fillRect(side - 0.5, 0, 0.5, BAR_H)
  }
  if (!bare) {
    ctx.fillStyle = dark ? "rgba(255,255,255,.07)" : "rgba(0,0,0,.11)"
    ctx.fillRect(side, BAR_H - 0.5, w - side, 0.5)
  }
  LIGHTS.forEach((col, i) => {
    const cx = 14 + i * 20
    const cy = BAR_H / 2
    ctx.beginPath()
    ctx.arc(cx, cy, 6, 0, Math.PI * 2)
    ctx.fillStyle = focused ? col : dark ? "#565659" : "#d6d6d6"
    ctx.fill()
    ctx.strokeStyle = focused ? "rgba(0,0,0,.14)" : dark ? "rgba(255,255,255,.06)" : "rgba(0,0,0,.07)"
    ctx.lineWidth = 0.5
    ctx.stroke()
  })
  const bar = TOOLBARS[kind]
  if (bar) return bar(ctx, w, win, focused, side)
  // a document window: its name, centred, with the document's icon beside it
  const doc = kind === "paper" || kind === "plan" || kind === "sheet"
  const ic = doc && w > 200 ? 15 : 0
  const label = win.title || ""
  ctx.font = `600 13px ${FONT}`
  const lo = Math.max(side + 12, 74)
  const tw = Math.min(ctx.measureText(label).width, w - lo - 16 - ic)
  const x0 = Math.max(lo, (side + w) / 2 - (tw + (ic ? ic + 5 : 0)) / 2)
  if (ic) ctx.drawImage(iconCanvas(win.app, ic * 3), x0, BAR_H / 2 - ic / 2, ic, ic)
  text(ctx, label, x0 + (ic ? ic + 5 : 0), BAR_H / 2 + 4.5, {
    size: 13,
    weight: 600,
    color: dark ? (focused ? "#e5e5e7" : "#8e8e93") : focused ? "rgba(0,0,0,.85)" : "rgba(0,0,0,.34)",
    max: w - x0 - 12 - ic,
  })
}

// how wide a window's sidebar is (0: none); the painters and the title bar agree on it
function sideWidth(win, w) {
  switch (win.kind) {
    case "jot":
      return w > 560 ? 190 : 0
    case "mail":
      return w > 760 ? 176 : 0
    case "chat":
      return w > 760 ? 230 : 0
    case "settings":
      return w > 640 ? 200 : 150
    default:
      return 0
  }
}

// small toolbar glyphs, drawn the way SF Symbols are: 1.5 pt strokes, round caps
function glyph(ctx, name, x, y, color, s = 1) {
  ctx.save()
  ctx.translate(x, y)
  ctx.scale(s, s)
  ctx.strokeStyle = color
  ctx.fillStyle = color
  ctx.lineWidth = 1.5
  ctx.lineCap = "round"
  ctx.lineJoin = "round"
  ctx.beginPath()
  if (name === "back" || name === "fwd") {
    const d = name === "back" ? 1 : -1
    ctx.moveTo(2.5 * d, -5.5)
    ctx.lineTo(-2.5 * d, 0)
    ctx.lineTo(2.5 * d, 5.5)
  } else if (name === "plus") {
    ctx.moveTo(-5.5, 0)
    ctx.lineTo(5.5, 0)
    ctx.moveTo(0, -5.5)
    ctx.lineTo(0, 5.5)
  } else if (name === "share") {
    ctx.moveTo(-3, -2.5)
    ctx.lineTo(-5, -2.5)
    ctx.lineTo(-5, 6.5)
    ctx.lineTo(5, 6.5)
    ctx.lineTo(5, -2.5)
    ctx.lineTo(3, -2.5)
    ctx.moveTo(0, 2)
    ctx.lineTo(0, -7)
    ctx.moveTo(-2.8, -4.4)
    ctx.lineTo(0, -7.2)
    ctx.lineTo(2.8, -4.4)
  } else if (name === "tabs") {
    ctx.roundRect(-5.5, -3.5, 9, 9, 2)
    ctx.moveTo(-2.5, -6.5)
    ctx.lineTo(4.5, -6.5)
    ctx.quadraticCurveTo(6.5, -6.5, 6.5, -4.5)
    ctx.lineTo(6.5, 2.5)
  } else if (name === "sidebar") {
    ctx.roundRect(-7, -5.5, 14, 11, 2.5)
    ctx.moveTo(-2.5, -5.5)
    ctx.lineTo(-2.5, 5.5)
  } else if (name === "compose") {
    ctx.moveTo(1, -5.5)
    ctx.lineTo(-4, -5.5)
    ctx.quadraticCurveTo(-6, -5.5, -6, -3.5)
    ctx.lineTo(-6, 4)
    ctx.quadraticCurveTo(-6, 6, -4, 6)
    ctx.lineTo(3.5, 6)
    ctx.quadraticCurveTo(5.5, 6, 5.5, 4)
    ctx.lineTo(5.5, -1)
    ctx.moveTo(-1.5, 1.5)
    ctx.lineTo(5.5, -5.5)
  } else if (name === "info") {
    ctx.arc(0, 0, 6.5, 0, Math.PI * 2)
    ctx.moveTo(0, -0.5)
    ctx.lineTo(0, 3.5)
    ctx.stroke()
    ctx.beginPath()
    ctx.arc(0, -3.2, 0.9, 0, Math.PI * 2)
    ctx.fill()
    ctx.restore()
    return
  } else if (name === "trash") {
    ctx.moveTo(-6, -4)
    ctx.lineTo(6, -4)
    ctx.moveTo(-2, -4)
    ctx.lineTo(-2, -6)
    ctx.lineTo(2, -6)
    ctx.lineTo(2, -4)
    ctx.moveTo(-4.5, -4)
    ctx.lineTo(-3.8, 6)
    ctx.lineTo(3.8, 6)
    ctx.lineTo(4.5, -4)
  } else if (name === "reply") {
    ctx.moveTo(-1, -5)
    ctx.lineTo(-6, 0)
    ctx.lineTo(-1, 5)
    ctx.moveTo(-6, 0)
    ctx.lineTo(2, 0)
    ctx.quadraticCurveTo(6, 0, 6, 5)
  } else if (name === "lock") {
    ctx.lineWidth = 1.2
    ctx.roundRect(-3.5, -0.5, 7, 5.5, 1.2)
    ctx.moveTo(-2, -0.5)
    ctx.lineTo(-2, -2.5)
    ctx.arc(0, -2.5, 2, Math.PI, 0)
    ctx.lineTo(2, -0.5)
  } else if (name === "search") {
    ctx.arc(-1, -1, 4.2, 0, Math.PI * 2)
    ctx.moveTo(2.2, 2.2)
    ctx.lineTo(5.5, 5.5)
  }
  ctx.stroke()
  ctx.restore()
}

const tool = (focused, dark) => (dark ? (focused ? "rgba(255,255,255,.72)" : "rgba(255,255,255,.3)") : focused ? "rgba(0,0,0,.6)" : "rgba(0,0,0,.25)")

function host(url) {
  return (url || "").replace(/^https?:\/\//, "").split("/")[0]
}

const TOOLBARS = {
  // Safari: sidebar and the arrows on the left, the address centred, share, new tab and tabs on the right
  browser(ctx, w, win, focused) {
    const d = win.data || {}
    const c = tool(focused)
    const cy = BAR_H / 2
    let x = 84
    if (w > 330) {
      glyph(ctx, "sidebar", x, cy, c, 0.9)
      x += 30
    }
    glyph(ctx, "back", x, cy, c, 0.9)
    glyph(ctx, "fwd", x + 22, cy, d.fwd ? c : tool(false), 0.9)
    const right = w > 380 ? 92 : 12
    if (d.dl != null && w > 380) downloadIndicator(ctx, w - 100, cy, d.dl, c)
    if (w > 380) {
      glyph(ctx, "share", w - 72, cy, c, 0.85)
      glyph(ctx, "plus", w - 46, cy, c, 0.9)
      glyph(ctx, "tabs", w - 20, cy, c, 0.85)
    }
    const lo = x + 40
    const pw = Math.max(60, Math.min(w * 0.46, 520, w - right - lo - 8))
    const px = Math.max(lo, w / 2 - pw / 2)
    // typing in the address field: white, a focus ring, the old address selected until the first key
    if (d.edit != null) {
      ctx.fillStyle = "#fff"
      roundRect(ctx, px, 4.5, pw, BAR_H - 9, 7)
      ctx.fill()
      ctx.strokeStyle = "rgba(10,132,255,.55)"
      ctx.lineWidth = 2.5
      roundRect(ctx, px - 1, 3.5, pw + 2, BAR_H - 7, 8)
      ctx.stroke()
      const s = d.sel ? "https://" + (d.url || "") : d.edit
      ctx.font = `400 12.5px ${FONT}`
      const tw = Math.min(ctx.measureText(s).width, pw - 24)
      if (d.sel && s) {
        ctx.fillStyle = "#b3d7ff"
        ctx.fillRect(px + 10, cy - 7.5, tw + 1, 15)
      }
      text(ctx, s, px + 10, cy + 4.5, { size: 12.5, color: INK, max: pw - 24 })
      if (!d.sel) {
        ctx.fillStyle = "#0a84ff"
        ctx.fillRect(px + 10 + tw + 1, cy - 7, 1.25, 14)
      }
      return
    }
    ctx.fillStyle = focused ? "rgba(0,0,0,.065)" : "rgba(0,0,0,.045)"
    roundRect(ctx, px, 4.5, pw, BAR_H - 9, 7)
    ctx.fill()
    // a page loading: the blue bar along the foot of the field
    if (d.load != null) {
      ctx.save()
      roundRect(ctx, px, 4.5, pw, BAR_H - 9, 7)
      ctx.clip()
      ctx.fillStyle = "#0a84ff"
      ctx.fillRect(px, BAR_H - 6.5, pw * d.load, 2)
      ctx.restore()
    }
    // a page he typed the address of shows it in full
    const site = d.full ? (d.url || "").replace(/^https?:\/\//, "") : host(d.url)
    ctx.font = `400 12.5px ${FONT}`
    const sw = Math.min(ctx.measureText(site || "Search or enter website name").width, pw - 34)
    const tx = px + pw / 2 - sw / 2 + (site ? 6 : 0)
    if (site) glyph(ctx, "lock", tx - 11, cy, focused ? "rgba(0,0,0,.5)" : "rgba(0,0,0,.25)")
    text(ctx, site || "Search or enter website name", tx, cy + 4.5, { size: 12.5, color: site ? (focused ? INK : "rgba(0,0,0,.4)") : "rgba(0,0,0,.36)", max: pw - 34 })
  },
  // Messages: the conversation's name centred over the conversation, the compose and info buttons
  chat(ctx, w, win, focused, side) {
    const d = win.data || {}
    const c = tool(focused)
    if (side) glyph(ctx, "compose", side - 22, BAR_H / 2, c, 0.85)
    if (w - side > 260) glyph(ctx, "info", w - 22, BAR_H / 2, c, 0.85)
    const name = d.who || win.title || ""
    ctx.font = `600 13px ${FONT}`
    const lo = Math.max(side + 14, 74)
    const tw = Math.min(ctx.measureText(name).width, w - lo - 44)
    text(ctx, name, Math.max(lo, (side + w) / 2 - tw / 2), BAR_H / 2 + 4.5, { size: 13, weight: 600, color: focused ? "rgba(0,0,0,.85)" : "rgba(0,0,0,.34)", max: w - lo - 44 })
  },
  // Mail: the mailbox and its count over the list, the message actions over the message
  mail(ctx, w, win, focused, side) {
    const d = win.data || {}
    const c = tool(focused)
    const x = Math.max(side + 14, 74)
    text(ctx, "Inbox", x, BAR_H / 2 + 4.5, { size: 13, weight: 700, color: focused ? "rgba(0,0,0,.85)" : "rgba(0,0,0,.34)" })
    ctx.font = `700 13px ${FONT}`
    const iw = ctx.measureText("Inbox").width
    text(ctx, (d.count || "1,206") + " messages", x + iw + 8, BAR_H / 2 + 4.5, { size: 12, color: focused ? SOFT : "rgba(0,0,0,.28)", max: w - x - iw - 150 })
    if (w > 520) {
      glyph(ctx, "compose", w - 112, BAR_H / 2, c, 0.85)
      glyph(ctx, "reply", w - 80, BAR_H / 2, c, 0.85)
      glyph(ctx, "trash", w - 48, BAR_H / 2, c, 0.85)
      glyph(ctx, "search", w - 20, BAR_H / 2, c, 0.85)
    }
  },
  // Notes: the compose button over the list; the note itself carries its own title
  jot(ctx, w, win, focused, side) {
    const c = tool(focused)
    if (side) glyph(ctx, "compose", side - 22, BAR_H / 2, c, 0.85)
    if (w - side > 300) glyph(ctx, "search", w - 20, BAR_H / 2, c, 0.85)
  },
  // Calendar: Day / Week / Month, and the arrows
  daybook(ctx, w, win, focused) {
    const d = win.data || {}
    if (w < 360) return
    const mode = d.week ? 1 : d.month ? 2 : 0
    const labels = ["Day", "Week", "Month"]
    const sw = 50
    const x0 = w / 2 - (sw * 3) / 2
    ctx.fillStyle = "rgba(0,0,0,.06)"
    roundRect(ctx, x0, 5, sw * 3, BAR_H - 10, 6)
    ctx.fill()
    ctx.fillStyle = "#fff"
    roundRect(ctx, x0 + mode * sw + 1.5, 6.5, sw - 3, BAR_H - 13, 5)
    ctx.fill()
    labels.forEach((s, i) => text(ctx, s, x0 + i * sw + sw / 2, BAR_H / 2 + 4, { size: 11.5, weight: i === mode ? 600 : 500, color: focused ? INK : "rgba(0,0,0,.34)", align: "center" }))
    const c = tool(focused)
    glyph(ctx, "back", w - 70, BAR_H / 2, c, 0.8)
    text(ctx, "Today", w - 42, BAR_H / 2 + 4, { size: 11.5, weight: 500, color: c, align: "center" })
    glyph(ctx, "fwd", w - 14, BAR_H / 2, c, 0.8)
  },
  settings() {},
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
    // a line can carry its own box (one to-do in an otherwise plain note)
    if (check && s.nobox) tx += size + 9
    else if (check || s.box) {
      // Notes' checklist: an open circle, or a filled yellow one with a white tick
      const r = size * 0.5
      const cx = x + r
      const cy = y - size * 0.36
      ctx.beginPath()
      ctx.arc(cx, cy, r - 0.5, 0, Math.PI * 2)
      if (s.done) {
        ctx.fillStyle = NOTES_YELLOW
        ctx.fill()
        ctx.strokeStyle = "#fff"
        ctx.lineWidth = 1.7
        ctx.lineCap = "round"
        ctx.lineJoin = "round"
        ctx.beginPath()
        ctx.moveTo(cx - r * 0.42, cy + 0.2)
        ctx.lineTo(cx - r * 0.1, cy + r * 0.34)
        ctx.lineTo(cx + r * 0.45, cy - r * 0.32)
        ctx.stroke()
      } else {
        ctx.strokeStyle = "rgba(0,0,0,.28)"
        ctx.lineWidth = 1.2
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

// A translucent sidebar: the frosted desktop shows through (the window canvas stays clear here).
function sidebar(ctx, w, h, sw, dark, focused = true) {
  ctx.clearRect(0, BAR_H, sw, h - BAR_H)
  ctx.fillStyle = dark ? "rgba(30,30,32,.8)" : focused ? "rgba(232,232,234,.78)" : "rgba(242,242,244,.86)"
  ctx.fillRect(0, BAR_H, sw, h - BAR_H)
  ctx.fillStyle = dark ? "rgba(255,255,255,.06)" : "rgba(0,0,0,.09)"
  ctx.fillRect(sw - 0.5, BAR_H, 0.5, h - BAR_H)
}

function sideItem(ctx, label, y, sw, { sel, count, icon, focused = true } = {}) {
  if (sel) {
    ctx.fillStyle = focused ? "rgba(0,0,0,.1)" : "rgba(0,0,0,.06)"
    roundRect(ctx, 10, y - 17, sw - 20, 24, 6)
    ctx.fill()
  }
  if (icon) sideGlyph(ctx, icon === 1 ? "folder" : icon, 26, y - 4.5, focused ? ACCENT : "rgba(0,0,0,.3)")
  text(ctx, label, icon ? 40 : 20, y, { size: 13, weight: 400, max: sw - 76 })
  if (count) text(ctx, count, sw - 18, y, { size: 12, color: SOFT, align: "right" })
}

// Mail's mailbox symbols
function sideGlyph(ctx, name, x, y, color) {
  ctx.save()
  ctx.translate(x, y)
  ctx.strokeStyle = color
  ctx.fillStyle = color
  ctx.lineWidth = 1.4
  ctx.lineJoin = "round"
  ctx.lineCap = "round"
  ctx.beginPath()
  if (name === "tray") {
    ctx.roundRect(-7, -5.5, 14, 11, 2.5)
    ctx.moveTo(-7, 1)
    ctx.lineTo(-3, 1)
    ctx.lineTo(-2, 3)
    ctx.lineTo(2, 3)
    ctx.lineTo(3, 1)
    ctx.lineTo(7, 1)
  } else if (name === "flag") {
    ctx.moveTo(-5, 7)
    ctx.lineTo(-5, -6)
    ctx.lineTo(5, -6)
    ctx.lineTo(2.5, -2.5)
    ctx.lineTo(5, 1)
    ctx.lineTo(-5, 1)
  } else if (name === "send") {
    ctx.moveTo(-7, -1)
    ctx.lineTo(7, -6)
    ctx.lineTo(2, 7)
    ctx.lineTo(0, 1)
    ctx.closePath()
    ctx.moveTo(0, 1)
    ctx.lineTo(7, -6)
  } else {
    ctx.moveTo(-7, -4)
    ctx.lineTo(-7, 5.5)
    ctx.lineTo(7, 5.5)
    ctx.lineTo(7, -2.5)
    ctx.lineTo(-0.5, -2.5)
    ctx.lineTo(-2.5, -5)
    ctx.lineTo(-7, -5)
    ctx.closePath()
  }
  ctx.stroke()
  ctx.restore()
}

const ARTICLE = [
  "Every autumn the same question comes up on the street: which kind of guard actually keeps a gutter clear? We put four types on the same stretch of roofline for a year and went up the ladder once a month.",
  "Mesh guards kept out the most leaves but needed brushing after every storm. Foam inserts were the cheapest and the quickest to fit, and the first to clog with seeds.",
  "Brush guards sat in the gutter like a bottle brush. They were easy to lift out and clean, but pine needles wove into them by spring.",
  "Reverse-curve guards worked best on the long run over the porch, where water comes off the roof fastest. They are also the most visible from the street.",
]

// A week, his neighbour's: seven columns, the hours that matter, what they do and when.
function weekView(ctx, w, h, d) {
  text(ctx, d.title || "", 18, BAR_H + 30, { size: 15, weight: 650, max: w - 36 })
  const left = 40
  const top = BAR_H + 62
  const cw = (w - left - 8) / 7
  const from = d.from ?? 6
  const to = d.to ?? 24
  const hour = (h - top - 8) / (to - from)
  d.week.forEach((s, i) => text(ctx, s, left + i * cw + cw / 2, top - 10, { size: 11, weight: 600, color: SOFT, align: "center" }))
  for (let hr = from; hr <= to; hr += 3) {
    const y = top + (hr - from) * hour
    ctx.fillStyle = RULE
    ctx.fillRect(left, y, w - left - 8, 1)
    text(ctx, String(hr % 24).padStart(2, "0"), left - 6, y + 4, { size: 10, color: SOFT, align: "right" })
  }
  for (let i = 1; i < 7; i++) {
    ctx.fillStyle = RULE
    ctx.fillRect(left + i * cw, top, 1, h - top)
  }
  for (const e of d.events || []) {
    const y = top + (e.at - from) * hour
    const hh = Math.max(14, e.len * hour - 2)
    const col = e.color || "#e9c9c4"
    ctx.fillStyle = col
    roundRect(ctx, left + e.day * cw + 2, y + 1, cw - 4, hh, 4)
    ctx.fill()
    ctx.fillStyle = shade(col, 0.62)
    roundRect(ctx, left + e.day * cw + 2, y + 1, 2.5, hh, 1.25)
    ctx.fill()
    if (hh > 13) text(ctx, e.t, left + e.day * cw + 8, y + 12, { size: 10.5, weight: 600, color: shade(col, 0.3), max: cw - 11 })
  }
}

// A month, with one entry in it.
function monthView(ctx, w, h, d) {
  const mm = /^(\w+)\s*(\d*)/.exec(d.month || "")
  text(ctx, mm?.[1] || d.month || "", 20, BAR_H + 36, { size: 22, weight: 700 })
  if (mm?.[2]) {
    ctx.font = `700 22px ${FONT}`
    text(ctx, mm[2], 20 + ctx.measureText(mm[1]).width + 8, BAR_H + 36, { size: 22, weight: 400, color: SOFT })
  }
  const top = BAR_H + 70
  const cw = (w - 24) / 7
  const rows = Math.ceil((d.first + d.days) / 7)
  const rh = (h - top - 10) / rows
  ;["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].forEach((s, i) => text(ctx, s, 12 + i * cw + cw - 8, top - 10, { size: 11, weight: 600, color: SOFT, align: "right" }))
  for (let k = 0; k < rows * 7; k++) {
    const day = k - d.first + 1
    const x = 12 + (k % 7) * cw
    const y = top + Math.floor(k / 7) * rh
    ctx.fillStyle = RULE
    ctx.fillRect(x, y, cw, 1)
    if (day < 1 || day > d.days) continue
    if (day === d.today) {
      ctx.fillStyle = "#e5392f"
      ctx.beginPath()
      ctx.arc(x + cw - 15, y + 13, 11, 0, Math.PI * 2)
      ctx.fill()
    }
    text(ctx, String(day), day === d.today ? x + cw - 15 : x + cw - 8, y + 17.5, { size: 12.5, weight: day === d.today ? 700 : 400, color: day === d.today ? "#fff" : INK, align: day === d.today ? "center" : "right" })
    for (const e of d.events || [])
      if (e.day === day) {
        const col = e.color || "#d6dff0"
        // big enough to read with the whole desktop in view
        const ph = Math.min(28, rh - 32)
        ctx.fillStyle = col
        roundRect(ctx, x + 4, y + 28, cw - 8, ph, 5)
        ctx.fill()
        ctx.fillStyle = shade(col, 0.62)
        roundRect(ctx, x + 4, y + 28, 3.5, ph, 1.75)
        ctx.fill()
        text(ctx, e.t, x + 12, y + 28 + ph / 2 + 5.5, { size: 15.5, weight: 700, color: shade(col, 0.3), max: cw - 18 })
      }
  }
}

// Safari's start page: Favourites
function startPage(ctx, w, h, top) {
  const cw = Math.min(w - 40, 560)
  const x0 = (w - cw) / 2
  text(ctx, "Favourites", x0, top + 54, { size: 17, weight: 700 })
  const favs = ["Pinecrest", "Council", "Weather", "Maps", "News", "Bank"]
  const n = Math.max(3, Math.min(6, Math.floor(cw / 86)))
  const step = cw / n
  favs.slice(0, n * 2).forEach((name, i) => {
    const x = x0 + (i % n) * step + step / 2
    const y = top + 80 + Math.floor(i / n) * 98
    if (y + 72 > h) return
    favTile(ctx, name, x - 30, y, 60)
    text(ctx, name, x, y + 78, { size: 11.5, color: INK, align: "center", max: step - 6 })
  })
}

// A shop's order page: its header, a tick, the order in a card
function storePage(ctx, w, h, top, d) {
  const site = host(d.url)
  const brand = site.split(".")[0].replace(/^./, (c) => c.toUpperCase())
  ctx.fillStyle = "#fafafa"
  ctx.fillRect(0, top, w, 44)
  ctx.fillStyle = RULE
  ctx.fillRect(0, top + 44, w, 1)
  text(ctx, brand, 24, top + 28, { size: 17, weight: 800, color: "#16181d" })
  if (w > 420) text(ctx, "Account    Orders    Cart", w - 24, top + 27, { size: 12.5, color: SOFT, align: "right" })
  const cw = Math.min(w - 48, 560)
  const x0 = (w - cw) / 2
  let y = top + 44 + 34
  ctx.fillStyle = "#2fa84f"
  ctx.beginPath()
  ctx.arc(x0 + 14, y + 6, 14, 0, Math.PI * 2)
  ctx.fill()
  ctx.strokeStyle = "#fff"
  ctx.lineWidth = 2.4
  ctx.lineCap = "round"
  ctx.lineJoin = "round"
  ctx.beginPath()
  ctx.moveTo(x0 + 8, y + 6.5)
  ctx.lineTo(x0 + 12.5, y + 11)
  ctx.lineTo(x0 + 20.5, y + 1.5)
  ctx.stroke()
  text(ctx, d.heading || "", x0 + 40, y + 14, { size: 22, weight: 700, max: cw - 40 })
  y += 42
  const body = (d.lines || []).filter((l) => !l.gap)
  if (!body.length) return
  const gaps = (d.lines || []).filter((l) => l.gap).length
  const adv = (l) => (l.size ? Math.max(l.size, 22) * 1.6 : 26)
  // the first baseline sits 34 in; the last line's own advance is replaced by the bottom padding
  const ch = Math.min(h - y - 16, 34 + body.reduce((a, l) => a + adv(l), 0) - adv(body.at(-1)) + gaps * 15.6 + 24)
  ctx.fillStyle = "#fff"
  roundRect(ctx, x0, y, cw, ch, 12)
  ctx.fill()
  ctx.strokeStyle = "rgba(0,0,0,.12)"
  ctx.lineWidth = 1
  roundRect(ctx, x0 + 0.5, y + 0.5, cw - 1, ch - 1, 12)
  ctx.stroke()
  const big = (l) => (typeof l === "object" && l.size ? { ...l, size: Math.max(l.size, 22) } : l)
  lines(ctx, (d.lines || []).map(big), x0 + 22, y + 34, cw - 44, { size: 15, gap: 26 })
}

const AVATARS = ["#a3a8b4", "#b0a49a", "#9fb0a4", "#b3a2b4", "#a4adb8"]
function initials(name) {
  return (name || "?").replace(/\(.*?\)|·.*$/g, "").trim().split(/\s+/).filter((p) => /^[A-Za-z]/.test(p)).slice(0, 2).map((p) => p[0].toUpperCase()).join("") || "?"
}
function avatar(ctx, name, cx, cy, r, k = 0) {
  const g = ctx.createLinearGradient(0, cy - r, 0, cy + r)
  const c = AVATARS[k % AVATARS.length]
  g.addColorStop(0, c)
  g.addColorStop(1, shade(c, 0.82))
  ctx.fillStyle = g
  ctx.beginPath()
  ctx.arc(cx, cy, r, 0, Math.PI * 2)
  ctx.fill()
  text(ctx, initials(name), cx, cy + r * 0.36, { size: r * 0.9, weight: 600, color: "#fff", align: "center" })
}

// Messages' conversation list
function chatList(ctx, h, side, d, focused) {
  ctx.clearRect(0, BAR_H, side, h - BAR_H)
  ctx.fillStyle = focused ? "rgba(232,232,234,.78)" : "rgba(242,242,244,.86)"
  ctx.fillRect(0, BAR_H, side, h - BAR_H)
  ctx.fillStyle = "rgba(0,0,0,.1)"
  ctx.fillRect(side - 0.5, BAR_H, 0.5, h - BAR_H)
  ctx.fillStyle = "rgba(0,0,0,.06)"
  roundRect(ctx, 10, BAR_H + 8, side - 20, 24, 7)
  ctx.fill()
  glyph(ctx, "search", 24, BAR_H + 20, "rgba(0,0,0,.4)", 0.75)
  text(ctx, "Search", 36, BAR_H + 24.5, { size: 12.5, color: "rgba(0,0,0,.4)" })
  const last = (d.msgs || []).at(-1)
  const rows = [{ n: d.who, p: last?.t || "", t: "11:52 PM" }]
  for (const r of [
    { n: "Pinecrest Neighbours · 9", p: "Whose is the blue car?", t: "8:02 PM" },
    { n: "Alan (No. 10)", p: "Thanks for the drill!", t: "Yesterday" },
    { n: "Ruth (No. 8)", p: "See you Saturday", t: "Saturday" },
    { n: "Juliana (No. 16)", p: "Just putting mine out Wednesdays now, hope that's ok!", t: "11/26/25" },
    { n: "Pinecrest Dental", p: "Your appointment is confirmed", t: "11/14/25" },
  ])
    if (r.n !== d.who) rows.push(r)
  rows.forEach((r, i) => {
    const y = BAR_H + 40 + i * 60
    if (y > h) return
    const on = i === 0 && focused
    if (i === 0) {
      ctx.fillStyle = on ? SEL : "rgba(0,0,0,.08)"
      roundRect(ctx, 8, y, side - 16, 56, 8)
      ctx.fill()
    }
    avatar(ctx, r.n, 34, y + 28, 19, i)
    text(ctx, r.n || "", 62, y + 23, { size: 13, weight: 600, color: on ? "#fff" : INK, max: side - 140 })
    text(ctx, r.t, side - 16, y + 23, { size: 11.5, color: on ? "rgba(255,255,255,.8)" : SOFT, align: "right" })
    // unread: a count in a blue capsule at the row's end
    const u = d.unread?.[r.n]
    let pw = 0
    if (u) {
      ctx.font = `600 11px ${FONT}`
      pw = Math.max(18, ctx.measureText(String(u)).width + 10)
      ctx.fillStyle = on ? "#fff" : ACCENT
      roundRect(ctx, side - 16 - pw, y + 30, pw, 16, 8)
      ctx.fill()
      text(ctx, String(u), side - 16 - pw / 2, y + 42, { size: 11, weight: 600, color: on ? ACCENT : "#fff", align: "center" })
    }
    text(ctx, r.p, 62, y + 41, { size: 12, weight: u ? 600 : 400, color: on ? "rgba(255,255,255,.85)" : u ? INK : SOFT, max: side - 78 - (pw ? pw + 6 : 0) })
  })
}

// one bubble, with its tail at the bottom corner on the sender's side
function bubble(ctx, x, y, bw, bh, me, color) {
  ctx.fillStyle = color
  roundRect(ctx, x, y, bw, bh, Math.min(18, bh / 2))
  ctx.fill()
  ctx.beginPath()
  if (me) {
    const ex = x + bw
    ctx.moveTo(ex - 10, y + bh - 14)
    ctx.quadraticCurveTo(ex - 2, y + bh - 2, ex + 5, y + bh)
    ctx.quadraticCurveTo(ex - 6, y + bh + 0.5, ex - 14, y + bh - 4)
  } else {
    ctx.moveTo(x + 10, y + bh - 14)
    ctx.quadraticCurveTo(x + 2, y + bh - 2, x - 5, y + bh)
    ctx.quadraticCurveTo(x + 6, y + bh + 0.5, x + 14, y + bh - 4)
  }
  ctx.closePath()
  ctx.fill()
}

// a message's lines at 15 px, wrapped to the bubble's widest
function wrapMsg(ctx, s, max) {
  ctx.font = `400 15px ${FONT}`
  const out = []
  let line = ""
  for (const word of s.split(" ")) {
    const next = line ? line + " " + word : word
    if (line && ctx.measureText(next).width > max) {
      out.push(line)
      line = word
    } else line = next
  }
  if (line) out.push(line)
  return out
}

function thread(ctx, w, h, d) {
  ctx.fillStyle = PAGE
  ctx.fillRect(0, BAR_H, w, h - BAR_H)
  const foot = d.compose != null ? 52 : 0
  const maxW = Math.min(w * 0.72, 420)
  // the conversation as rows: a centred time, or a bubble (with its lines), in order
  const rows = [{ stamp: d.day || d.when || "Today 11:52 PM", head: 1 }]
  for (const m of d.msgs || []) rows.push(m.stamp ? { stamp: m.stamp } : { m, lines: wrapMsg(ctx, m.t, maxW - 26) })
  const sent = (d.sent || []).slice(0, d.shownSent ?? (d.sent || []).length)
  if (sent.length && d.sentAt) rows.push({ stamp: d.sentAt })
  sent.forEach((t, i) => rows.push({ m: { t, me: 1 }, lines: wrapMsg(ctx, t, maxW - 26), last: i === sent.length - 1, pop: i === sent.length - 1 ? d.pop ?? 1 : 1 }))
  // heights: bubbles in a run sit 3 px apart and only the run's last one has a tail; a new sender or a
  // time opens a 12 px gap; "Delivered" sits under the last one he sent
  const bh = (r) => 14 + r.lines.length * 20
  let total = 0
  rows.forEach((r, i) => {
    const prev = rows[i - 1]
    r.gap = !prev ? 0 : r.stamp || prev.stamp || !!prev.m.me !== !!r.m?.me || prev.m.from ? 12 : 3
    r.h = r.stamp ? (r.head ? 34 : 30) : (r.m.from ? 15 : 0) + bh(r)
    total += r.gap + r.h
  })
  if (sent.length) total += 18
  // a long thread is scrolled to its newest message, the way Messages opens it
  const top = BAR_H + 18
  const room = h - foot - 12 - top
  let y = top - Math.max(0, total - room)
  ctx.save()
  ctx.beginPath()
  ctx.rect(0, BAR_H, w, h - BAR_H - foot)
  ctx.clip()
  rows.forEach((r, i) => {
    y += r.gap
    if (r.stamp) {
      if (r.head) text(ctx, "iMessage", w / 2, y + 4, { size: 11, weight: 600, color: SOFT, align: "center" })
      text(ctx, r.stamp, w / 2, y + (r.head ? 18 : 14), { size: 11, color: SOFT, align: "center" })
      y += r.h
      return
    }
    const m = r.m
    ctx.font = `400 15px ${FONT}`
    const bw = Math.min(maxW, Math.max(...r.lines.map((l) => ctx.measureText(l).width)) + 26)
    const x = m.me ? w - bw - 18 : 18
    if (m.from) {
      text(ctx, m.from, x + 12, y + 8, { size: 11, color: SOFT })
      y += 15
    }
    const next = rows[i + 1]
    const tail = !next || next.stamp || !!next.m?.me !== !!m.me
    const hh = bh(r)
    ctx.save()
    // a message just sent pops up from its corner
    if (r.pop < 1) {
      const k = 0.86 + 0.14 * r.pop + 0.03 * Math.sin(r.pop * Math.PI)
      const ox = m.me ? x + bw : x
      ctx.globalAlpha = Math.min(1, r.pop * 2.2)
      ctx.translate(ox, y + hh)
      ctx.scale(k, k)
      ctx.translate(-ox, -(y + hh))
    }
    if (tail) bubble(ctx, x, y, bw, hh, m.me, m.me ? ACCENT : "#e9e9eb")
    else {
      ctx.fillStyle = m.me ? ACCENT : "#e9e9eb"
      roundRect(ctx, x, y, bw, hh, Math.min(18, hh / 2))
      ctx.fill()
    }
    r.lines.forEach((l, k) => text(ctx, l, x + 13, y + 22 + k * 20, { size: 15, color: m.me ? "#fff" : "#000" }))
    ctx.restore()
    if (m.note) text(ctx, m.note, m.me ? w - 18 : 20, y + hh + 17, { size: 11, weight: 500, color: SOFT, align: m.me ? "right" : "left" })
    if (r.last && r.pop >= 1) text(ctx, "Delivered", w - 20, y + hh + 14, { size: 11, weight: 500, color: SOFT, align: "right" })
    y += hh
  })
  ctx.restore()
  if (d.compose != null) {
    const fy = h - 42
    // the apps button, then the field
    ctx.fillStyle = "rgba(0,0,0,.07)"
    ctx.beginPath()
    ctx.arc(28, fy + 15, 13, 0, Math.PI * 2)
    ctx.fill()
    glyph(ctx, "plus", 28, fy + 15, "rgba(0,0,0,.5)", 0.8)
    ctx.strokeStyle = "rgba(0,0,0,.16)"
    ctx.lineWidth = 1
    roundRect(ctx, 50.5, fy + 0.5, w - 66, 30, 15)
    ctx.stroke()
    text(ctx, d.compose || "iMessage", 64, fy + 20, { size: 14, color: d.compose ? INK : "rgba(0,0,0,.3)", max: w - 96 })
    if (d.caret) {
      ctx.font = `400 14px ${FONT}`
      ctx.fillStyle = ACCENT
      ctx.fillRect(63 + (d.compose ? ctx.measureText(d.compose).width + 1 : 0), fy + 7, 1.5, 17)
    }
  }
}

const painters = {
  // Notes: the list of notes on the left, the selected one in Notes' yellow; the note on white,
  // its date centred above it
  jot(ctx, w, h, d, win, focused) {
    const side = sideWidth(win, w)
    ctx.fillStyle = PAGE
    ctx.fillRect(side, BAR_H, w - side, h - BAR_H)
    if (side) {
      sidebar(ctx, w, h, side, false, focused)
      const base = ["To Do", "Recipes", "Q3 ideas", "passwords??"]
      // the open note heads the list
      const list = d.list || [d.heading || "To Do", ...base.filter((n) => n !== (d.heading || "To Do"))].slice(0, 5)
      const when = ["11:48 PM", "Yesterday", "Sunday", "11/30/25", "11/12/25"]
      const first = d.lines?.find((l) => typeof l === "string" || (l && l.t))
      list.forEach((n, i) => {
        const y = BAR_H + 8 + i * 56
        if (y > h) return
        if (i === 0) {
          ctx.fillStyle = focused ? "rgba(255,206,0,.5)" : "rgba(0,0,0,.08)"
          roundRect(ctx, 8, y, side - 16, 50, 7)
          ctx.fill()
        } else if (i > 1) {
          ctx.fillStyle = RULE
          ctx.fillRect(20, y - 3, side - 32, 1)
        }
        text(ctx, n, 20, y + 21, { size: 13, weight: 700, max: side - 40 })
        const pv = i === 0 && first ? (typeof first === "string" ? first : first.t) : "No additional text"
        text(ctx, when[i % when.length], 20, y + 39, { size: 11.5, color: INK })
        ctx.font = `400 11.5px ${FONT}`
        const dw = ctx.measureText(when[i % when.length]).width
        text(ctx, pv, 20 + dw + 8, y + 39, { size: 11.5, color: SOFT, max: side - 48 - dw })
      })
    }
    const x0 = side + 26
    let y = BAR_H + 26
    if (d.edited) text(ctx, d.edited, (side + w) / 2, y, { size: 11.5, weight: 500, color: SOFT, align: "center" })
    y += 32
    text(ctx, d.heading || "", x0, y, { size: 22, weight: 700, max: w - x0 - 24 })
    y += 34
    lines(ctx, d.lines || [], x0, y, w - x0 - 24, { size: 15, gap: 25, check: d.check })
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
    ctx.fillStyle = "#fbe88f"
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    const font = '"Marker Felt", "Comic Neue", ' + FONT
    ctx.font = `500 26px ${font}`
    const ls = wrap(ctx, d.text || "", w - 44)
    ls.forEach((ln, i) => {
      const y = BAR_H + 44 + i * 34
      text(ctx, ln, 22, y, { size: 26, weight: 500, color: "#3f3510", font })
      if (d.done) {
        ctx.fillStyle = "#6c5a14"
        ctx.font = `500 26px ${font}`
        ctx.fillRect(22, y - 9, ctx.measureText(ln).width, 1.8)
      }
    })
  },
  paper(ctx, w, h, d) {
    paperBody(ctx, w, h, d)
  },
  // Mail: mailboxes, the message list (the selection in the accent colour), and the message with
  // its header block
  mail(ctx, w, h, d, win, focused) {
    const side = sideWidth(win, w)
    const listW = w - side > 500 ? Math.min(320, (w - side) * 0.44) : w - side
    ctx.fillStyle = PAGE
    ctx.fillRect(side, BAR_H, w - side, h - BAR_H)
    if (side) {
      sidebar(ctx, w, h, side, false, focused)
      text(ctx, "Favourites", 18, BAR_H + 24, { size: 11, weight: 600, color: SOFT })
      sideItem(ctx, "Inbox", BAR_H + 50, side, { sel: 1, count: d.count || "1,206", icon: "tray", focused })
      sideItem(ctx, "Flagged", BAR_H + 78, side, { icon: "flag", focused })
      sideItem(ctx, "Sent", BAR_H + 106, side, { icon: "send", focused })
      text(ctx, "On My Mac", 18, BAR_H + 142, { size: 11, weight: 600, color: SOFT })
      sideItem(ctx, "HOA", BAR_H + 168, side, { icon: 1, count: "212", focused })
      sideItem(ctx, "Receipts", BAR_H + 196, side, { icon: 1, focused })
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
      const sel = i === d.sel
      const on = sel && focused
      if (sel) {
        ctx.fillStyle = on ? SEL : "rgba(0,0,0,.08)"
        roundRect(ctx, x0 + 6, y + 2, listW - 12, rowH - 4, 6)
        ctx.fill()
      } else if (i !== d.sel - 1) {
        ctx.fillStyle = RULE
        ctx.fillRect(x0 + 26, y + rowH - 1, listW - 34, 1)
      }
      if (m.unread) {
        ctx.fillStyle = on ? "#fff" : ACCENT
        ctx.beginPath()
        ctx.arc(x0 + 15, y + 20, 4, 0, Math.PI * 2)
        ctx.fill()
      }
      text(ctx, m.from, x0 + 26, y + 25, { size: 14, weight: 700, color: on ? "#fff" : INK, max: listW - 110 })
      text(ctx, m.when || "", x0 + listW - 16, y + 25, { size: 12, color: on ? "rgba(255,255,255,.8)" : SOFT, align: "right" })
      text(ctx, (m.clip ? "📎 " : "") + m.subject, x0 + 26, y + 46, { size: 13.5, color: on ? "#fff" : INK, max: listW - 44 })
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
        // the header block: the sender's initials, name, subject, to, and the date
        const ax = x0 + 26
        const ay = BAR_H + 20
        const ini = (m.from || "?").split(/\s+/).filter((p) => /^[A-Za-z]/.test(p)).slice(0, 2).map((p) => p[0].toUpperCase()).join("")
        const g = ctx.createLinearGradient(0, ay, 0, ay + 40)
        g.addColorStop(0, "#a8a8ad")
        g.addColorStop(1, "#8a8a8f")
        ctx.fillStyle = g
        ctx.beginPath()
        ctx.arc(ax + 20, ay + 20, 20, 0, Math.PI * 2)
        ctx.fill()
        text(ctx, ini, ax + 20, ay + 26, { size: 15, weight: 600, color: "#fff", align: "center" })
        const tx = ax + 52
        text(ctx, m.from, tx, ay + 14, { size: 15, weight: 700, max: mw - 140 })
        if (m.when) text(ctx, m.when, w - 22, ay + 14, { size: 12, color: SOFT, align: "right" })
        text(ctx, m.subject, tx, ay + 34, { size: 14, weight: 500, max: mw - 52 })
        text(ctx, "To: " + (m.to || "Me") + (m.cc ? "   Cc: " + m.cc : ""), tx, ay + 52, { size: 12, color: SOFT, max: mw - 52 })
        ctx.fillStyle = RULE
        ctx.fillRect(x0 + 20, ay + 70, w - x0 - 40, 1)
        const end = lines(ctx, m.body || [], x0 + 28, ay + 102, mw, { size: 15, gap: 24 })
        if (m.attach) {
          // the attachment, as a chip with a thumbnail
          const cx = x0 + 28
          const cy = end + 4
          ctx.fillStyle = m.attachSel ? "#d6e6ff" : "#f2f2f4"
          roundRect(ctx, cx, cy, 230, 64, 10)
          ctx.fill()
          ctx.strokeStyle = m.attachSel ? "rgba(10,132,255,.5)" : "rgba(0,0,0,.1)"
          ctx.lineWidth = 1
          roundRect(ctx, cx + 0.5, cy + 0.5, 229, 63, 10)
          ctx.stroke()
          const a = plates.across
          if (a) {
            const [u0, v0, u1, v1] = coverUV(a, 64, 48)
            ctx.save()
            roundRect(ctx, cx + 8, cy + 8, 64, 48, 5)
            ctx.clip()
            ctx.drawImage(a, u0 * a.width, v0 * a.height, (u1 - u0) * a.width, (v1 - v0) * a.height, cx + 8, cy + 8, 64, 48)
            ctx.restore()
          }
          text(ctx, m.attach, cx + 84, cy + 28, { size: 13.5, weight: 600, max: 140 })
          text(ctx, "JPEG image · 2.4 MB", cx + 84, cy + 47, { size: 12, color: SOFT })
        }
      }
    }
  },
  // Safari: the address is in the title bar; under it the tab bar, then the page
  browser(ctx, w, h, d, win, focused) {
    const TB = 28
    ctx.fillStyle = focused ? "#e3e3e5" : "#efeff1"
    ctx.fillRect(0, BAR_H, w, TB)
    ctx.fillStyle = "rgba(0,0,0,.1)"
    ctx.fillRect(0, BAR_H + TB - 0.5, w, 0.5)
    const n = d.tabs || 1
    // the open tab keeps its width; the rest crush down to their favicons, as Safari does with 38 of them
    if (n > 1) {
      const aw = Math.max(80, Math.min(240, (w - 16) * 0.34, (w - 16) / Math.min(n, 4)))
      const rw = Math.max(9, (w - 16 - aw) / (n - 1))
      ctx.fillStyle = focused ? "#f6f6f7" : "#fafafa"
      roundRect(ctx, 8, BAR_H + 3, aw, TB - 6, 6)
      ctx.fill()
      ctx.fillStyle = "rgba(0,0,0,.06)"
      roundRect(ctx, 8.5, BAR_H + 3.5, aw - 1, TB - 7, 6)
      ctx.fill()
      ctx.fillStyle = focused ? "#fff" : "#fbfbfb"
      roundRect(ctx, 8.5, BAR_H + 3.5, aw - 1, TB - 7.5, 6)
      ctx.fill()
      ctx.fillStyle = FAVICONS[0]
      roundRect(ctx, 16, BAR_H + TB / 2 - 5, 10, 10, 2.5)
      ctx.fill()
      text(ctx, d.tab || TAB_TITLES[0], 32, BAR_H + TB / 2 + 4, { size: 11.5, color: INK, max: aw - 34 })
      for (let i = 1; i < n; i++) {
        const x = 8 + aw + (i - 1) * rw
        if (x + rw > w - 6) break
        if (i > 1) {
          ctx.fillStyle = "rgba(0,0,0,.12)"
          ctx.fillRect(x - 0.5, BAR_H + 8, 1, TB - 16)
        }
        const f = Math.min(10, rw - 4)
        if (rw >= 70) {
          ctx.fillStyle = FAVICONS[i % FAVICONS.length]
          roundRect(ctx, x + 8, BAR_H + TB / 2 - 5, 10, 10, 2.5)
          ctx.fill()
          text(ctx, (d.tabNames || TAB_TITLES)[(i - 1) % (d.tabNames || TAB_TITLES).length], x + 24, BAR_H + TB / 2 + 4, { size: 11.5, color: "#5b5b60", max: rw - 32 })
        } else if (rw >= 11) {
          ctx.fillStyle = FAVICONS[i % FAVICONS.length]
          roundRect(ctx, x + (rw - f) / 2, BAR_H + TB / 2 - f / 2, f, f, 2.5)
          ctx.fill()
        }
      }
    }
    const top = BAR_H + (n > 1 ? TB : 0)
    ctx.fillStyle = PAGE
    ctx.fillRect(0, top, w, h - top)
    if (d.page === "spaceman") return sitePage(ctx, w, h, top, d)
    if (d.tab === "New Tab") return startPage(ctx, w, h, top)
    if (d.lines) return storePage(ctx, w, h, top, d)
    const cw = Math.min(w - 52, 620)
    const x0 = (w - cw) / 2
    let y = top + 52
    text(ctx, d.heading || "", x0, y, { size: 26, weight: 700, max: cw, font: SERIF })
    y += 26
    text(ctx, "By the Home Desk · 6 min read", x0, y, { size: 12.5, color: SOFT })
    y += 30
    ctx.font = `400 15px ${SERIF}`
    for (const para of ARTICLE) {
      for (const ln of wrap(ctx, para, cw)) {
        if (y > h - 8) return
        text(ctx, ln, x0, y, { size: 15, color: "#2c2c2e", font: SERIF })
        y += 23
      }
      y += 12
    }
  },
  // Messages: conversations on the left when there's room; the thread with its timestamp, bubbles
  // with tails, and the compose field
  chat(ctx, w, h, d, win, focused) {
    const side = sideWidth(win, w)
    if (side) chatList(ctx, h, side, d, focused)
    ctx.save()
    ctx.translate(side, 0)
    thread(ctx, w - side, h, d)
    ctx.restore()
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
  // Calendar's day view: the date with today's number in red, hour lines, events as tinted blocks
  // with a solid edge in their calendar's colour
  daybook(ctx, w, h, d) {
    ctx.fillStyle = PAGE
    ctx.fillRect(0, BAR_H, w, h - BAR_H)
    if (d.week) return weekView(ctx, w, h, d)
    if (d.month) return monthView(ctx, w, h, d)
    const m = /^(\w+),\s+(\w+)\s+(\d+)/.exec(d.date || "")
    if (m) {
      text(ctx, m[2] + " " + m[3], 20, BAR_H + 36, { size: 22, weight: 700 })
      ctx.font = `700 22px ${FONT}`
      text(ctx, m[1], 20 + ctx.measureText(m[2] + " " + m[3]).width + 10, BAR_H + 36, { size: 22, weight: 400, color: "#d63a2f" })
    } else text(ctx, d.date || "", 20, BAR_H + 36, { size: 20, weight: 700 })
    const top = BAR_H + 56
    const hour = d.hour || 40
    const from = d.from ?? 7
    ctx.fillStyle = RULE
    ctx.fillRect(0, top - 6, w, 1)
    for (let i = 0; i < 24; i++) {
      const y = top + i * hour
      if (y > h) break
      ctx.fillStyle = "rgba(0,0,0,.07)"
      ctx.fillRect(60, y, w - 60, 1)
      const hh = (i + from) % 24
      if (i) text(ctx, `${hh % 12 || 12} ${hh < 12 ? "AM" : "PM"}`, 52, y + 4, { size: 10.5, weight: 500, color: SOFT, align: "right" })
    }
    for (const e of d.events || []) {
      const y = top + (e.at - from) * hour
      const col = e.color || "#e9c9c4"
      const eh = e.len * hour - 3
      ctx.fillStyle = col
      roundRect(ctx, 64, y + 1.5, w - 80, eh, 5)
      ctx.fill()
      ctx.fillStyle = shade(col, 0.62)
      roundRect(ctx, 64, y + 1.5, 3.5, eh, 1.75)
      ctx.fill()
      text(ctx, e.t, 76, y + 18, { size: 13.5, weight: 600, color: shade(col, 0.3), max: w - 104 })
      if (eh > 36) text(ctx, `${e.at % 12 || 12}:00 ${e.at < 12 ? "AM" : "PM"}`, 76, y + 34, { size: 11.5, color: shade(col, 0.42) })
    }
  },
  album: (...a) => albumPainter(...a),
  feed(ctx, w, h, d, win) {
    paintFeed(ctx, 0, BAR_H, w, h - BAR_H, d, win)
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
  // Spaceman's Settings, as agent/src/view.rs and map.rs draw it: the panes under the lights, and
  // the Rules pane's map, a tile per workspace with the apps that open there
  settings(ctx, w, h, d, win, focused) {
    const G = settingsGeom(w, h)
    const k = G.k
    ctx.clearRect(0, 0, w, h)
    ctx.fillStyle = "#f2f2f3"
    ctx.fillRect(0, 0, w, h)
    // the sidebar: five panes, the version and the update check at its foot
    const panes = [["General", "general"], ["Rules", "rules"], ["Workspaces", "workspaces"], ["Shortcuts", "shortcuts"], ["Activity", "activity"]]
    panes.forEach(([label, ic], i) => {
      const y = 52 + i * 32
      const on = label === "Rules"
      if (on) {
        ctx.fillStyle = "#e3e3e6"
        roundRect(ctx, 10, y, G.side - 16, 28, 4)
        ctx.fill()
      }
      paneIcon(ctx, ic, 24, y + 14, on ? "#3a3a3e" : "#8a8a90")
      text(ctx, label, 40, y + 18.5, { size: 13, color: on ? INK : "#6e6e73", max: G.side - 56 })
    })
    text(ctx, "Version 0.3.0", 16, h - 38, { size: 11.5, color: "#a1a1a6" })
    text(ctx, "Check for updates", 16, h - 18, { size: 11.5, color: "#6e6e73" })
    // the raised panel
    ctx.fillStyle = "#fff"
    roundRect(ctx, G.px, G.py, G.pw, G.ph, 8)
    ctx.fill()
    ctx.strokeStyle = "rgba(0,0,0,.09)"
    ctx.lineWidth = 1
    roundRect(ctx, G.px + 0.5, G.py + 0.5, G.pw - 1, G.ph - 1, 8)
    ctx.stroke()
    text(ctx, "Rules", G.mx, G.py + 33, { size: 14, weight: 600 })
    // Where apps open
    text(ctx, "Where apps open", G.mx, G.my - 14, { size: 13, weight: 600 })
    ctx.font = `600 13px ${FONT}`
    text(ctx, "Drag an app to move it", G.mx + ctx.measureText("Where apps open").width + 8, G.my - 14, { size: 12, color: SOFT })
    const active = d.active || []
    for (let n = 1; n <= 10; n++) {
      const t = G.tile(n)
      const hl = d.hl === n
      const on = active.includes(n)
      ctx.fillStyle = hl ? "#eef4ff" : "#f7f7f8"
      roundRect(ctx, t.x, t.y, t.w, t.h, 8)
      ctx.fill()
      ctx.strokeStyle = hl ? ACCENT : on ? "rgba(0,0,0,.22)" : "rgba(0,0,0,.1)"
      ctx.lineWidth = hl || on ? 1.5 : 1
      roundRect(ctx, t.x + 0.5, t.y + 0.5, t.w - 1, t.h - 1, 8)
      ctx.stroke()
      text(ctx, String(n), t.x + 12 * k, t.y + 10 * k + 12, { size: 13, weight: 500, color: on ? INK : SOFT })
      if (d.hover === n || d.menu === n) {
        const [ax, ay] = G.add(n)
        ctx.fillStyle = d.menu === n ? "rgba(0,0,0,.1)" : "rgba(0,0,0,.06)"
        roundRect(ctx, ax - 11 * k, ay - 11 * k, 22 * k, 22 * k, 4)
        ctx.fill()
        glyph(ctx, "plus", ax, ay, "#6e6e73", 0.62 * k + 0.1)
      }
      const here = (d.rules || []).filter((r) => r.ws === n)
      here.filter((r) => !r.title).forEach((r, slot) => {
        const p = G.icon(n, slot)
        ctx.drawImage(iconCanvas(r.icon, 72), p.x, p.y, p.s, p.s)
      })
      // a rule by window title: the app's icon and the title, in a chip along the tile's foot
      here.filter((r) => r.title).forEach((r, i) => {
        const ch = 24 * k
        const cx = t.x + 8 * k
        const cy = t.y + t.h - 8 * k - ch - i * (ch + 4 * k)
        const a = r.in ?? 1
        ctx.save()
        ctx.globalAlpha = a
        ctx.fillStyle = "#fff"
        roundRect(ctx, cx, cy, t.w - 16 * k, ch, 6 * k)
        ctx.fill()
        ctx.strokeStyle = "rgba(0,0,0,.14)"
        ctx.lineWidth = 1
        roundRect(ctx, cx + 0.5, cy + 0.5, t.w - 16 * k - 1, ch - 1, 6 * k)
        ctx.stroke()
        ctx.drawImage(iconCanvas(r.icon, 48), cx + 4 * k, cy + 4 * k, 16 * k, 16 * k)
        text(ctx, r.title, cx + 24 * k, cy + ch / 2 + 4 * k, { size: 11.5 * k + 1, weight: 500, max: t.w - 46 * k })
        ctx.restore()
      })
    }
    // Floating
    text(ctx, "Floating", G.mx, G.floatTop + 14, { size: 13, weight: 600 })
    const by = G.floatTop + 24
    ctx.fillStyle = "#f7f7f8"
    roundRect(ctx, G.mx, by, G.mapW, 52 * k, 8)
    ctx.fill()
    ctx.strokeStyle = "rgba(0,0,0,.1)"
    roundRect(ctx, G.mx + 0.5, by + 0.5, G.mapW - 1, 52 * k - 1, 8)
    ctx.stroke()
    ctx.strokeStyle = "rgba(0,0,0,.16)"
    roundRect(ctx, G.mx + 10 * k + 0.5, by + 10 * k + 0.5, 128 * k, 32 * k, 6)
    ctx.stroke()
    glyph(ctx, "plus", G.mx + 26 * k, by + 26 * k, "#8a8a90", 0.6)
    text(ctx, "Add app", G.mx + 38 * k, by + 26 * k + 4.5, { size: 13, color: SOFT })
    // the apps that float: icon and name, after the Add button
    ;(d.floats || []).forEach((f, i) => {
      const fx = G.mx + 150 * k + i * 130 * k
      ctx.fillStyle = "#fff"
      roundRect(ctx, fx, by + 10 * k, 120 * k, 32 * k, 6)
      ctx.fill()
      ctx.strokeStyle = "rgba(0,0,0,.14)"
      roundRect(ctx, fx + 0.5, by + 10 * k + 0.5, 120 * k - 1, 32 * k - 1, 6)
      ctx.stroke()
      ctx.drawImage(iconCanvas(f.icon, 48), fx + 8 * k, by + 18 * k, 16 * k, 16 * k)
      text(ctx, f.app, fx + 30 * k, by + 26 * k + 4.5, { size: 13, max: 84 * k })
    })
    // the Add menu: the running apps without a rule
    if (d.menu) {
      const m = G.menu(d.menu)
      ctx.save()
      ctx.shadowColor = "rgba(0,0,0,.22)"
      ctx.shadowBlur = 18
      ctx.shadowOffsetY = 6
      ctx.fillStyle = "rgba(250,250,251,.98)"
      roundRect(ctx, m.x, m.y, m.w, m.h, 9)
      ctx.fill()
      ctx.restore()
      ctx.strokeStyle = "rgba(0,0,0,.12)"
      roundRect(ctx, m.x + 0.5, m.y + 0.5, m.w - 1, m.h - 1, 9)
      ctx.stroke()
      text(ctx, "Running apps", m.x + 14, m.y + 22, { size: 11.5, weight: 600, color: SOFT })
      MENU_APPS.forEach((app, i) => {
        const [, iy] = G.menuItem(i)
        const on = d.menuHover === i
        if (on) {
          ctx.fillStyle = ACCENT
          roundRect(ctx, m.x + 5, iy - 12, m.w - 10, 24, 5)
          ctx.fill()
        }
        ctx.drawImage(iconCanvas(app, 48), m.x + 14, iy - 8, 16, 16)
        text(ctx, APPS[app]?.name || app, m.x + 38, iy + 4.5, { size: 13, color: on ? "#fff" : INK })
      })
    }
  },
}

const MENU_APPS = ["chatter", "jot", "postbox", "browsy", "paper", "daybook"]

// Where everything in the Settings window sits (window coordinates), for the painter and the director.
export function settingsGeom(w, h) {
  const side = sideWidth({ kind: "settings" }, w)
  const px = side
  const py = 8
  const pw = w - side - 8
  const ph = h - 16
  const k = Math.max(0.5, Math.min(1, (pw - 48) / 580, (ph - 56 - 4 - 30 - 24 - 24 - 12) / (210 + 52)))
  const TW = 108 * k
  const TH = 100 * k
  const GP = 10 * k
  const mx = px + 24
  const my = py + 56 + 4 + 30
  const mapW = 5 * TW + 4 * GP
  const tile = (n) => {
    const i = n - 1
    return { x: mx + (i % 5) * (TW + GP), y: my + Math.floor(i / 5) * (TH + GP), w: TW, h: TH }
  }
  const add = (n) => {
    const t = tile(n)
    return [t.x + t.w - 19 * k, t.y + 19 * k]
  }
  const icon = (n, slot) => {
    const t = tile(n)
    return { x: t.x + 12 * k + (slot % 3) * 30 * k, y: t.y + 38 * k + Math.floor(slot / 3) * 30 * k, s: 24 * k }
  }
  const menu = (n) => {
    const [ax, ay] = add(n)
    return { x: ax - 11 * k, y: ay + 11 * k + 4, w: 220, h: 34 + MENU_APPS.length * 24 + 6 }
  }
  const menuItem = (i, n = 2) => {
    const m = menu(n)
    return [m.x + 80, m.y + 34 + i * 24 + 12 - 2]
  }
  const floatTop = my + 2 * TH + GP + 24
  return { side, px, py, pw, ph, k, mx, my, mapW, tile, add, icon, menu, menuItem, floatTop }
}

// the panes' sidebar icons
function paneIcon(ctx, name, x, y, color) {
  ctx.save()
  ctx.translate(x, y)
  ctx.strokeStyle = color
  ctx.fillStyle = color
  ctx.lineWidth = 1.3
  ctx.lineCap = "round"
  ctx.lineJoin = "round"
  ctx.beginPath()
  if (name === "general") {
    ctx.arc(0, 0, 6.5, 0, Math.PI * 2)
    ctx.moveTo(2.5, 0)
    ctx.arc(0, 0, 2.5, 0, Math.PI * 2)
  } else if (name === "rules") {
    for (const yy of [-4, 0, 4]) {
      ctx.moveTo(-2.5, yy)
      ctx.lineTo(6, yy)
      ctx.moveTo(-6, yy)
      ctx.lineTo(-5.4, yy)
    }
  } else if (name === "workspaces") {
    ctx.roundRect(-6.5, -5, 6, 4.5, 1.2)
    ctx.roundRect(0.5, -5, 6, 4.5, 1.2)
    ctx.roundRect(-6.5, 1, 6, 4.5, 1.2)
    ctx.roundRect(0.5, 1, 6, 4.5, 1.2)
  } else if (name === "shortcuts") {
    ctx.roundRect(-7, -4.5, 14, 9, 2)
    ctx.moveTo(-3.5, 2)
    ctx.lineTo(3.5, 2)
    for (const xx of [-4, -1.3, 1.3, 4]) {
      ctx.moveTo(xx, -1.5)
      ctx.lineTo(xx + 0.01, -1.5)
    }
  } else {
    ctx.moveTo(-7, 0)
    ctx.lineTo(-3.5, 0)
    ctx.lineTo(-1.5, -5)
    ctx.lineTo(1.5, 5)
    ctx.lineTo(3.5, 0)
    ctx.lineTo(7, 0)
  }
  ctx.stroke()
  ctx.restore()
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
    [{ kind: "browser", app: "browsy", title: "Hedge height bylaws", data: { tabs: 21, url: "pinecrest.example/bylaws/4-12", heading: "Hedge height bylaws" } }, 40, 60, 620, 470],
    [{ kind: "daybook", app: "daybook", title: "Calendar", data: { date: "Monday, March 2", events: [{ at: 8, len: 1, t: "Bins out" }] } }, 520, 40, 420, 520],
    [jot("Untitled 44", "Untitled 44", ["hedge — measure?", "ask Alan re: drill"], "Edited February 3"), 300, 230, 380, 330],
    [{ kind: "browser", app: "browsy", title: "Cordless drills compared", data: { tabs: 21, url: "homedesk.example/cordless-drills", heading: "Cordless drills compared" } }, 820, 120, 580, 440],
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
