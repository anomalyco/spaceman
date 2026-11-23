// Critically damped springs (bounce 0) that keep their velocity when retargeted. A spring is a
// duration: w = 2π / d. Ported from the HUD track's bar.
const TAU = Math.PI * 2

export const sp = (x, d = 0.38) => ({ x, t: x, v: 0, w: TAU / d })
// bounce > 0 makes it underdamped (a pop that settles), until the next to() without one
export const to = (s, t, d, bounce) => {
  s.t = t
  if (d) s.w = TAU / d
  s.z = bounce ? 1 - bounce : undefined
}
export const jump = (s, x) => {
  s.x = s.t = x
  s.v = 0
}
export function step(s, dt) {
  const d = s.x - s.t
  if (Math.abs(d) < 0.0005 && Math.abs(s.v) < 0.0005) {
    s.x = s.t
    s.v = 0
    return
  }
  if (s.z !== undefined) {
    // semi-implicit Euler in small steps: stable for the short springs used here
    const n = Math.max(1, Math.ceil(dt / (1 / 240)))
    const h = dt / n
    for (let i = 0; i < n; i++) {
      s.v += (-s.w * s.w * (s.x - s.t) - 2 * s.z * s.w * s.v) * h
      s.x += s.v * h
    }
    return
  }
  const e = Math.exp(-s.w * dt)
  const k = (s.v + s.w * d) * dt
  s.x = s.t + (d + k) * e
  s.v = (s.v - s.w * k) * e
}

export const clamp = (x, a = 0, b = 1) => (x < a ? a : x > b ? b : x)
export const mix = (a, b, t) => a + (b - a) * t
export const smooth = (t) => {
  t = clamp(t)
  return t * t * (3 - 2 * t)
}
export const easeInOut = (t) => {
  t = clamp(t)
  return t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2
}
