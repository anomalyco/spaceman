// The show engine. createShow() draws the still with Canvas 2D (only the small wallpaper is
// fetched); start() runs inside the key press: it brings up WebGPU (or Canvas 2D), fetches the tape,
// the effects and the plates, and plays the film. The picture is cut to the tape's word times
// (tape.js), so it runs the same with or without sound. ?capture exposes window.__show for
// deterministic frame stepping and a motion trace.
import { World } from "./world.js"
import { build, pile, HOLD, END } from "./director.js"
import { createCanvas2D } from "./canvas2d.js"
import { createGPU, POST_DEFAULT } from "./gpu.js"
import { Audio } from "./audio.js"
import { LINES, TAPE } from "./tape.js"
import { plates, loadPlate, coverUV } from "./plates.js"
import { glowCanvas, STUDY, KITCHEN, coverView, zoomT } from "./levels.js"
import { clamp, mix } from "./springs.js"

const PLATES = ["wall", "hedge", "door", "across", "study", "kitchen"]

export function createShow({ still, canvas, base, capture = false, reduced = false, on = {} }) {
  const emit = (k, v) => on[k]?.(v)
  let cssW = 0
  let cssH = 0
  let SW = 1440
  let SH = 900
  let world = null
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
  let tapeAt = null // { ctx, t } when the tape started: the film follows the tape's clock
  const cam = { D: 1, cx: 0, cy: 0 }
  const grade = [0, 0, 0]
  const fx = { ...POST_DEFAULT }
  const scene = { pull: 0, win: 0, fade: 0, kpush: 0 }
  const look = { focus: null, dof: 0.6 }
  const audio = new Audio(base + "audio/", { log: capture })
  const stats = { intervals: [], cpu: [], frames: 0 }
  const trace = []

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
    w.cursor.x = SW * 0.7
    w.cursor.y = SH * 0.62
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

  // Window canvases are painted at the device pixels they cover, so text stays sharp in a push.
  function setRes(w, k, d) {
    const r = Math.round(clamp(k * d, 0.5, 5) * 4) / 4
    if (w.res !== r) {
      w.res = r
      w.menuCanvas = null
      w.keys.canvas = null
    }
  }

  // ── Projection ─────────────────────────────────────────────────────────
  // The camera looks at part of his screen: D is the fraction of the screen in view.
  function camT(F) {
    const s = 1 / cam.D
    return { k: s * F.k, ox: (SW / 2 - cam.cx * s) * F.k + F.ox, oy: (SH / 2 - cam.cy * s) * F.k + F.oy }
  }

  function focusRect(T) {
    const id = look.focus
    const w = id && world.wins.get(id)
    if (!w) return null
    return [w.x.x * T.k + T.ox, w.y.x * T.k + T.oy, (w.x.x + w.w.x) * T.k + T.ox, (w.y.x + w.h.x) * T.k + T.oy]
  }

  // The study: his screen is a photograph's monitor; the world is drawn into it.
  let pullFrom = null
  function studyT(F) {
    const img = plates.study
    const M = STUDY.screen
    const g = Math.min(M.w / SW, M.h / SH)
    const G = { k: g, ox: M.x + (M.w - SW * g) / 2, oy: M.y + (M.h - SH * g) / 2 }
    if (!pullFrom) {
      // where the pull starts: exactly the framing the camera had on the screen
      const T0 = camT(F)
      const k = T0.k / g
      pullFrom = { k, ox: T0.ox - k * G.ox, oy: T0.oy - k * G.oy }
    }
    const A = cssW / cssH
    const tall = A < 1
    const wide = coverView(img, cssW, cssH, tall ? STUDY.tallFocus : STUDY.focus, 1)
    const near = coverView(img, cssW, cssH, STUDY.window, tall ? 1.7 : 2.6)
    let P = zoomT(pullFrom, wide, stepIf(scene.pull))
    if (scene.win > 0) P = zoomT(wide, near, stepIf(scene.win))
    return { P, G }
  }

  // Reduced motion: held framings with cuts between them instead of continuous moves.
  const stepIf = (u) => (reduced ? (u < 0.5 ? 0 : 1) : ease(u))
  const ease = (u) => {
    u = clamp(u)
    return u < 0.5 ? 4 * u * u * u : 1 - (-2 * u + 2) ** 3 / 2
  }

  function drawStudy(list, F) {
    const img = plates.study
    const { P, G } = studyT(F)
    list.push({ x: P.ox, y: P.oy, w: img.width * P.k, h: img.height * P.k, tex: img, g: 0 })
    const M = STUDY.screen
    const mx = P.ox + M.x * P.k
    const my = P.oy + M.y * P.k
    const mw = M.w * P.k
    const mh = M.h * P.k
    const clip = [mx, my, mx + mw, my + mh]
    list.push({ x: mx, y: my, w: mw, h: mh, col: [0.02, 0.02, 0.025, 1], r: 1.5 * P.k })
    // past a portrait desktop's edges, the monitor shows the wallpaper
    const wall = world.wall
    list.push({ x: mx, y: my, w: mw, h: mh, tex: wall, uv: coverUV(wall, mw, mh), r: 1.5 * P.k, clip })
    list.push({ x: mx, y: my, w: mw, h: mh, col: [0, 0, 0, 0.45], r: 1.5 * P.k, clip })
    const T = { k: G.k * P.k, ox: P.ox + G.ox * P.k, oy: P.oy + G.oy * P.k }
    setRes(world, T.k, dpr())
    world.draw(list, T, { clip, g: 1 })
    // the panel's own sheen, and its light on the desk and the wall around it
    const glow = glowCanvas()
    list.push({ x: mx, y: my, w: mw, h: mh, col: [1, 1, 1, 0.035], r: 1.5 * P.k, clip })
    list.push({ x: mx - mw * 0.9, y: my - mh * 0.7, w: mw * 2.8, h: mh * 2.6, tex: glow, col: [0.55, 0.62, 0.8, 0.16], g: 0 })
    fx.focus = scene.win > 0.2 ? [P.ox + STUDY.window.x * img.width * P.k - 80, P.oy + STUDY.window.y * img.height * P.k - 60, P.ox + STUDY.window.x * img.width * P.k + 80, P.oy + STUDY.window.y * img.height * P.k + 60] : clip
  }

  // The kitchen across the street, where the recorder is.
  function drawKitchen(list, a) {
    const img = plates.kitchen
    const tall = cssW < cssH
    const K0 = coverView(img, cssW, cssH, tall ? KITCHEN.tallFocus : KITCHEN.focus, tall ? 1.05 : 1.04)
    const K1 = coverView(img, cssW, cssH, KITCHEN.recorder, tall ? 1.5 : 1.55)
    const P = zoomT(K0, K1, reduced ? 0 : clamp(scene.kpush))
    list.push({ x: P.ox, y: P.oy, w: img.width * P.k, h: img.height * P.k, tex: img, a, g: 0 })
    // the record light
    const lx = P.ox + KITCHEN.light.x * img.width * P.k
    const ly = P.oy + KITCHEN.light.y * img.height * P.k
    const s = 70 * P.k
    list.push({ x: lx - s, y: ly - s, w: 2 * s, h: 2 * s, tex: glowCanvas(), col: [1.6, 0.25, 0.18, 0.35], a, g: 0 })
    list.push({ x: lx - 15 * P.k, y: ly - 10 * P.k, w: 30 * P.k, h: 20 * P.k, col: [1.5, 0.3, 0.22, 0.45], r: 6 * P.k, a, g: 0 })
    if (a > 0.5) fx.focus = [lx - 260 * P.k, ly - 220 * P.k, lx + 260 * P.k, ly + 160 * P.k]
  }

  function buildList() {
    const list = []
    const F = fit()
    fx.focus = null
    const plateEnd = scene.pull > 0 && plates.study
    if (scene.fade < 1) {
      if (!plateEnd) {
        const T = camT(F)
        setRes(world, T.k, dpr())
        world.draw(list, T, { g: 1 })
        fx.focus = focusRect(T)
        if (F.ox > 0.5 || F.oy > 0.5) letterbox(list, F)
      } else drawStudy(list, F)
    }
    if (scene.fade > 0 && plates.kitchen) drawKitchen(list, clamp(scene.fade))
    return list
  }

  function postFx() {
    // depth of field grows with the push, so a full-screen view is sharp everywhere
    const depth = scene.pull > 0 || scene.fade > 0 ? 1 : clamp((1 - cam.D) / 0.45)
    return { ...fx, dof: fx.focus ? look.dof * depth : 0 }
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
    get t() {
      return t
    },
    reduced,
    cam,
    grade,
    fx,
    scene,
    look,
    sfx(name, gain = 1) {
      audio.play("sfx-" + name + ".mp3", { t, gain })
    },
    bed(name, v, tc) {
      audio.bed("sfx-" + name + ".mp3", v, t, tc)
    },
    hint(v) {
      emit("hint", v)
    },
    end() {
      phase = "ended"
      setCaption(null)
      audio.stopAll()
      tapeAt = null
      emit("phase", "ended")
    },
  }

  // ── The tape ───────────────────────────────────────────────────────────
  function startTape() {
    const tape = audio.tape
    if (!tape || phase !== "live" || t >= tape.dur - 0.05) return
    const h = audio.play(tape.file, { t, offset: t, kind: "voice" })
    tapeAt = h ? { ctx: audio.ctx.currentTime, t } : null
  }
  audio.onReady = (file) => {
    if (file === audio.tape?.file && phase === "live" && !tapeAt) {
      startTape()
      audio.restoreBeds(t)
    }
  }

  // one sentence at a time, held a moment after it ends unless the next one starts
  let li = 0
  function captions() {
    while (li < LINES.length && t > LINES[li][2] + 0.6) li++
    const L = LINES[li]
    const on = L && t >= L[1] - 0.05 && (li + 1 >= LINES.length || t < LINES[li + 1][1] - 0.05)
    setCaption(on ? L[0] : null)
  }
  function setCaption(text) {
    if (text === caption) return
    caption = text
    emit("caption", text ? { text } : null)
  }

  function setup() {
    world = makeWorld()
    t = 0
    ci = 0
    li = 0
    grade.fill(0)
    Object.assign(fx, POST_DEFAULT)
    Object.assign(scene, { pull: 0, win: 0, fade: 0, kpush: 0 })
    look.focus = null
    pullFrom = null
    cam.D = 1
    cam.cx = SW / 2
    cam.cy = SH / 2
    trace.length = 0
    const b = build(S)
    cues = b.cues
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
      world.update(h)
      if (phase !== "live") break
    }
    t = Math.max(t, Math.min(t1, END))
    if (phase === "live") captions()
  }

  function render() {
    if (!renderer) return
    const c0 = performance.now()
    const d = sizeCanvas(canvas)
    const list = phase === "ended" ? [] : buildList()
    renderer.render(list, cssW, cssH, d, grade, postFx())
    renderer.sweep()
    stats.cpu.push(performance.now() - c0)
    if (stats.cpu.length > 600) stats.cpu.shift()
  }

  // While the tape plays, its clock leads: a long frame never lets the picture fall behind it.
  function tapeTime() {
    if (!tapeAt || !audio.on) return null
    return tapeAt.t + (audio.ctx.currentTime - tapeAt.ctx)
  }

  function frame(now) {
    raf = 0
    let dt = last ? Math.min(0.05, (now - last) / 1000) : 1 / 60
    if (last) {
      stats.intervals.push(now - last)
      if (stats.intervals.length > 600) stats.intervals.shift()
    }
    last = now
    const tt = tapeTime()
    if (tt != null) dt = clamp(tt - t, 0, 0.12)
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
    if (!still || phase !== "still") return
    measure()
    stillR ||= createCanvas2D(still)
    const d = sizeCanvas(still)
    setRes(world, fit().k, d)
    const list = []
    world.draw(list, fit(), { g: 1 })
    letterbox(list, fit())
    stillR.render(list, cssW, cssH, d, [0, 0, 0])
  }

  logical()
  setup()
  drawStill()
  // the only fetch before the key press: a 17 KB wallpaper for the still
  loadPlate(base, "wall-s").then(() => drawStill())

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
      const platesIn = Promise.all(PLATES.map((p) => loadPlate(base, p)))
      const [m, r] = await Promise.all([
        manifest ? manifest : fetch(base + "audio/manifest.json").then((x) => x.json()).catch(() => null),
        renderer ? renderer : (wantGPU ? createGPU(canvas).catch(() => null) : Promise.resolve(null)).then((g) => g || createCanvas2D(canvas)),
      ])
      manifest = m
      renderer = r
      // the wallpaper first (the first frame needs it); the rest arrive long before they're seen
      await Promise.race([Promise.all([manifest ? audio.load(manifest) : null, loadPlate(base, "wall")]), new Promise((res) => setTimeout(res, 2500))])
      if (capture) await platesIn
      for (const w of world.wins.values()) w.dirty = true
      phase = "live"
      emit("phase", "live")
      last = 0
      startTape()
      render()
      schedule()
    },
    setMuted(muted, context) {
      audio.setMuted(muted, context)
      if (!muted && phase === "live") {
        audio.stopAll()
        tapeAt = null
        startTape()
        audio.restoreBeds(t)
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
      const T = camT(fit())
      const lx = (x - T.ox) / T.k
      const ly = (y - T.oy) / T.k
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

  function sample() {
    return { t: +t.toFixed(4), cue: ci, cam: [+cam.cx.toFixed(2), +cam.cy.toFixed(2), +cam.D.toFixed(5)], scene: { ...scene }, active: world.active, ...world.sample() }
  }

  if (capture) {
    window.__show = {
      api,
      // Advance the clock by n steps of dt and render once; every step lands in the trace.
      async step(dt = 1 / 30, n = 1) {
        for (let i = 0; i < n; i++) {
          advance(dt)
          trace.push(sample())
        }
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
      state: () => ({ t: +t.toFixed(3), phase, active: world.active, D: +cam.D.toFixed(3), scene: { ...scene }, renderer: renderer?.kind, dprScale, plates: Object.keys(plates) }),
      trace: (from = 0) => trace.slice(from),
      audioLog: () => audio.log,
      stats: summary,
      tape: TAPE,
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

  void mix
  return api
}
