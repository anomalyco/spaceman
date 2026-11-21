// Canvas 2D fallback. Same display list, drawn with clips and drawImage. Grading darkens a
// primitive by overdrawing its shape; shadows use a soft offset fill instead of shadowBlur.
import { roundRect } from "./apps.js"

// A tinted copy of a (white) texture, for glows.
const tints = new WeakMap()
function tinted(tex, col) {
  let m = tints.get(tex)
  if (!m) tints.set(tex, (m = new Map()))
  const key = col.slice(0, 3).map((v) => Math.round(v * 40)).join()
  let c = m.get(key)
  if (c) return c
  c = document.createElement("canvas")
  c.width = tex.width
  c.height = tex.height
  const x = c.getContext("2d")
  x.drawImage(tex, 0, 0)
  x.globalCompositeOperation = "source-in"
  x.fillStyle = `rgb(${(col[0] * 255) | 0},${(col[1] * 255) | 0},${(col[2] * 255) | 0})`
  x.fillRect(0, 0, c.width, c.height)
  m.set(key, c)
  return c
}

export function createCanvas2D(canvas) {
  const ctx = canvas.getContext("2d", { alpha: false })
  if (!ctx) return null
  return {
    kind: "canvas2d",
    lost: false,
    render(list, W, H, dpr, grade) {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      ctx.fillStyle = "#08080a"
      ctx.fillRect(0, 0, W, H)
      let clipKey = null
      for (const p of list) {
        const a = (p.a ?? 1) * (p.col?.[3] ?? 1)
        if (a < 0.003 || p.w <= 0 || p.h <= 0) continue
        if (p.x > W || p.y > H || p.x + p.w < 0 || p.y + p.h < 0) continue
        const key = p.clip ? p.clip.join() : null
        if (key !== clipKey) {
          if (clipKey) ctx.restore()
          if (key) {
            ctx.save()
            ctx.beginPath()
            ctx.rect(p.clip[0], p.clip[1], p.clip[2] - p.clip[0], p.clip[3] - p.clip[1])
            ctx.clip()
          }
          clipKey = key
        }
        const col = p.col || [1, 1, 1, 1]
        const r = p.r ?? 0
        if (p.shadow) {
          ctx.globalAlpha = a * 0.35
          ctx.fillStyle = "#000"
          roundRect(ctx, p.x - p.shadow * 0.2, p.y + p.shadow * 0.25, p.w + p.shadow * 0.4, p.h + p.shadow * 0.35, r + p.shadow * 0.3)
          ctx.fill()
          ctx.globalAlpha = a * 0.5
          roundRect(ctx, p.x, p.y + p.shadow * 0.15, p.w, p.h, r)
          ctx.fill()
          ctx.globalAlpha = 1
          continue
        }
        const css = `rgb(${(col[0] * 255) | 0},${(col[1] * 255) | 0},${(col[2] * 255) | 0})`
        ctx.globalAlpha = p.tex ? p.a ?? 1 : a
        if (p.ring) {
          ctx.strokeStyle = css
          ctx.globalAlpha = a
          ctx.lineWidth = p.ring
          roundRect(ctx, p.x + p.ring / 2, p.y + p.ring / 2, p.w - p.ring, p.h - p.ring, Math.max(0, r - p.ring / 2))
          ctx.stroke()
        } else if (p.tex) {
          if (!p.tex.width || !p.tex.height) continue
          const img = col[0] < 0.999 || col[1] < 0.999 || col[2] < 0.999 ? tinted(p.tex, col) : p.tex
          ctx.globalAlpha = (p.a ?? 1) * (col[3] ?? 1)
          if (r > 0.5) {
            ctx.save()
            roundRect(ctx, p.x, p.y, p.w, p.h, r)
            ctx.clip()
            ctx.drawImage(img, p.x, p.y, p.w, p.h)
            ctx.restore()
          } else ctx.drawImage(img, p.x, p.y, p.w, p.h)
        } else {
          ctx.fillStyle = css
          if (r > 0.5) {
            roundRect(ctx, p.x, p.y, p.w, p.h, r)
            ctx.fill()
          } else ctx.fillRect(p.x, p.y, p.w, p.h)
        }
        if (p.g && grade[0] > 0.01 && !p.ring && !p.glow) {
          ctx.globalAlpha = Math.min(1, grade[0] * p.g * (p.a ?? 1))
          ctx.fillStyle = grade[1] > 0.3 ? "#05070d" : "#000"
          if (r > 0.5) {
            roundRect(ctx, p.x, p.y, p.w, p.h, r)
            ctx.fill()
          } else ctx.fillRect(p.x, p.y, p.w, p.h)
        }
      }
      if (clipKey) ctx.restore()
      ctx.globalAlpha = 1
    },
    sweep() {},
    done: () => Promise.resolve(),
    destroy() {},
  }
}
