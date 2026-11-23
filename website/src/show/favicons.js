// Safari's Favourites tiles: each site's touch icon as vectors (no image files, no real logos).
import { roundRect } from "./apps.js"

// A Favourites tile: the site's touch icon, drawn as vectors so it stays sharp at 2x (s: tile size)
const FAV = {
  Pinecrest: ["#4c9a68", "#2a6440"],
  Council: ["#3b6fbe", "#22477f"],
  Weather: ["#63b8f4", "#2c7fd4"],
  Maps: ["#f2efe6", "#e2ddd0"],
  News: ["#ffffff", "#eeeef0"],
  Bank: ["#253c66", "#14233f"],
}
export function favTile(ctx, name, x, y, s) {
  const [c0, c1] = FAV[name] || ["#888", "#666"]
  const r = s * 0.22
  ctx.save()
  ctx.shadowColor = "rgba(0,0,0,.16)"
  ctx.shadowBlur = s * 0.08
  ctx.shadowOffsetY = s * 0.025
  const g = ctx.createLinearGradient(0, y, 0, y + s)
  g.addColorStop(0, c0)
  g.addColorStop(1, c1)
  ctx.fillStyle = g
  roundRect(ctx, x, y, s, s, r)
  ctx.fill()
  ctx.restore()
  ctx.save()
  roundRect(ctx, x, y, s, s, r)
  ctx.clip()
  const u = s / 64
  const cx = x + s / 2
  const cy = y + s / 2
  ctx.lineJoin = "round"
  ctx.lineCap = "round"
  if (name === "Pinecrest") {
    // the HOA's crest: a ring, a pine of three tiers on its trunk
    ctx.strokeStyle = "rgba(255,255,255,.9)"
    ctx.lineWidth = 2 * u
    ctx.beginPath()
    ctx.arc(cx, cy, 22 * u, 0, Math.PI * 2)
    ctx.stroke()
    ctx.fillStyle = "#fff"
    for (const [ty, hw, hh] of [[-15, 7, 9], [-9, 10.5, 10], [-2, 14, 11]]) {
      ctx.beginPath()
      ctx.moveTo(cx, cy + ty * u)
      ctx.lineTo(cx + hw * u, cy + (ty + hh) * u)
      ctx.lineTo(cx - hw * u, cy + (ty + hh) * u)
      ctx.closePath()
      ctx.fill()
    }
    ctx.fillRect(cx - 2.2 * u, cy + 9 * u, 4.4 * u, 6 * u)
  } else if (name === "Council" || name === "Bank") {
    // a civic front: pediment, columns, steps (the bank's in gold, on a shield)
    const col = name === "Bank" ? "#e2bd62" : "#fff"
    ctx.fillStyle = col
    ctx.strokeStyle = col
    if (name === "Bank") {
      ctx.lineWidth = 1.8 * u
      ctx.beginPath()
      ctx.moveTo(cx, cy - 23 * u)
      ctx.lineTo(cx + 19 * u, cy - 16 * u)
      ctx.lineTo(cx + 17 * u, cy + 6 * u)
      ctx.quadraticCurveTo(cx + 12 * u, cy + 18 * u, cx, cy + 24 * u)
      ctx.quadraticCurveTo(cx - 12 * u, cy + 18 * u, cx - 17 * u, cy + 6 * u)
      ctx.lineTo(cx - 19 * u, cy - 16 * u)
      ctx.closePath()
      ctx.stroke()
    }
    const k = name === "Bank" ? 0.62 : 1
    const P = (px, py) => [cx + px * u * k, cy + (py * k + (name === "Bank" ? 1 : 0)) * u]
    ctx.beginPath()
    ctx.moveTo(...P(0, -19))
    ctx.lineTo(...P(20, -9))
    ctx.lineTo(...P(-20, -9))
    ctx.closePath()
    ctx.fill()
    ctx.fillRect(...P(-19, -7), 38 * u * k, 3 * u * k)
    for (let i = 0; i < 4; i++) ctx.fillRect(...P(-15 + i * 9.4, -3), 4.4 * u * k, 16 * u * k)
    ctx.fillRect(...P(-19, 14), 38 * u * k, 3 * u * k)
    ctx.fillRect(...P(-22, 18), 44 * u * k, 3 * u * k)
  } else if (name === "Weather") {
    const sg = ctx.createRadialGradient(cx - 7 * u, cy - 8 * u, 1, cx - 7 * u, cy - 8 * u, 14 * u)
    sg.addColorStop(0, "#fff3a8")
    sg.addColorStop(1, "#ffcc2e")
    ctx.fillStyle = sg
    ctx.beginPath()
    ctx.arc(cx - 7 * u, cy - 8 * u, 12 * u, 0, Math.PI * 2)
    ctx.fill()
    ctx.fillStyle = "#fff"
    ctx.shadowColor = "rgba(20,60,120,.25)"
    ctx.shadowBlur = 3 * u
    ctx.beginPath()
    ctx.arc(cx - 8 * u, cy + 8 * u, 8 * u, Math.PI * 0.5, Math.PI * 1.5)
    ctx.arc(cx + 1 * u, cy + 1 * u, 10.5 * u, Math.PI, Math.PI * 1.9)
    ctx.arc(cx + 12 * u, cy + 7 * u, 7 * u, Math.PI * 1.4, Math.PI * 0.5)
    ctx.closePath()
    ctx.fill()
  } else if (name === "Maps") {
    // a map: parkland, water, roads, and the pin
    ctx.fillStyle = "#cfe5c0"
    ctx.beginPath()
    ctx.moveTo(x, y + 34 * u)
    ctx.quadraticCurveTo(x + 18 * u, y + 30 * u, x + 26 * u, y + s)
    ctx.lineTo(x, y + s)
    ctx.fill()
    ctx.fillStyle = "#9fd0f4"
    ctx.beginPath()
    ctx.moveTo(x + 40 * u, y)
    ctx.quadraticCurveTo(x + 46 * u, y + 16 * u, x + s, y + 20 * u)
    ctx.lineTo(x + s, y)
    ctx.fill()
    ctx.strokeStyle = "#fff"
    ctx.lineWidth = 6 * u
    ctx.beginPath()
    ctx.moveTo(x - 2, y + 46 * u)
    ctx.lineTo(x + s + 2, y + 22 * u)
    ctx.moveTo(x + 20 * u, y - 2)
    ctx.lineTo(x + 30 * u, y + s + 2)
    ctx.stroke()
    ctx.strokeStyle = "#f6c544"
    ctx.lineWidth = 3.4 * u
    ctx.beginPath()
    ctx.moveTo(x - 2, y + 46 * u)
    ctx.lineTo(x + s + 2, y + 22 * u)
    ctx.stroke()
    const px = x + 40 * u
    const py = y + 30 * u
    ctx.fillStyle = "#ff3b30"
    ctx.shadowColor = "rgba(0,0,0,.25)"
    ctx.shadowBlur = 2.5 * u
    ctx.shadowOffsetY = 1 * u
    ctx.beginPath()
    ctx.arc(px, py - 6 * u, 8 * u, Math.PI * 0.82, Math.PI * 2.18)
    ctx.lineTo(px, py + 8 * u)
    ctx.closePath()
    ctx.fill()
    ctx.shadowColor = "transparent"
    ctx.fillStyle = "#fff"
    ctx.beginPath()
    ctx.arc(px, py - 6 * u, 3 * u, 0, Math.PI * 2)
    ctx.fill()
  } else if (name === "News") {
    // a folded paper: masthead, a headline, columns
    ctx.fillStyle = "#fbfbfc"
    ctx.strokeStyle = "rgba(0,0,0,.14)"
    ctx.lineWidth = 1
    roundRect(ctx, cx - 18 * u, cy - 21 * u, 36 * u, 42 * u, 3 * u)
    ctx.fill()
    ctx.stroke()
    ctx.fillStyle = "#e0393e"
    ctx.fillRect(cx - 14 * u, cy - 17 * u, 28 * u, 6 * u)
    ctx.fillStyle = "#2c2c30"
    ctx.fillRect(cx - 14 * u, cy - 7 * u, 28 * u, 3.2 * u)
    ctx.fillStyle = "#b9b9bf"
    for (let i = 0; i < 5; i++) {
      ctx.fillRect(cx - 14 * u, cy + (0 + i * 3.6) * u, 12.5 * u, 1.6 * u)
      ctx.fillRect(cx + 1.5 * u, cy + (0 + i * 3.6) * u, 12.5 * u, 1.6 * u)
    }
  }
  ctx.restore()
  // glass: a soft light across the top, and a hairline
  ctx.save()
  roundRect(ctx, x, y, s, s, r)
  ctx.clip()
  const gl = ctx.createLinearGradient(0, y, 0, y + s * 0.55)
  gl.addColorStop(0, "rgba(255,255,255,.22)")
  gl.addColorStop(1, "rgba(255,255,255,0)")
  ctx.fillStyle = gl
  ctx.fillRect(x, y, s, s * 0.55)
  ctx.restore()
  ctx.strokeStyle = "rgba(0,0,0,.1)"
  ctx.lineWidth = 1
  roundRect(ctx, x + 0.5, y + 0.5, s - 1, s - 1, r)
  ctx.stroke()
}
