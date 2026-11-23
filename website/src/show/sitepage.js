// Spaceman's download page at anoma.ly/spaceman, as he sees it in Safari on "a small program called Spaceman":
// the end card's dark page in miniature (icon, name, tagline, Download), and the toolbar's
// download indicator after he clicks it.
import { FONT, iconCanvas, roundRect } from "./apps.js"

const clamp = (v, a, b) => Math.max(a, Math.min(b, v))

function label(ctx, s, x, y, size, weight, color) {
  ctx.font = `${weight} ${size}px ${FONT}`
  ctx.fillStyle = color
  ctx.textAlign = "center"
  ctx.textBaseline = "alphabetic"
  ctx.fillText(s, x, y)
}

// Where everything sits on a page w × h whose content starts at top (the director clicks the button).
export function sitePageGeom(w, h, top) {
  const ph = h - top
  const k = clamp(Math.min(w / 520, ph / 400), 0.62, 1.3)
  const icon = 66 * k
  const stack = icon + 22 * k + 30 * k + 12 * k + 15 * k + 28 * k + 38 * k + 14 * k + 20 * k
  const y0 = top + Math.max(10 * k, (ph - stack) / 2 - 6 * k)
  const bw = 176 * k
  const bh = 38 * k
  const by = y0 + icon + 22 * k + 30 * k + 12 * k + 15 * k + 28 * k
  return { k, icon, y0, bx: w / 2 - bw / 2, by, bw, bh }
}

// d.press (0..1) presses the button in
export function sitePage(ctx, w, h, top, d) {
  const ph = h - top
  const g = ctx.createRadialGradient(w / 2, top - ph * 0.12, 0, w / 2, top - ph * 0.12, Math.max(w, ph) * 0.95)
  g.addColorStop(0, "#23232b")
  g.addColorStop(0.42, "#141418")
  g.addColorStop(0.72, "#0a0a0c")
  g.addColorStop(1, "#0a0a0c")
  ctx.fillStyle = g
  ctx.fillRect(0, top, w, ph)
  const G = sitePageGeom(w, h, top)
  const { k } = G
  const cx = w / 2
  let y = G.y0
  // the icon, drawn at three times its size so it stays sharp in a push
  ctx.save()
  ctx.shadowColor = "rgba(0,0,0,.7)"
  ctx.shadowBlur = 18 * k
  ctx.shadowOffsetY = 8 * k
  ctx.drawImage(iconCanvas("spaceman", G.icon * 3), cx - G.icon / 2, y, G.icon, G.icon)
  ctx.restore()
  y += G.icon + 22 * k + 26 * k
  label(ctx, "Spaceman", cx, y, 30 * k, 600, "#f5f5f7")
  y += 12 * k + 15 * k
  label(ctx, "For your windows.", cx, y, 14.5 * k, 400, "#8e8e93")
  // Download for macOS, pressed in a little on the click
  const p = d.press || 0
  const s = 1 - 0.035 * p
  ctx.save()
  ctx.translate(cx, G.by + G.bh / 2)
  ctx.scale(s, s)
  ctx.fillStyle = p ? `rgb(${Math.round(245 - 26 * p)},${Math.round(245 - 26 * p)},${Math.round(247 - 26 * p)})` : "#f5f5f7"
  roundRect(ctx, -G.bw / 2, -G.bh / 2, G.bw, G.bh, G.bh / 2)
  ctx.fill()
  label(ctx, "Download for macOS", 0, 5 * k, 14 * k, 500, "#1d1d1f")
  ctx.restore()
  // the version under it
  const vy = G.by + G.bh + 14 * k
  const vs = "Version 0.3.0 · macOS 14 or later"
  ctx.font = `400 ${11.5 * k}px ${FONT}`
  const vw = ctx.measureText(vs).width + 22 * k
  ctx.fillStyle = "rgba(255,255,255,.06)"
  roundRect(ctx, cx - vw / 2, vy, vw, 20 * k, 10 * k)
  ctx.fill()
  label(ctx, vs, cx, vy + 14 * k, 11.5 * k, 400, "#8e8e93")
}

// Safari's download button in the toolbar: it drops in with a bounce when a download starts, and a
// blue bar under it fills as the file comes down. dl: 0..1 over the download.
export function downloadIndicator(ctx, x, y, dl, color) {
  const bounce = dl < 0.45 ? Math.sin((dl / 0.45) * Math.PI) * (1 - dl / 0.45) * 5 : 0
  const a = clamp(dl / 0.08, 0, 1)
  ctx.save()
  ctx.globalAlpha = a
  ctx.translate(x, y - bounce)
  ctx.strokeStyle = color
  ctx.lineWidth = 1.5
  ctx.lineCap = "round"
  ctx.lineJoin = "round"
  ctx.beginPath()
  ctx.arc(0, -0.5, 6.5, 0, Math.PI * 2)
  ctx.moveTo(0, -4)
  ctx.lineTo(0, 2.5)
  ctx.moveTo(-2.8, -0.3)
  ctx.lineTo(0, 2.5)
  ctx.lineTo(2.8, -0.3)
  ctx.stroke()
  ctx.restore()
  // the progress under it
  ctx.save()
  ctx.globalAlpha = a
  ctx.fillStyle = "rgba(0,0,0,.12)"
  roundRect(ctx, x - 7, y + 9, 14, 2, 1)
  ctx.fill()
  ctx.fillStyle = "#0a84ff"
  roundRect(ctx, x - 7, y + 9, 14 * clamp((dl - 0.1) / 0.9, 0.05, 1), 2, 1)
  ctx.fill()
  ctx.restore()
}
