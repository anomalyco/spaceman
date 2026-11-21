// Invented apps and their icons, drawn here (no real app artwork). Each icon is a rounded square
// with a simple glyph, cached as a canvas per size.
export const FONT = '-apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", "Segoe UI", Arial, sans-serif'
export const MONO = 'ui-monospace, "SF Mono", Menlo, Consolas, monospace'

export const APPS = {
  jot: { name: "Jot", bg: ["#f6d36b", "#e9b93f"] },
  postbox: { name: "Postbox", bg: ["#5d8fd6", "#3c6db8"] },
  browsy: { name: "Browsy", bg: ["#4fb3a7", "#2f8d83"] },
  chatter: { name: "Chatter", bg: ["#8a74d1", "#6a54b5"] },
  daybook: { name: "Daybook", bg: ["#fbfaf7", "#e7e3db"] },
  paper: { name: "Paper", bg: ["#f0a35e", "#d9803a"] },
  sheets: { name: "Sheets", bg: ["#6cbf7d", "#43a05a"] },
  viewer: { name: "Viewer", bg: ["#9fb4c8", "#7290ad"] },
  doorbell: { name: "Doorbell", bg: ["#3a3f47", "#1f2328"] },
  call: { name: "Call", bg: ["#5cc27a", "#36a058"] },
  spaceman: { name: "Spaceman", bg: ["#2b2d33", "#17181c"] },
}

export function roundRect(ctx, x, y, w, h, r) {
  r = Math.max(0, Math.min(r, w / 2, h / 2))
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.arcTo(x + w, y, x + w, y + h, r)
  ctx.arcTo(x + w, y + h, x, y + h, r)
  ctx.arcTo(x, y + h, x, y, r)
  ctx.arcTo(x, y, x + w, y, r)
  ctx.closePath()
}

