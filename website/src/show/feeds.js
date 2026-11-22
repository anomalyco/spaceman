// Vigil, the camera app: each window is one security camera. A source frame (public/show/feeds/) goes
// through a cheap sensor at the camera's own resolution and frame rate: exposure hunting, a rolling
// wobble, sensor noise that is worst in the dark, 8 px compression blocks, the odd stale macroblock or
// dropped frame, and the camera's own burned-in name and clock. The result is upscaled soft into the
// window, so both renderers just see a window canvas that repaints at the camera's rate.
import { plates, coverUV } from "./plates.js"
import { MONO } from "./apps.js"

export const FEEDS = ["seq-walker", "seq-cat", "seq-alan", "cam-hall", "cam-kitchen", "cam-living", "cam-landing", "cam-back", "st-08", "st-10", "st-10-empty", "st-14", "st-16", "st-16-empty", "st-16-cat", "st-16-bins", "st-18", "cam-cellar", "sixteen"]
let clock = 0
// the film's own time, so a feed is the same frame at the same moment in every run
export function feedClock(t) {
  clock = t
}
export const feedTick = (d) => Math.floor(clock * (d.fps || 12))

const rnd = (n) => {
  const x = Math.sin(n * 127.1 + 311.7) * 43758.5453
  return x - Math.floor(x)
}
const noiseTiles = []
function noiseTile(k) {
  if (noiseTiles[k]) return noiseTiles[k]
  const c = document.createElement("canvas")
  c.width = c.height = 128
  const x = c.getContext("2d")
  const img = x.createImageData(128, 128)
  for (let i = 0; i < img.data.length; i += 4) {
    const v = Math.max(0, Math.min(255, 128 + (rnd(i * 0.731 + k * 9137) + rnd(i * 1.37 + k * 71) - 1) * 150))
    img.data[i] = img.data[i + 1] = img.data[i + 2] = v
    img.data[i + 3] = 255
  }
  x.putImageData(img, 0, 0)
  return (noiseTiles[k] = c)
}
function canvasOf(o, key, w, h) {
  let c = o[key]
  if (!c) c = o[key] = document.createElement("canvas")
  if (c.width !== w || c.height !== h) {
    c.width = w
    c.height = h
  }
  return c
}

const pad = (n) => String(n).padStart(2, "0")
// a stamp in the camera's own format; base is seconds after midnight on the night of the interview
function stamp(d, t) {
  // no two cameras' clocks quite agree
  const s = Math.floor((d.clock ?? 23 * 3600 + 47 * 60 + 2) + t + ((d.seed || 0) % 5) - 2)
  const hh = Math.floor(s / 3600) % 24
  const mm = Math.floor(s / 60) % 60
  const ss = s % 60
  return (d.wd ? d.wd + " " : "") + (d.us ? `${d.date || "11-20-2026"}  ${pad(((hh + 11) % 12) + 1)}:${pad(mm)}:${pad(ss)} ${hh < 12 ? "AM" : "PM"}` : `${d.date || "2026-11-20"} ${pad(hh)}:${pad(mm)}:${pad(ss)}`)
}

