// The show engine. createShow() draws the still with Canvas 2D (nothing else is loaded); start()
// runs inside the key press: it brings up WebGPU (or Canvas 2D), fetches the voice bank and plays
// the film. ?capture exposes window.__show for deterministic frame stepping.
import { World, MENU_H } from "./world.js"
import { build, pile, HOLD, END } from "./director.js"
import { createCanvas2D } from "./canvas2d.js"
import { createGPU } from "./gpu.js"
import { Audio } from "./audio.js"
import { roomPrims, headPrims, frontPrims, makeOuter, stampCanvas, Z_HEAD, Z_FRONT } from "./levels.js"
import { clamp, mix, smooth, easeInOut } from "./springs.js"

const LINES = {
  march: "I'd had some of those windows open since March.",
  "same-note": "Three of them were the same note. I closed two.",
  mail: "Mail went to two. The internet went to three.",
  rule: "Then I wrote a rule. Chatter opens on two.",
  "not-closed": "It isn't closed. It's on two.",
  drill: "I returned the drill. I'd had it four years. He said I didn't need to.",
  house: "The house has a workspace.",
  minutes: "Nobody wanted to take the minutes. So I take the minutes.",
  hedge: "Number fourteen's hedge is six foot two. The limit is six feet.",
  second: "I started recording the height after the second conversation.",
  circulated: "Nobody corrected the minutes. I circulated them again.",
  space: "Everyone gets the same amount of space.",
  light: "Number fourteen leaves a light on.",
}

