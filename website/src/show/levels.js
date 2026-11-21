// The last shots are photographs: his study, with his screen composited onto its monitor, and the
// kitchen across the street where the recorder sits. Coordinates are in plate pixels or fractions.
import { clamp } from "./springs.js"

export const STUDY = {
  // the monitor's panel, inside the dark border painted into the plate
  screen: { x: 642, y: 515, w: 529, h: 313 },
  focus: { x: 0.5, y: 0.5 },
  tallFocus: { x: 0.43, y: 0.47 },
  // No. 14's lit window, across the street, through his study window
  window: { x: 0.702, y: 0.381 },
}

export const KITCHEN = {
  focus: { x: 0.5, y: 0.56 },
  tallFocus: { x: 0.3, y: 0.66 },
  recorder: { x: 0.3, y: 0.74 },
  light: { x: 0.19, y: 0.736 },
}

// The kitchen floor, from the door (floor.jpg), and afterwards (after.jpg): same camera.
export const FLOOR = {
  floor: { f0: { x: 0.5, y: 0.52 }, f1: { x: 0.5, y: 0.6 }, tall0: { x: 0.5, y: 0.6 }, tall1: { x: 0.5, y: 0.64 }, z0: 1.0, z1: 1.18, focus: { x: 0.5, y: 0.62, w: 0.16, h: 0.24 } },
  after: { f0: { x: 0.5, y: 0.58 }, f1: { x: 0.5, y: 0.76 }, tall0: { x: 0.5, y: 0.72 }, tall1: { x: 0.5, y: 0.82 }, z0: 1.02, z1: 1.3, focus: { x: 0.5, y: 0.84, w: 0.3, h: 0.12 } },
}

// The plate-to-CSS transform that covers a W x H viewport at zoom z, centred on f (kept inside).
export function coverView(img, W, H, f, z = 1) {
  const k = Math.max(W / img.width, H / img.height) * z
  const vw = W / k
  const vh = H / k
  const cx = clamp(f.x * img.width, vw / 2, img.width - vw / 2)
  const cy = clamp(f.y * img.height, vh / 2, img.height - vh / 2)
  return { k, ox: W / 2 - cx * k, oy: H / 2 - cy * k }
}

// From one framing to another as a single zoom about the point both framings agree on, so
// everything on screen moves along a straight line away from (or toward) it. e is eased progress.
export function zoomT(A, B, e) {
  if (e <= 0) return A
  if (e >= 1) return B
  const lk = Math.log(B.k / A.k)
  if (Math.abs(lk) < 0.02) return { k: A.k + (B.k - A.k) * e, ox: A.ox + (B.ox - A.ox) * e, oy: A.oy + (B.oy - A.oy) * e }
  const k = A.k * Math.exp(lk * e)
  const px = (B.ox - A.ox) / (A.k - B.k)
  const py = (B.oy - A.oy) / (A.k - B.k)
  return { k, ox: A.k * px + A.ox - k * px, oy: A.k * py + A.oy - k * py }
}

let glowTex = null
export function glowCanvas() {
  if (glowTex) return glowTex
  const c = (glowTex = document.createElement("canvas"))
  c.width = c.height = 256
  const ctx = c.getContext("2d")
  const g = ctx.createRadialGradient(128, 128, 0, 128, 128, 128)
  g.addColorStop(0, "rgba(255,255,255,1)")
  g.addColorStop(0.35, "rgba(255,255,255,.45)")
  g.addColorStop(1, "rgba(255,255,255,0)")
  ctx.fillStyle = g
  ctx.fillRect(0, 0, 256, 256)
  c.version = 1
  return c
}
