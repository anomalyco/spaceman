// The show engine. The page's first image is a still of the study (a Blender render); the study
// itself (study.glb, baked in Blender) is fetched once the page is idle. start() runs inside the key
// press: it brings up WebGPU (or Canvas 2D), fetches the tape and the effects, and plays the film.
// In WebGPU the desktop is drawn into the monitor's panel and the room around it is real 3D; tight on
// the screen the desktop is drawn flat, framed exactly as the panel was, so the two cut together.
// Canvas 2D gets the same film with stills rendered from the same cameras. The picture is cut to the tape's word times
// (tape.js), so it runs the same with or without sound. ?capture exposes window.__show for
// deterministic frame stepping and a motion trace.
import { World } from "./world.js"
import { build, pile, HOLD, END } from "./director.js"
import { createCanvas2D } from "./canvas2d.js"
import { createGPU, POST_DEFAULT } from "./gpu.js"
import { Audio } from "./audio.js"
import { LINES, TAPE } from "./tape.js"
import { plates, loadPlate, coverUV } from "./plates.js"
import { PHOTOS } from "./library.js"
import { loadGLB, mul, perspective, lookAt, norm, sub, add, scale, cross, dot, lerp3 } from "./glb.js"
import { clamp, mix, smooth } from "./springs.js"