// d: { src, name, ir, q (0 new .. 1 old and cheap), res: [w, h], fps, seed, clock + date (the stamp),
//      lost: [[t0, t1]] (reconnecting), unlit: [[x0, y0, x1, y1]] (a light gone off),
//      life: [{ moth } | { sweep, every } | { flick: [x0, y0, x1, y1], every } | { leaves }] }
export function feed(ctx, x, y, w, h, d, win) {
  const st = (win._feed ||= {})
  const seed = d.seed || 1
  const q = d.q ?? 0.4
  // the sensor is framed to the window, so the camera's own text is always in view
  const SW = d.res?.[0] || (q > 0.6 ? 320 : 400)
  const SH = Math.round((SW * Math.min(1.2, Math.max(0.4, h / Math.max(1, w)))) / 4) * 4
  const fps = d.fps || 12
  let f = Math.floor(clock * fps)
  // dropped frames: an older camera sometimes holds a frame for one more tick
  if (rnd(f * 3.1 + seed) < 0.05 + q * 0.12 && st.frame) f = st.f
  const lost = (d.lost || []).some(([a, b]) => clock >= a && clock < b)
  // a new date on the stamp (the story moves on a day) shows at once, not on the next frame
  if ((st.f !== f || st.sh !== SH || st.date !== d.date) && !lost) {
    st.date = d.date
    st.sh = SH
    st.f = f
    const t = f / fps
    const s = canvasOf(st, "sensor", SW, SH)
    const g = s.getContext("2d")
    const src0 = (d.seq && seqFrame(d.seq)) || plates[d.src]
    // a camera can see part of a source (a second camera on the same house, zoomed on the door)
    const src = src0 && d.crop ? cropOf(src0, d.crop) : src0
    g.globalCompositeOperation = "source-over"
    g.globalAlpha = 1
    g.fillStyle = "#000"
    g.fillRect(0, 0, SW, SH)
    // auto-exposure hunting, slow and slightly too much
    const ex = 1 + (0.05 + q * 0.05) * Math.sin(t * 0.55 + seed) + 0.025 * Math.sin(t * 1.9 + seed * 2)
    if (src) {
      const [u0, v0, u1, v1] = coverUV(src, SW, SH, 0.5, 0.5)
      // a rolling-shutter wobble: the frame slides a fraction of a pixel, a band at a time
      const bands = 6
      for (let b = 0; b < bands; b++) {
        const dx = (rnd(f * 0.7 + b * 13 + seed) - 0.5) * (0.4 + q * 0.8)
        const sy = (b / bands) * SH
        const sh = SH / bands
        const vv0 = v0 + (v1 - v0) * (b / bands)
        const vv1 = v0 + (v1 - v0) * ((b + 1) / bands)
        g.globalAlpha = Math.min(1, ex)
        g.drawImage(src, u0 * src.width, vv0 * src.height, (u1 - u0) * src.width, (vv1 - vv0) * src.height, dx, sy, SW, sh + 0.5)
      }
      if (ex > 1) {
        g.globalCompositeOperation = "lighter"
        g.globalAlpha = (ex - 1) * 1.6
        g.drawImage(s, 0, 0)
        g.globalCompositeOperation = "source-over"
      }
    }
    g.globalAlpha = 1
    // a light that has gone off since the source was made
    for (const [x0, y0, x1, y1] of d.unlit || []) {
      g.fillStyle = "rgba(9,9,11,.94)"
      g.fillRect(x0 * SW, y0 * SH, (x1 - x0) * SW, (y1 - y0) * SH)
    }
    life(g, SW, SH, d, t, seed)
    // a cheap wide lens: the corners fall off
    const vg = g.createRadialGradient(SW / 2, SH / 2, Math.min(SW, SH) * 0.35, SW / 2, SH / 2, Math.hypot(SW, SH) * 0.56)
    vg.addColorStop(0, "rgba(0,0,0,0)")
    vg.addColorStop(1, `rgba(0,0,0,${0.45 + q * 0.25})`)
    g.fillStyle = vg
    g.fillRect(0, 0, SW, SH)
    // sensor noise: a grain over everything, and a floor that lifts the blacks
    const n = noiseTile(Math.floor(rnd(f + seed * 7) * 6))
    const ox = -Math.floor(rnd(f * 1.3 + seed) * 64)
    const oy = -Math.floor(rnd(f * 2.1 + seed) * 64)
    g.globalCompositeOperation = "overlay"
    g.globalAlpha = 0.16 + q * 0.18
    for (let yy = oy; yy < SH; yy += 128) for (let xx = ox; xx < SW; xx += 128) g.drawImage(n, xx, yy)
    g.globalCompositeOperation = "lighter"
    g.globalAlpha = 0.035 + q * 0.04
    for (let yy = oy; yy < SH; yy += 128) for (let xx = ox; xx < SW; xx += 128) g.drawImage(n, xx, yy)
    if (d.ir) {
      g.globalCompositeOperation = "saturation"
      g.globalAlpha = 1
      g.fillStyle = "#808080"
      g.fillRect(0, 0, SW, SH)
    }
    // compression: the frame averaged in 8 px blocks, laid back over itself
    const bw = Math.ceil(SW / 8)
    const bh = Math.ceil(SH / 8)
    const blk = canvasOf(st, "blk", bw, bh)
    const bg = blk.getContext("2d")
    bg.imageSmoothingQuality = "low"
    bg.drawImage(s, 0, 0, bw, bh)
    g.globalCompositeOperation = "source-over"
    g.imageSmoothingEnabled = false
    g.globalAlpha = 0.1 + q * 0.16
    g.drawImage(blk, 0, 0, bw * 8, bh * 8)
    // now and then a stale macroblock row from the last frame, shifted
    const prev = st.prev
    if (prev && rnd(f * 5.3 + seed) < 0.03 + q * 0.07) {
      const by = Math.floor(rnd(f * 9.1 + seed) * (SH / 16)) * 16
      const bx = Math.floor(rnd(f * 4.7 + seed) * (SW / 16)) * 16
      const bwid = (2 + Math.floor(rnd(f + 3) * 6)) * 16
      g.globalAlpha = 0.9
      g.drawImage(prev, bx, by, bwid, 16, bx + (rnd(f * 8.3) > 0.5 ? 2 : -2), by + 1, bwid, 16)
    }
    g.imageSmoothingEnabled = true
    g.globalAlpha = 1
    // the camera's own text, at the sensor's resolution
    const fs = Math.round(SW * 0.029)
    g.font = `600 ${fs}px ${MONO}`
    g.textBaseline = "top"
    g.lineWidth = Math.max(1.5, fs * 0.16)
    g.strokeStyle = "rgba(0,0,0,.85)"
    g.fillStyle = "rgba(245,245,240,.95)"
    const name = d.name || ""
    const tstr = stamp(d, clock)
    const m = Math.round(fs * 0.6)
    g.textAlign = "left"
    g.strokeText(name, m, m)
    g.fillText(name, m, m)
    g.textAlign = "right"
    g.strokeText(tstr, SW - m, m)
    g.fillText(tstr, SW - m, m)
    if (d.ch) {
      g.textAlign = "left"
      g.strokeText(d.ch, m, SH - m - fs)
      g.fillText(d.ch, m, SH - m - fs)
    }
    const prevC = canvasOf(st, "prev", SW, SH)
    prevC.getContext("2d").drawImage(s, 0, 0)
    st.frame = true
  }
  // into the window, upscaled soft
  ctx.fillStyle = "#0b0c0d"
  ctx.fillRect(x, y, w, h)
  const s = st.sensor
  if (s) {
    let [u0, v0, u1, v1] = coverUV(s, w, h, 0.5, 0.5)
    // the viewer's digital zoom: a smaller part of the same low-resolution stream, blown up
    const z = d.zoom?.z || 1
    if (z > 1.001) {
      const uw = (u1 - u0) / z
      const vh = (v1 - v0) / z
      const cu = Math.min(u1 - uw / 2, Math.max(u0 + uw / 2, u0 + (u1 - u0) * d.zoom.x))
      const cv = Math.min(v1 - vh / 2, Math.max(v0 + vh / 2, v0 + (v1 - v0) * d.zoom.y))
      u0 = cu - uw / 2
      u1 = cu + uw / 2
      v0 = cv - vh / 2
      v1 = cv + vh / 2
    }
    ctx.imageSmoothingEnabled = z < 2.6
    ctx.imageSmoothingQuality = "low"
    ctx.globalAlpha = lost ? 0.55 : 1
    ctx.drawImage(s, u0 * s.width, v0 * s.height, (u1 - u0) * s.width, (v1 - v0) * s.height, x, y, w, h)
    ctx.globalAlpha = 1
  }
  // the app's own marks, not the camera's: live, or reconnecting
  const fs = Math.max(10, Math.min(13, w * 0.024))
  ctx.font = `600 ${fs}px ${MONO}`
  ctx.textBaseline = "alphabetic"
  ctx.textAlign = "right"
  const by = y + h - 10
  if (lost) {
    ctx.fillStyle = "rgba(0,0,0,.55)"
    ctx.fillRect(x, y + h / 2 - 16, w, 32)
    ctx.fillStyle = "rgba(235,235,230,.9)"
    ctx.textAlign = "center"
    ctx.fillText("Reconnecting…", x + w / 2, y + h / 2 + fs * 0.35)
  } else {
    ctx.fillStyle = "rgba(0,0,0,.4)"
    ctx.fillRect(x + w - fs * 4.6 - 10, by - fs - 3, fs * 4.6 + 4, fs + 8)
    ctx.fillStyle = "rgba(235,235,230,.92)"
    ctx.fillText("LIVE", x + w - 10, by)
    ctx.fillStyle = Math.floor(clock * 1.2) % 2 ? "#e0483c" : "#a3362d"
    ctx.beginPath()
    ctx.arc(x + w - fs * 3.4 - 10, by - fs * 0.36, fs * 0.3, 0, Math.PI * 2)
    ctx.fill()
  }
  if (d.zoom?.z > 1.05 && !lost) {
    ctx.textAlign = "left"
    ctx.fillStyle = "rgba(0,0,0,.4)"
    const zs = "ZOOM " + d.zoom.z.toFixed(1) + "×"
    const zw = ctx.measureText(zs).width
    ctx.fillRect(x + 6, by - fs - 3, zw + 8, fs + 8)
    ctx.fillStyle = "rgba(235,235,230,.92)"
    ctx.fillText(zs, x + 10, by)
  }
  // a snapshot, saved, its name being edited
  if (d.snap) {
    const sw = Math.min(w - 24, 330)
    const sx = x + 12
    const sy = y + h - 96
    ctx.fillStyle = "rgba(28,29,32,.92)"
    roundRectPath(ctx, sx, sy, sw, 64, 9)
    ctx.fill()
    ctx.font = `500 11.5px ${MONO}`
    ctx.fillStyle = "rgba(200,198,192,.85)"
    ctx.textAlign = "left"
    ctx.fillText("Snapshot saved to Stills", sx + 12, sy + 20)
    ctx.fillStyle = d.snap.edit ? "rgba(255,255,255,.08)" : "rgba(0,0,0,0)"
    ctx.fillRect(sx + 9, sy + 29, sw - 18, 25)
    ctx.font = `500 14px ${MONO}`
    const nm = d.snap.name
    const nw = ctx.measureText(nm).width
    if (d.snap.sel) {
      ctx.fillStyle = "rgba(70,120,220,.75)"
      ctx.fillRect(sx + 13, sy + 33, nw + 2, 18)
    }
    ctx.fillStyle = "rgba(240,238,232,.96)"
    ctx.fillText(nm, sx + 14, sy + 47)
    if (d.snap.edit && !d.snap.sel && Math.floor(clock * 2) % 2 === 0) ctx.fillRect(sx + 15 + nw, sy + 34, 1.5, 16)
  }
  ctx.textAlign = "left"
}