export function createShow({ still, canvas, base, capture = false, reduced = false, on = {} }) {
  const emit = (k, v) => on[k]?.(v)
  let cssW = 0
  let cssH = 0
  let SW = 1440
  let SH = 900
  let world = null
  let outer = null
  let renderer = null
  let phase = "still"
  let t = 0
  let cues = []
  let spans = []
  let ci = 0
  let raf = 0
  let last = 0
  let visible = true
  let dprScale = 1
  let caption = null
  let manifest = null
  const cam = { D: 1, cx: 0, cy: 0 }
  const grade = [0, 0, 0]
  const pull = { a: 0, b: 0, from: null }
  const audio = new Audio(base + "audio/", { log: capture })
  const stats = { intervals: [], cpu: [], frames: 0 }

  function measure() {
    const r = (canvas || still).getBoundingClientRect()
    cssW = Math.max(1, r.width)
    cssH = Math.max(1, r.height)
  }

  function logical() {
    measure()
    SW = clamp(Math.round(cssW * 1.25), 600, 1440)
    SH = Math.round(clamp((SW * cssH) / cssW, 0.5 * SW, 2.2 * SW))
    cam.cx = SW / 2
    cam.cy = SH / 2
  }

  function makeWorld() {
    const w = new World(SW, SH, { reduced })
    w.clock = "Tue Jun 2  11:48 PM"
    w.frontApp = "Jot"
    for (const def of pile(SW, SH)) w.add(def)
    w.setFocus("notes")
    w.cursor.x.x = w.cursor.x.t = SW * 0.7
    w.cursor.y.x = w.cursor.y.t = SH * 0.62
    return w
  }

  const dpr = () => Math.min(window.devicePixelRatio || 1, 2) * dprScale
  function fit() {
    const k = Math.min(cssW / SW, cssH / SH)
    return { k, ox: (cssW - SW * k) / 2, oy: (cssH - SH * k) / 2 }
  }

  function sizeCanvas(c) {
    let d = dpr()
    const px = cssW * cssH * d * d
    if (px > 3.6e6) d *= Math.sqrt(3.6e6 / px)
    const w = Math.round(cssW * d)
    const h = Math.round(cssH * d)
    if (c.width !== w || c.height !== h) {
      c.width = w
      c.height = h
    }
    return d
  }

  function setRes(w, k, d) {
    const r = Math.round(clamp(k * d, 0.5, 4) * 4) / 4
    if (w.res !== r) {
      w.res = r
      for (const x of w.wins.values()) x.dirty = true
      w.menuCanvas = null
      w.keys.canvas = null
    }
  }

  // ── Projection ─────────────────────────────────────────────────────────
  const PHOTO_H = 4.6 // the photographed part of the house front, in screen heights
  const D_END = Z_FRONT + PHOTO_H

  function camera() {
    if (pull.a <= 0) return { D: cam.D, cx: cam.cx, cy: cam.cy }
    if (!pull.from) pull.from = { ...cam }
    const f = pull.from
    const e = reduced ? stepped(pull.a) : easeInOut(pull.a)
    const D = Math.exp(mix(Math.log(f.D), Math.log(D_END), e))
    const lc = smooth(clamp((Math.log(D) - Math.log(f.D)) / (Math.log(1.6) - Math.log(f.D) || 1)))
    return { D, cx: mix(f.cx, SW / 2, lc), cy: mix(f.cy, SH / 2, lc) }
  }
  // Reduced motion: three held framings with cuts between them instead of a continuous zoom.
  const stepped = (u) => (u < 0.3 ? 0 : u < 0.62 ? 0.55 : 1)

  // Plane at depth z → CSS px, given the frame→px transform F.
  function planeT(z, c, F) {
    const s = 1 / (c.D - z)
    return { k: s * F.k, ox: (SW / 2 - c.cx * s) * F.k + F.ox, oy: (SH / 2 - c.cy * s) * F.k + F.oy, s }
  }

  function pushPrims(list, prims, T, opts) {
    for (const p of prims) list.push({ ...p, x: p.x * T.k + T.ox, y: p.y * T.k + T.oy, w: p.w * T.k, h: p.h * T.k, r: (p.r || 0) * T.k, ring: (p.ring || 0) * T.k, a: (p.a ?? 1) * opts.am, clip: opts.clip, g: opts.g })
  }

  const photoRegion = () => ({ x: SW / 2 - (PHOTO_H * SW) / 2, y: SH / 2 - (PHOTO_H * SH) / 2, w: PHOTO_H * SW, h: PHOTO_H * SH })

  function inner(list, F, clip, am) {
    const c = camera()
    const T0 = planeT(0, c, F)
    const light = 1 - grade[0] * 0.6
    const opts = { clip, g: 1, am }
    if (c.D > 1.001) pushPrims(list, roomPrims(SW, SH, light), T0, { ...opts, g: 0 })
    setRes(world, F.k * T0.s, dpr())
    world.draw(list, T0, { clip, g: 1, am })
    if (c.D - Z_HEAD > 0.15) {
      const a = clamp((c.D - Z_HEAD - 0.15) / 0.55)
      if (a > 0) pushPrims(list, headPrims(SW, SH, light), planeT(Z_HEAD, c, F), { clip, g: 0, am: am * a })
    }
    if (c.D - Z_FRONT > 0.05) {
      const TF = planeT(Z_FRONT, c, F)
      const fr = frontPrims(SW, SH, photoRegion(), light, TF.s)
      pushPrims(list, fr.prims, TF, { clip, g: 0, am })
    }
  }

  function buildList() {
    const list = []
    const F = fit()
    if (pull.b <= 0 || !outer) {
      inner(list, F, null, 1)
      if (F.ox > 0.5 || F.oy > 0.5) letterbox(list, F)
      return list
    }
    // Stage B: the other desktop, zooming out from the photograph.
    const pw = outer.wins.get("o-photo")
    const pr = outer.photoRect
    const u = reduced ? (pull.b < 0.5 ? 0 : 1) : easeInOut(pull.b)
    const zFull = SW / pr.w
    const z = Math.exp(mix(Math.log(zFull), 0, u))
    const cx = mix(pr.x + pr.w / 2, SW / 2, u)
    const cy = mix(pr.y + pr.h / 2, SH / 2, u)
    const TO = { k: z * F.k, ox: (SW / 2 - cx * z) * F.k + F.ox, oy: (SH / 2 - cy * z) * F.k + F.oy }
    setRes(outer, TO.k, dpr())
    outer.draw(list, TO, { g: 0, part: "base" })
    const off = pw.off.x
    const px = { x: (pr.x + off) * TO.k + TO.ox, y: pr.y * TO.k + TO.oy, w: pr.w * TO.k, h: pr.h * TO.k }
    const FI = { k: px.w / SW, ox: px.x, oy: px.y }
    const clip = [px.x, px.y, px.x + px.w, px.y + px.h]
    inner(list, FI, clip, pw.a.x)
    const st = stampCanvas("11/20/2026  11:48 PM")
    const sh = px.h * 0.045
    list.push({ x: px.x + px.w - sh * 9.4, y: px.y + px.h - sh * 1.7, w: sh * 8.1, h: sh, tex: st, a: clamp(pull.b * 3) * pw.a.x, clip })
    outer.draw(list, TO, { g: 0, part: "top" })
    if (F.ox > 0.5 || F.oy > 0.5) letterbox(list, F)
    return list
  }

  function letterbox(list, F) {
    const col = [0.03, 0.03, 0.035, 1]
    if (F.ox > 0.5) {
      list.push({ x: 0, y: 0, w: F.ox, h: cssH, col })
      list.push({ x: cssW - F.ox, y: 0, w: F.ox, h: cssH, col })
    }
    if (F.oy > 0.5) {
      list.push({ x: 0, y: 0, w: cssW, h: F.oy, col })
      list.push({ x: 0, y: cssH - F.oy, w: cssW, h: F.oy, col })
    }
  }

  // ── Clock ──────────────────────────────────────────────────────────────
  const S = {
    get world() {
      return world
    },
    get SW() {
      return SW
    },
    get SH() {
      return SH
    },
    cam,
    grade,
    pull,
    say(at, id) {
      cues.push({ t: at, fn: () => speak(id, at) })
    },
    sfx(name, gain = 1) {
      audio.play("sfx-" + name + ".mp3", { t, gain })
    },
    room(v, tc) {
      roomLevel = v
      audio.roomLevel(v, t, tc)
    },
    hint(v) {
      emit("hint", v)
    },
    outerKey() {
      audio.play("sfx-key2.mp3", { t, gain: 0.8 })
      if (outer) outer.switchTo(6, { quiet: true })
    },
    end() {
      phase = "ended"
      emit("caption", null)
      audio.stopAll()
      emit("phase", "ended")
    },
  }
  let roomLevel = 0

  function speak(id, at) {
    const v = audio.voiceFile(id)
    const dur = v?.dur ?? LINES[id].split(" ").length * 0.38
    caption = { id, text: LINES[id], t0: at, t1: at + dur + 0.5, words: v?.words || null }
    emit("caption", caption)
    audio.play(v?.file || id + ".mp3", { t, kind: "voice" })
  }

  function setup() {
    world = makeWorld()
    outer = null
    t = 0
    ci = 0
    grade.fill(0)
    pull.a = pull.b = 0
    pull.from = null
    cam.D = 1
    cam.cx = SW / 2
    cam.cy = SH / 2
    cues = []
    spans = []
    const b = build(S)
    cues = [...cues, ...b.cues].sort((x, y) => x.t - y.t)
    spans = b.spans
  }

  function advance(dt) {
    if (phase !== "live") return
    const t1 = t + dt
    // substeps keep springs and cues in lockstep at any frame rate
    const n = Math.max(1, Math.ceil(dt / (1 / 60)))
    const h = dt / n
    for (let i = 0; i < n; i++) {
      t += h
      while (ci < cues.length && cues[ci].t <= t) cues[ci++].fn()
      for (const s of spans) if (t >= s.t0 && t - h < s.t1) s.fn(clamp((t - s.t0) / (s.t1 - s.t0)), t)
      if (pull.a >= 0.995 && !outer) outer = makeOuter(SW, SH, world.res)
      if (pull.b <= 0) world.update(h)
      else outer?.update(h)
      if (phase !== "live") break
    }
    t = Math.max(t, Math.min(t1, END))
    if (caption && t > caption.t1) {
      caption = null
      emit("caption", null)
    }
  }

  function render() {
    if (!renderer) return
    const c0 = performance.now()
    const d = sizeCanvas(canvas)
    const list = phase === "ended" ? [] : buildList()
    renderer.render(list, cssW, cssH, d, grade)
    renderer.sweep()
    stats.cpu.push(performance.now() - c0)
    if (stats.cpu.length > 600) stats.cpu.shift()
  }

  function frame(now) {
    raf = 0
    const dt = last ? Math.min(0.05, (now - last) / 1000) : 1 / 60
    if (last) {
      stats.intervals.push(now - last)
      if (stats.intervals.length > 600) stats.intervals.shift()
    }
    last = now
    advance(dt)
    render()
    adapt()
    schedule()
  }

  // If frames run long for a while, drop the pixel ratio a step.
  let slow = 0
  function adapt() {
    const n = stats.intervals.length
    if (n < 2) return
    const iv = stats.intervals[n - 1]
    slow = iv > 24 ? slow + 1 : Math.max(0, slow - 1)
    if (slow > 40 && dprScale > 0.55) {
      dprScale -= 0.15
      slow = 0
    }
  }

  function schedule() {
    if (raf || capture || !visible || phase !== "live") return
    raf = requestAnimationFrame(frame)
  }

  function setVisible(v) {
    if (v === visible) return
    visible = v
    audio.setHidden(!v)
    last = 0
    if (v) schedule()
    else if (raf) {
      cancelAnimationFrame(raf)
      raf = 0
    }
  }
  const onVis = () => setVisible(!document.hidden && inView)
  let inView = true
  document.addEventListener("visibilitychange", onVis)
  const io = new IntersectionObserver((es) => {
    inView = es[0].isIntersecting
    onVis()
  })
  io.observe(canvas)
  const ro = new ResizeObserver(() => {
    measure()
    if (phase === "still") drawStill()
    else if (phase !== "live") render()
  })
  ro.observe(canvas)

  // ── The still ──────────────────────────────────────────────────────────
  let stillR = null
  function drawStill() {
    if (!still) return
    measure()
    stillR ||= createCanvas2D(still)
    const d = sizeCanvas(still)
    setRes(world, fit().k, d)
    const list = []
    world.draw(list, fit(), { g: 1 })
    letterbox(list, fit())
    stillR.render(list, cssW, cssH, d, [0.08, 0.3, 0])
  }

  logical()
  setup()
  drawStill()

  const api = {
    get phase() {
      return phase
    },
    get renderer() {
      return renderer?.kind ?? null
    },
    get time() {
      return t
    },
    async start({ context = null, muted = false } = {}) {
      if (phase !== "still" && phase !== "ended") return
      if (phase === "ended") setup()
      phase = "loading"
      emit("phase", "loading")
      audio.attach(context, muted)
      const wantGPU = !new URLSearchParams(location.search).has("canvas2d")
      const [m, r] = await Promise.all([
        manifest ? manifest : fetch(base + "audio/manifest.json").then((x) => x.json()).catch(() => null),
        renderer ? renderer : (wantGPU ? createGPU(canvas).catch(() => null) : Promise.resolve(null)).then((g) => g || createCanvas2D(canvas)),
      ])
      manifest = m
      renderer = r
      if (manifest) await audio.load(manifest)
      phase = "live"
      emit("phase", "live")
      last = 0
      render()
      schedule()
    },
    setMuted(muted, context) {
      audio.setMuted(muted, context)
      if (!muted && phase === "live") {
        audio.stopAll()
        if (roomLevel > 0) audio.roomLevel(roomLevel, t, 0.3)
        if (caption) {
          const v = audio.voiceFile(caption.id)
          if (v && t - caption.t0 < v.dur) audio.play(v.file, { t, offset: t - caption.t0, kind: "voice" })
        }
      }
    },
    skip() {
      if (phase === "live" || phase === "loading") S.end()
    },
    // Option+n during the visitor's turn.
    key(n) {
      if (phase !== "live" || t < HOLD[0] || t > HOLD[1] - 0.5 || n < 1 || n > 3) return false
      world.switchTo(n)
      S.sfx("key", 0.6)
      return true
    },
    // A tap on the bar during the visitor's turn: which workspace was it?
    tap(x, y) {
      if (phase !== "live" || !world) return false
      const F = fit()
      const lx = (x - F.ox) / F.k
      const ly = (y - F.oy) / F.k
      const g = world.barGeom()
      if (ly < g.y - 20 || ly > g.y + g.h + 20) return false
      for (let n = 1; n <= world.bar.cells.length; n++) {
        const r = world.cellRect(n)
        if (lx >= r.x - 6 && lx <= r.x + r.w + 6) return api.key(n)
      }
      return false
    },
    destroy() {
      document.removeEventListener("visibilitychange", onVis)
      io.disconnect()
      ro.disconnect()
      if (raf) cancelAnimationFrame(raf)
      audio.close()
      renderer?.destroy()
    },
  }

  if (capture) {
    window.__show = {
      api,
      // Advance the clock by n steps of dt and render once.
      async step(dt = 1 / 30, n = 1) {
        for (let i = 0; i < n; i++) advance(dt)
        render()
        await renderer?.done()
      },
      // Jump forward without rendering (cues and springs still run).
      async seek(to) {
        while (t < to - 1e-6 && phase === "live") advance(Math.min(1 / 30, to - t))
        render()
        await renderer?.done()
      },
      // Real-time playback for perf: run rAF for ms and report frame stats.
      perf(ms) {
        return new Promise((res) => {
          stats.intervals.length = 0
          stats.cpu.length = 0
          last = 0
          const t0 = performance.now()
          const loop = (now) => {
            const dt = last ? Math.min(0.05, (now - last) / 1000) : 1 / 60
            if (last) stats.intervals.push(now - last)
            last = now
            advance(dt)
            render()
            if (now - t0 < ms && phase === "live") requestAnimationFrame(loop)
            else res(summary())
          }
          requestAnimationFrame(loop)
        })
      },
      state: () => ({ t: +t.toFixed(3), phase, active: world.active, D: +camera().D.toFixed(3), pullA: pull.a, pullB: pull.b, renderer: renderer?.kind, dprScale }),
      audioLog: () => audio.log,
      stats: summary,
    }
  }

  function summary() {
    const q = (arr, p) => {
      if (!arr.length) return 0
      const s = [...arr].sort((a, b) => a - b)
      return +s[Math.min(s.length - 1, Math.floor(p * s.length))].toFixed(2)
    }
    return { n: stats.intervals.length, interval: { p50: q(stats.intervals, 0.5), p95: q(stats.intervals, 0.95), p99: q(stats.intervals, 0.99), max: q(stats.intervals, 1) }, cpu: { p50: q(stats.cpu, 0.5), p95: q(stats.cpu, 0.95), max: q(stats.cpu, 1) }, over20: stats.intervals.filter((x) => x > 20).length, dprScale }
  }

  void MENU_H
  return api
}