function glyph(ctx, app, s) {
  ctx.lineCap = "round"
  ctx.lineJoin = "round"
  const W = "rgba(255,255,255,.95)"
  switch (app) {
    case "jot":
      ctx.strokeStyle = "rgba(90,60,10,.55)"
      ctx.lineWidth = s * 0.05
      for (let i = 0; i < 4; i++) {
        ctx.beginPath()
        ctx.moveTo(s * 0.26, s * (0.34 + i * 0.12))
        ctx.lineTo(s * (i === 3 ? 0.55 : 0.74), s * (0.34 + i * 0.12))
        ctx.stroke()
      }
      break
    case "postbox":
      ctx.strokeStyle = W
      ctx.lineWidth = s * 0.06
      roundRect(ctx, s * 0.22, s * 0.32, s * 0.56, s * 0.38, s * 0.05)
      ctx.stroke()
      ctx.beginPath()
      ctx.moveTo(s * 0.24, s * 0.35)
      ctx.lineTo(s * 0.5, s * 0.54)
      ctx.lineTo(s * 0.76, s * 0.35)
      ctx.stroke()
      break
    case "browsy":
      ctx.strokeStyle = W
      ctx.lineWidth = s * 0.055
      ctx.beginPath()
      ctx.arc(s * 0.5, s * 0.5, s * 0.25, 0, Math.PI * 2)
      ctx.stroke()
      ctx.beginPath()
      ctx.ellipse(s * 0.5, s * 0.5, s * 0.1, s * 0.25, 0, 0, Math.PI * 2)
      ctx.moveTo(s * 0.25, s * 0.5)
      ctx.lineTo(s * 0.75, s * 0.5)
      ctx.stroke()
      break
    case "chatter":
      ctx.fillStyle = W
      roundRect(ctx, s * 0.22, s * 0.27, s * 0.56, s * 0.38, s * 0.14)
      ctx.fill()
      ctx.beginPath()
      ctx.moveTo(s * 0.34, s * 0.6)
      ctx.lineTo(s * 0.3, s * 0.76)
      ctx.lineTo(s * 0.48, s * 0.63)
      ctx.fill()
      break
    case "daybook":
      ctx.fillStyle = "#d9534a"
      ctx.fillRect(0, 0, s, s * 0.3)
      ctx.fillStyle = "#33312e"
      ctx.font = `600 ${s * 0.42}px ${FONT}`
      ctx.textAlign = "center"
      ctx.textBaseline = "middle"
      ctx.fillText("14", s * 0.5, s * 0.64)
      break
    case "paper":
      ctx.fillStyle = W
      roundRect(ctx, s * 0.28, s * 0.2, s * 0.44, s * 0.6, s * 0.04)
      ctx.fill()
      ctx.strokeStyle = "rgba(200,110,40,.7)"
      ctx.lineWidth = s * 0.035
      for (let i = 0; i < 4; i++) {
        ctx.beginPath()
        ctx.moveTo(s * 0.35, s * (0.33 + i * 0.1))
        ctx.lineTo(s * (i === 3 ? 0.52 : 0.65), s * (0.33 + i * 0.1))
        ctx.stroke()
      }
      break
    case "sheets":
      ctx.strokeStyle = W
      ctx.lineWidth = s * 0.045
      roundRect(ctx, s * 0.22, s * 0.24, s * 0.56, s * 0.52, s * 0.05)
      ctx.stroke()
      ctx.beginPath()
      for (const f of [0.41, 0.58]) {
        ctx.moveTo(s * 0.22, s * f)
        ctx.lineTo(s * 0.78, s * f)
      }
      ctx.moveTo(s * 0.42, s * 0.24)
      ctx.lineTo(s * 0.42, s * 0.76)
      ctx.stroke()
      break
    case "viewer":
      ctx.fillStyle = W
      ctx.beginPath()
      ctx.moveTo(s * 0.2, s * 0.72)
      ctx.lineTo(s * 0.42, s * 0.42)
      ctx.lineTo(s * 0.56, s * 0.6)
      ctx.lineTo(s * 0.66, s * 0.5)
      ctx.lineTo(s * 0.8, s * 0.72)
      ctx.fill()
      ctx.beginPath()
      ctx.arc(s * 0.66, s * 0.32, s * 0.07, 0, Math.PI * 2)
      ctx.fill()
      break
    case "doorbell":
      ctx.fillStyle = "#0c0e11"
      ctx.beginPath()
      ctx.arc(s * 0.5, s * 0.5, s * 0.24, 0, Math.PI * 2)
      ctx.fill()
      ctx.strokeStyle = "rgba(160,190,220,.6)"
      ctx.lineWidth = s * 0.04
      ctx.stroke()
      ctx.fillStyle = "rgba(170,200,230,.55)"
      ctx.beginPath()
      ctx.arc(s * 0.45, s * 0.45, s * 0.06, 0, Math.PI * 2)
      ctx.fill()
      break
    case "call":
      ctx.fillStyle = W
      roundRect(ctx, s * 0.2, s * 0.34, s * 0.4, s * 0.32, s * 0.06)
      ctx.fill()
      ctx.beginPath()
      ctx.moveTo(s * 0.62, s * 0.46)
      ctx.lineTo(s * 0.8, s * 0.36)
      ctx.lineTo(s * 0.8, s * 0.64)
      ctx.lineTo(s * 0.62, s * 0.54)
      ctx.fill()
      break
    case "spaceman":
      ctx.fillStyle = "rgba(255,255,255,.85)"
      roundRect(ctx, s * 0.24, s * 0.26, s * 0.25, s * 0.48, s * 0.04)
      ctx.fill()
      roundRect(ctx, s * 0.53, s * 0.26, s * 0.23, s * 0.22, s * 0.04)
      ctx.fill()
      roundRect(ctx, s * 0.53, s * 0.52, s * 0.23, s * 0.22, s * 0.04)
      ctx.fill()
      break
  }
}

const iconCache = new Map()
// A canvas holding the app's icon at px pixels.
export function iconCanvas(app, px) {
  px = Math.max(8, Math.round(px))
  const key = app + px
  let c = iconCache.get(key)
  if (c) return c
  c = document.createElement("canvas")
  c.width = c.height = px
  const ctx = c.getContext("2d")
  const a = APPS[app] || APPS.paper
  roundRect(ctx, px * 0.04, px * 0.04, px * 0.92, px * 0.92, px * 0.22)
  const g = ctx.createLinearGradient(0, 0, 0, px)
  g.addColorStop(0, a.bg[0])
  g.addColorStop(1, a.bg[1])
  ctx.fillStyle = g
  ctx.fill()
  ctx.save()
  ctx.clip()
  glyph(ctx, app, px)
  ctx.restore()
  ctx.strokeStyle = "rgba(0,0,0,.18)"
  ctx.lineWidth = Math.max(0.5, px * 0.02)
  roundRect(ctx, px * 0.04, px * 0.04, px * 0.92, px * 0.92, px * 0.22)
  ctx.stroke()
  iconCache.set(key, c)
  return c
}