function roundRectPath(ctx, x, y, w, h, r) {
  ctx.beginPath()
  ctx.roundRect(x, y, w, h, r)
}

const crops = new Map()
function cropOf(img, [x, y, w, h]) {
  const k = img.version + ":" + [x, y, w, h].join(",")
  let c = crops.get(img)?.[k]
  if (c) return c
  c = document.createElement("canvas")
  c.width = Math.max(8, Math.round(img.width * w))
  c.height = Math.max(8, Math.round(img.height * h))
  c.getContext("2d").drawImage(img, img.width * x, img.height * y, c.width, c.height, 0, 0, c.width, c.height)
  if (!crops.has(img)) crops.set(img, {})
  crops.get(img)[k] = c
  return c
}

// A clip: generated footage (show/feeds/clips, notes/feeds.md) as a sheet of low-res frames,
// 6 to a row. seq: { name, n, fps, t0, loop: "hold" | "pingpong", from, end }. Before t0 it shows
// its first frame; "hold" keeps the last frame for a moment, then the first (she has walked on).
const cells = new Map()
function seqFrame(q) {
  const sheet = plates["seq-" + q.name]
  if (!sheet) return null
  const n = q.n
  const from = q.from || 0
  let i = Math.floor((clock - (q.t0 ?? 0)) * q.fps)
  if (i < 0) i = 0
  else if (i >= n && q.loop === "pingpong") {
    // once through, then back and forth over from..n-1
    const half = n - 1 - from
    const p = (i - (n - 1)) % (2 * half)
    i = p <= half ? n - 1 - p : from + (p - half)
  } else if (i >= n) i = q.end != null && clock > q.t0 + n / q.fps + q.end ? 0 : n - 1
  const key = q.name + i
  let c = cells.get(key)
  if (c) return c
  const cw = sheet.width / 6
  const ch = cw * (9 / 16)
  c = document.createElement("canvas")
  c.width = Math.round(cw)
  c.height = Math.round(ch)
  c.getContext("2d").drawImage(sheet, (i % 6) * cw, Math.floor(i / 6) * ch, cw, ch, 0, 0, c.width, c.height)
  c.version = 1
  cells.set(key, c)
  return c
}