// pictures inside his apps (renders of our own scenes), and the stills Canvas 2D uses for the room
const PLATES = ["wall", "wall-s", "door", "across"]
const STILLS = { CamChair: "chair", CamRoom: "room", CamWindow: "window", Before: "before", After: "after" }
const HANDOFF_FOV = 30

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
  const scene = {}
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

  // Reduced motion: held framings with cuts between them instead of continuous moves.
  const ease = (u) => {
    u = clamp(u)
    return u < 0.5 ? 4 * u * u * u : 1 - (-2 * u + 2) ** 3 / 2
  }

  // ── The room ───────────────────────────────────────────────────────────
  // three.a → three.b at three.u: named cameras (study.json), or "screen": the camera whose view of
  // the panel is exactly what the flat desktop shows right now.
  const three = { on: false, a: "CamChair", b: "CamChair", u: 0, glow: 1, black: 0, drift: 0.1, after: false }
  // the study, then the basement; each is a GLB and its study.json (cameras, the panel, bake scales)
  const SETS = ["study", "basement"]
  const room = { meta: {}, glb: {}, loading: {}, up: new Set() }
  function loadRoom(set = "study") {
    room.loading[set] ||= Promise.all([fetch(base + "3d/" + set + ".json").then((r) => r.json()), loadGLB(base + "3d/" + set + ".glb")])
      .then(([meta, glb]) => {
        room.meta[set] = meta
        room.glb[set] = glb
        return true
      })
      .catch(() => false)
    return room.loading[set]
  }
  // metadata alone (cameras, the panel), for Canvas 2D's stills
  function loadMeta(set) {
    if (room.meta[set]) return Promise.resolve(true)
    return fetch(base + "3d/" + set + ".json")
      .then((r) => r.json())
      .then((m) => (room.meta[set] = m))
      .catch(() => null)
  }
  async function uploadRoom(set = "study") {
    if (!renderer?.loadScene || !(await loadRoom(set))) return false
    if (!room.up.has(set)) {
      await renderer.loadScene(set, room.glb[set], room.meta[set])
      room.glb[set] = null
      room.up.add(set)
      if (set === "basement") renderer.setVisible("basement", "StateAfter", false)
    }
    return true
  }
  const setOf = (name) => (room.meta.basement?.cameras[name] ? "basement" : "study")
  const in3d = () => three.on && room.up.has("study") && (room.up.has(setOf(three.b)) || !room.meta.basement) && renderer?.render3d

  // the panel's texture: the desktop fitted into it, the wallpaper past its edges
  function panel() {
    const th = Math.round(clamp(canvas.height, 720, 1600))
    const tw = Math.round(th * 1.6)
    const g = Math.min(tw / SW, th / SH)
    return { tw, th, G: { k: g, ox: (tw - SW * g) / 2, oy: (th - SH * g) / 2 } }
  }

  // panel geometry (glTF, Y up): top-left corner, right and down edges, the normal toward the room
  function panelFrame() {
    const [bl, br, tr, tl] = room.meta.study.screen
    const ex = sub(tr, tl)
    const ey = sub(bl, tl)
    let n = norm(cross(ey, ex))
    const c0 = room.meta.study.cameras.CamScreen.pos
    if (dot(n, sub(c0, tl)) < 0) n = scale(n, -1)
    return { tl, ex, ey, n, br }
  }

  // The camera that sees the panel exactly as the flat view shows the desktop now.
  function screenShot() {
    const { tw, th, G } = panel()
    const T = camT(fit())
    const pf = panelFrame()
    const px = (cssW / 2 - T.ox) / T.k
    const py = (cssH / 2 - T.oy) / T.k
    const u = (G.ox + px * G.k) / tw
    const v = (G.oy + py * G.k) / th
    const C = add(pf.tl, add(scale(pf.ex, u), scale(pf.ey, v)))
    const mPerCss = (G.k / T.k) * (Math.hypot(...pf.ex) / tw)
    const fov = (HANDOFF_FOV * Math.PI) / 180
    const d = (cssH * mPerCss) / 2 / Math.tan(fov / 2)
    return { pos: add(C, scale(pf.n, d)), dir: scale(pf.n, -1), vfov: HANDOFF_FOV, focus: d, aperture: 0, tm: 0 }
  }

  // a named camera, its field of view opened up on tall screens so the subject still fits across
  function namedShot(name) {
    // a camera from a set that isn't here yet falls back to the room
    const c = room.meta[setOf(name)]?.cameras[name] || room.meta.study.cameras.CamRoom
    const A = cssW / cssH
    const k = A < 1.25 ? (1.25 / A) ** 0.42 : 1
    const vfov = (2 * Math.atan(Math.tan(((c.vfov * Math.PI) / 180) / 2) * k) * 180) / Math.PI
    return { pos: c.pos, dir: c.dir, vfov, focus: c.focus || 3, aperture: c.focus ? 0.9 : 0, tm: 1 }
  }
  const shotOf = (n) => (n === "screen" ? screenShot() : namedShot(n))

  function camera3() {
    // a move between the sets is a cut
    const a = setOf(three.a) === setOf(three.b) ? three.a : three.b
    const A = shotOf(a)
    const B = three.b === a ? A : shotOf(three.b)
    const u = three.u
    // on one camera, u is a slow drift in along its axis
    const pos = three.b === a ? add(A.pos, scale(A.dir, three.drift * u)) : lerp3(A.pos, B.pos, u)
    let dir = norm(lerp3(A.dir, B.dir, u))
    // off the screen, the camera keeps looking at what it was looking at on the screen, and only turns
    // to the room's own point of interest in the second half, so the screen never slides out of frame
    if (a === "screen" && B !== A) {
      const w = smooth(clamp((u - 0.25) / 0.75))
      const look = lerp3(add(A.pos, scale(A.dir, A.focus)), add(B.pos, scale(B.dir, B.focus)), w)
      dir = norm(sub(look, pos))
    } else if (three.b === "screen" && B !== A) {
      // and onto it, the mirror: it turns to the screen first, then comes in
      const w = smooth(clamp(u / 0.75))
      const look = lerp3(add(A.pos, scale(A.dir, A.focus)), add(B.pos, scale(B.dir, B.focus)), w)
      dir = norm(sub(look, pos))
    }
    const vfov = mix(A.vfov, B.vfov, u)
    // focus moves in dioptres, so it racks the way a lens does; off the screen it stays on the screen
    // until the last part of the move, then racks to the room
    let focus = 1 / mix(1 / A.focus, 1 / B.focus, u)
    if ((a === "screen" || three.b === "screen") && B !== A) {
      const pf = panelFrame()
      const mid = add(pf.tl, add(scale(pf.ex, 0.5), scale(pf.ey, 0.5)))
      const onScreen = 1 / Math.max(0.05, dot(sub(mid, pos), dir))
      focus = a === "screen" ? 1 / mix(onScreen, 1 / B.focus, smooth(clamp((u - 0.6) / 0.4))) : 1 / mix(1 / A.focus, onScreen, smooth(clamp(u / 0.4)))
    }
    // the tone curve and the depth of field arrive in the first part of a move off the screen
    const off = A.tm < B.tm ? clamp(u * 2.5) : B.tm < A.tm ? clamp((1 - u) * 2.5) : A.tm
    const aperture = Math.max(A.aperture, B.aperture) * off
    const P = perspective((vfov * Math.PI) / 180, cssW / cssH, 0.02, 150)
    const V = lookAt(pos, add(pos, dir), [0, 1, 0])
    return { vp: mul(P, V), eye: pos, focus, aperture, tm: off }
  }

  function render3d(d) {
    const { tw, th, G } = panel()
    const list = []
    const wall = world.wall
    list.push({ x: 0, y: 0, w: tw, h: th, tex: wall, uv: coverUV(wall, tw, th), g: 1 })
    list.push({ x: 0, y: 0, w: tw, h: th, col: [0, 0, 0, 0.4], g: 0 })
    setRes(world, G.k, 1)
    world.draw(list, G, { g: 1 })
    const c = camera3()
    const fx3 = { ...fx, dof: c.aperture > 0 ? 1 : 0, focus: null }
    const set = setOf(three.b)
    if (set === "basement") renderer.setVisible("basement", "StateAfter", three.after), renderer.setVisible("basement", "StateBefore", !three.after)
    renderer.render3d(list, tw, th, cssW, cssH, grade, fx3, {
      key: set,
      vp: c.vp,
      eye: c.eye,
      glow: three.glow,
      lit: 1,
      exposure: 1 - clamp(three.black),
      screenCol: [1, 1, 1, 1],
      focus: c.focus,
      aperture: c.aperture,
      tm: c.tm,
    })
    void d
  }

  // ── Getting ready ──────────────────────────────────────────────────────
  let rendererIn = null
  let platesIn = null
  let prepping = null
  const stillsIn = () => Promise.all([...Object.values(STILLS).map((p) => loadPlate(base, p)), loadMeta("study"), loadMeta("basement")])
  function prepare() {
    if (prepping) return prepping
    const wantGPU = !new URLSearchParams(location.search).has("canvas2d")
    rendererIn = (wantGPU ? createGPU(canvas).catch(() => null) : Promise.resolve(null)).then((g) => (renderer = g || createCanvas2D(canvas)))
    const manifestIn = fetch(base + "audio/manifest.json")
      .then((x) => x.json())
      .then((m) => (manifest = m))
      .catch(() => null)
    platesIn = Promise.all([...PLATES.map((p) => loadPlate(base, p)), ...PHOTOS.map((p) => loadPlate(base, p, "photos/"))])
    prepping = Promise.all([
      manifestIn.then((m) => m && audio.load(m)),
      loadPlate(base, "wall"),
      rendererIn.then((r) =>
        r.render3d
          ? uploadRoom().then((ok) => {
              if (!ok) return stillsIn()
              void uploadRoom("basement")
            })
          : stillsIn(),
      ),
    ]).then(() => true)
    return prepping
  }

  // ── Canvas 2D (or the room not loaded): stills from the same cameras ──
  // A move onto the screen pushes into the still's monitor; a move off it cuts to the still and
  // drifts in; a move between two cameras cuts to the second.
  function project(name, p, iw, ih) {
    const c = room.meta.study?.cameras[name]
    if (!c) return null
    const V = lookAt(c.pos, add(c.pos, c.dir), [0, 1, 0])
    const M = mul(perspective((c.vfov * Math.PI) / 180, iw / ih, 0.02, 150), V)
    const x = M[0] * p[0] + M[4] * p[1] + M[8] * p[2] + M[12]
    const y = M[1] * p[0] + M[5] * p[1] + M[9] * p[2] + M[13]
    const w = M[3] * p[0] + M[7] * p[1] + M[11] * p[2] + M[15]
    if (w <= 0.01) return null
    return [((x / w + 1) / 2) * iw, ((1 - y / w) / 2) * ih]
  }
  function drawStillShot(list) {
    const toScreen = three.b === "screen"
    const name = toScreen ? three.a : three.b
    const img = plates[STILLS[name === "Before" && three.after ? "After" : name]]
    if (!img) {
      list.push({ x: 0, y: 0, w: cssW, h: cssH, col: [0.02, 0.02, 0.025, 1], g: 0 })
      return
    }
    const cover = Math.max(cssW / img.width, cssH / img.height)
    let k = cover
    let cx = img.width / 2
    let cy = img.height / 2
    const near = toScreen && room.meta.study ? room.meta.study.screen.map((p) => project(name, p, img.width, img.height)) : null
    if (near && near.every(Boolean)) {
      // push in until the monitor fills the frame
      const pts = near
      const x0 = Math.min(...pts.map((p) => p[0]))
      const x1 = Math.max(...pts.map((p) => p[0]))
      const y0 = Math.min(...pts.map((p) => p[1]))
      const y1 = Math.max(...pts.map((p) => p[1]))
      const k1 = Math.min(cssW / (x1 - x0), cssH / (y1 - y0))
      const e = three.u
      k = cover * Math.exp(Math.log(k1 / cover) * e)
      cx = mix(cx, (x0 + x1) / 2, e)
      cy = mix(cy, (y0 + y1) / 2, e)
    } else k = cover * (1 + 0.05 * three.u)
    list.push({ x: cssW / 2 - cx * k, y: cssH / 2 - cy * k, w: img.width * k, h: img.height * k, tex: img, g: 0 })
    // the stills were rendered with a blank panel: the desktop goes on it, as it is now
    const pts = room.meta.study?.cameras[name] && room.meta.study.screen.map((p) => project(name, p, img.width, img.height))
    if (pts && pts.every((p) => p && isFinite(p[0]) && isFinite(p[1]))) {
      const X = pts.map((p) => cssW / 2 + (p[0] - cx) * k)
      const Y = pts.map((p) => cssH / 2 + (p[1] - cy) * k)
      const clip = [Math.min(...X), Math.min(...Y), Math.max(...X), Math.max(...Y)]
      const bw = clip[2] - clip[0]
      const bh = clip[3] - clip[1]
      if (bw > 2 && bw < cssW * 6) {
        const kk = Math.max(bw / SW, bh / SH)
        setRes(world, kk, dpr())
        world.draw(list, { k: kk, ox: (clip[0] + clip[2]) / 2 - (SW * kk) / 2, oy: (clip[1] + clip[3]) / 2 - (SH * kk) / 2 }, { clip, g: 1 })
        list.push({ x: clip[0], y: clip[1], w: bw, h: bh, col: [0, 0, 0, 0.12 + 0.88 * (1 - clamp(three.glow))], g: 0, clip })
      }
    }
    if (three.glow < 1 && name === "CamChair") list.push({ x: 0, y: 0, w: cssW, h: cssH, col: [0, 0, 0, 0.5 * (1 - three.glow)], g: 0 })
    if (three.black > 0) list.push({ x: 0, y: 0, w: cssW, h: cssH, col: [0, 0, 0, clamp(three.black)], g: 0 })
  }

  function buildList() {
    const list = []
    const F = fit()
    fx.focus = null
    if (three.on) {
      drawStillShot(list)
      return list
    }
    const T = camT(F)
    setRes(world, T.k, dpr())
    world.draw(list, T, { g: 1 })
    fx.focus = focusRect(T)
    if (F.ox > 0.5 || F.oy > 0.5) letterbox(list, F)
    return list
  }

  function postFx() {
    // depth of field grows with the push, so a full-screen view is sharp everywhere
    const depth = three.on ? 0.4 : clamp((1 - cam.D) / 0.45)
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
    three,
    // a move between two cameras of the room, at u (already eased)
    shot(a, b, u) {
      three.on = true
      three.a = a
      three.b = b
      three.u = u
    },
    flat() {
      three.on = false
    },
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
      // the interview runs on under the card: the detective suspends it, and the tone
      audio.stopAll(audio.tape?.file)
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
    const high = !!(on && S.capHigh && L[1] >= S.capHigh[0] && L[1] <= S.capHigh[1])
    setCaption(on ? L[0] : null, high)
  }
  function setCaption(text, high = false) {
    if (text === caption) return
    caption = text
    emit("caption", text ? { text, high } : null)
  }

  function setup() {
    world = makeWorld()
    t = 0
    ci = 0
    li = 0
    grade.fill(0)
    Object.assign(fx, POST_DEFAULT)
    Object.assign(three, { on: false, a: "CamChair", b: "CamChair", u: 0, glow: 1, black: 0, drift: 0.1, after: false })
    look.focus = null
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
    if (phase !== "ended" && in3d()) render3d(d)
    else renderer.render(phase === "ended" ? [] : buildList(), cssW, cssH, d, grade, postFx())
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
  // Everything but the sound's start comes down once the page is idle, so a click starts the film at
  // once: the renderer, the room (or Canvas 2D's stills), the tape and the effects. Save-Data waits
  // for the click.
  if (!capture && !navigator.connection?.saveData) setTimeout(() => void prepare(), 1200)

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
    prepare,
    async start({ context = null, muted = false } = {}) {
      if (phase !== "still" && phase !== "ended") return
      if (phase === "ended") {
        audio.stopAll()
        setup()
      }
      phase = "loading"
      emit("phase", "loading")
      audio.attach(context, muted)
      // usually everything is already here (prepare() ran while the page was idle); if not, the still
      // stays up a moment, then the film starts with whatever has arrived and the rest joins in
      const ready = prepare()
      await rendererIn
      await Promise.race([ready, new Promise((res) => setTimeout(res, capture ? 60000 : 1200))])
      if (!room.up.has("study")) void stillsIn()
      if (capture) await Promise.all([ready, uploadRoom("basement"), platesIn])
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
    return { t: +t.toFixed(4), cue: ci, cam: [+cam.cx.toFixed(2), +cam.cy.toFixed(2), +cam.D.toFixed(5)], three: three.on ? [three.a, three.b, +three.u.toFixed(4)] : null, active: world.active, ...world.sample() }
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
      state: () => ({ t: +t.toFixed(3), phase, active: world.active, D: +cam.D.toFixed(3), three: { ...three }, room: [...room.up], renderer: renderer?.kind, dprScale, plates: Object.keys(plates) }),
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