// The dog walker's track through the door camera's frame (measured from the clip): where she and
// the dog are at time t, x and y in 0..1 of the frame.
export function walkerAt(l, t) {
  const u = (t - l.t0) / l.dur
  return { u, x: l.x0 + (l.x1 - l.x0) * u, y: l.y }
}

// small life in a fixed frame: a moth at the IR lamp, a car's lights crossing, a light on a timer
function life(g, W, H, d, t, seed) {
  for (const l of d.life || []) {
    if (l.moth) {
      const k = t * 0.9 + seed
      const mx = W * (l.moth[0] + 0.07 * Math.sin(k * 2.3) + 0.04 * Math.sin(k * 7.1))
      const my = H * (l.moth[1] + 0.06 * Math.cos(k * 1.7) + 0.03 * Math.sin(k * 9.3))
      const r = H * (0.012 + 0.006 * Math.sin(k * 23))
      const gr = g.createRadialGradient(mx, my, 0, mx, my, r * 3)
      gr.addColorStop(0, "rgba(255,255,255,.95)")
      gr.addColorStop(0.35, "rgba(255,255,255,.4)")
      gr.addColorStop(1, "rgba(255,255,255,0)")
      g.globalCompositeOperation = "lighter"
      g.fillStyle = gr
      g.fillRect(mx - r * 3, my - r * 3, r * 6, r * 6)
      g.globalCompositeOperation = "source-over"
    }
    if (l.sweep) {
      const every = l.every || 23
      const u = ((t + seed * 5) % every) / 3.2
      if (u < 1) {
        const sx = W * (-0.3 + u * 1.6) * (l.dir || 1) + (l.dir < 0 ? W : 0)
        const gr = g.createRadialGradient(sx, H * (l.sweep[1] ?? 0.75), 0, sx, H * (l.sweep[1] ?? 0.75), W * 0.45)
        const a = Math.sin(u * Math.PI) * (l.a || 0.35)
        gr.addColorStop(0, `rgba(255,250,235,${a})`)
        gr.addColorStop(1, "rgba(255,250,235,0)")
        g.globalCompositeOperation = "lighter"
        g.fillStyle = gr
        g.fillRect(0, 0, W, H)
        g.globalCompositeOperation = "source-over"
      }
    }
    if (l.flick) {
      const on = Math.floor((t + seed) / (l.every || 9)) % 2 === 0
      if (!on) {
        const [x0, y0, x1, y1] = l.flick
        g.fillStyle = l.off || "rgba(14,13,12,.92)"
        g.fillRect(x0 * W, y0 * H, (x1 - x0) * W, (y1 - y0) * H)
      }
    }
    if (l.leaves) {
      g.fillStyle = "rgba(20,20,20,.75)"
      for (let i = 0; i < l.leaves; i++) {
        const p = (t * (0.05 + rnd(i) * 0.06) + rnd(i * 3)) % 1
        g.fillRect(W * ((rnd(i * 7) + p * 0.5) % 1), H * (0.55 + 0.4 * ((rnd(i * 5) + p * 0.3) % 1)), 2, 1.5)
      }
    }
  }
}
