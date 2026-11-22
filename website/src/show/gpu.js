// WebGPU renderer. The display list is drawn by one instanced rounded-rect pipeline into a
// half-float scene texture; a downsample chain of that scene then feeds one post pass that adds
// depth of field around the camera's subject, soft bloom, the grade, a vignette and film grain.
// Consecutive primitives sharing a texture go out in one draw call; canvas textures are re-uploaded
// only when their version changes.
const FLOATS = 24 // six vec4f per primitive, matching struct P
const LEVELS = 4

const PRIM = /* wgsl */ `
struct U { view: vec4f, grade: vec4f }
struct P { rect: vec4f, col: vec4f, p: vec4f, clip: vec4f, q: vec4f, uv: vec4f }
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var<storage, read> ps: array<P>;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
const CORNERS = array<vec2f, 6>(vec2f(0, 0), vec2f(1, 0), vec2f(0, 1), vec2f(0, 1), vec2f(1, 0), vec2f(1, 1));
struct VO {
  @builtin(position) pos: vec4f,
  @location(0) world: vec2f,
  @location(1) @interpolate(flat) ii: u32,
}
@vertex fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VO {
  let p = ps[ii];
  let pad = p.p.z * 1.6 + 2.0;
  let c = CORNERS[vi];
  let w = p.rect.xy - vec2f(pad) + c * (p.rect.zw + vec2f(pad * 2.0)) + vec2f(0.0, p.p.z * 0.35);
  var o: VO;
  o.pos = vec4f(w.x / u.view.x * 2.0 - 1.0, 1.0 - w.y / u.view.y * 2.0, 0.0, 1.0);
  o.world = w;
  o.ii = ii;
  return o;
}
fn sdBox(p: vec2f, b: vec2f, r: f32) -> f32 {
  let q = abs(p) - b + vec2f(r);
  return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let p = ps[i.ii];
  let w = i.world;
  if (w.x < p.clip.x || w.y < p.clip.y || w.x > p.clip.z || w.y > p.clip.w) { discard; }
  let half = p.rect.zw * 0.5;
  let r = min(p.p.x, min(half.x, half.y));
  let dpr = u.view.z;
  var cov = 0.0;
  var rgb = p.col.rgb;
  var a = p.col.a;
  if (p.p.z > 0.0) {
    // a soft shadow: a blurred box, a little below its window
    let s = p.p.z;
    let d = sdBox(w - (p.rect.xy + half) - vec2f(0.0, s * 0.35), half, r);
    let k = clamp(1.0 - (d + s * 0.25) / (s * 1.25), 0.0, 1.0);
    cov = k * k * (3.0 - 2.0 * k) * 0.8;
  } else {
    let d = sdBox(w - (p.rect.xy + half), half, r);
    if (p.p.w > 0.0) {
      cov = clamp(0.5 - (abs(d + p.p.w * 0.5) - p.p.w * 0.5) * dpr, 0.0, 1.0);
    } else {
      cov = clamp(0.5 - d * dpr, 0.0, 1.0);
    }
    if (p.q.y > 0.5) {
      let f = (w - p.rect.xy) / p.rect.zw;
      let uv = mix(p.uv.xy, p.uv.zw, f);
      let t = textureSampleLevel(tex, samp, uv, 0.0);
      // texture is premultiplied; col tints it (and may push it past 1 for light sources)
      if (t.a > 0.0001) { rgb = t.rgb / t.a * p.col.rgb; } else { rgb = vec3f(0.0); }
      a = t.a * p.col.a;
    }
  }
  // the story's grade: g.x darkness, g.y coolness, g.z desaturation
  let gw = p.q.x;
  if (gw > 0.0) {
    let lum = dot(rgb, vec3f(0.299, 0.587, 0.114));
    var gr = mix(rgb, vec3f(lum), u.grade.z);
    gr = gr * mix(vec3f(1.0), vec3f(0.78, 0.86, 1.08), u.grade.y);
    gr = gr * (1.0 - u.grade.x);
    rgb = mix(rgb, gr, gw);
  }
  let alpha = a * cov * p.p.y;
  return vec4f(rgb * alpha, alpha);
}
`

const FULL = /* wgsl */ `
struct VO { @builtin(position) pos: vec4f, @location(0) uv: vec2f }
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VO {
  let x = f32((vi << 1u) & 2u);
  let y = f32(vi & 2u);
  var o: VO;
  o.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
  o.uv = vec2f(x, y);
  return o;
}
`

// Dual-filter downsample: the centre and four diagonal half-texel taps.
const DOWN = FULL + /* wgsl */ `
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let ts = 1.0 / vec2f(textureDimensions(src));
  var c = textureSampleLevel(src, samp, i.uv, 0.0) * 4.0;
  c += textureSampleLevel(src, samp, i.uv + vec2f(-ts.x, -ts.y), 0.0);
  c += textureSampleLevel(src, samp, i.uv + vec2f(ts.x, -ts.y), 0.0);
  c += textureSampleLevel(src, samp, i.uv + vec2f(-ts.x, ts.y), 0.0);
  c += textureSampleLevel(src, samp, i.uv + vec2f(ts.x, ts.y), 0.0);
  return c / 8.0;
}
`

const POST = FULL + /* wgsl */ `
struct PU { res: vec4f, focus: vec4f, a: vec4f, b: vec4f, c: vec4f }
@group(0) @binding(0) var<uniform> pu: PU;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var scene: texture_2d<f32>;
@group(0) @binding(3) var l1: texture_2d<f32>;
@group(0) @binding(4) var l2: texture_2d<f32>;
@group(0) @binding(5) var l3: texture_2d<f32>;
@group(0) @binding(6) var l4: texture_2d<f32>;
// a 3x3 tent around uv, in that level's texels
fn tent(t: texture_2d<f32>, uv: vec2f) -> vec3f {
  let ts = 1.0 / vec2f(textureDimensions(t));
  var c = textureSampleLevel(t, samp, uv, 0.0).rgb * 4.0;
  c += (textureSampleLevel(t, samp, uv + vec2f(ts.x, 0.0), 0.0).rgb + textureSampleLevel(t, samp, uv - vec2f(ts.x, 0.0), 0.0).rgb + textureSampleLevel(t, samp, uv + vec2f(0.0, ts.y), 0.0).rgb + textureSampleLevel(t, samp, uv - vec2f(0.0, ts.y), 0.0).rgb) * 2.0;
  c += textureSampleLevel(t, samp, uv + ts, 0.0).rgb + textureSampleLevel(t, samp, uv - ts, 0.0).rgb + textureSampleLevel(t, samp, uv + vec2f(ts.x, -ts.y), 0.0).rgb + textureSampleLevel(t, samp, uv + vec2f(-ts.x, ts.y), 0.0).rgb;
  return c / 16.0;
}
fn hash(p: vec2f) -> f32 {
  var q = fract(p * vec2f(0.1031, 0.1030));
  q += dot(q, q.yx + 33.33);
  return fract((q.x + q.y) * q.x);
}
// AgX (Troy Sobotka's, in Benjamin Wrensch's polynomial fit): scene-linear in, display out
fn agxCurve(x: vec3f) -> vec3f {
  let x2 = x * x;
  let x4 = x2 * x2;
  return 15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232;
}
fn agx(c: vec3f) -> vec3f {
  let m = mat3x3f(vec3f(0.842479062253094, 0.0423282422610123, 0.0423756549057051), vec3f(0.0784335999999992, 0.878468636469772, 0.0784336), vec3f(0.0792237451477643, 0.0791661274605434, 0.879142973793104));
  let inv = mat3x3f(vec3f(1.19687900512017, -0.0528968517574562, -0.0529716355144438), vec3f(-0.0980208811401368, 1.15190312990417, -0.0980434501171241), vec3f(-0.0990297440797205, -0.0989611768448433, 1.15107367264116));
  var v = clamp(log2(max(m * c, vec3f(1e-10))), vec3f(-12.47393), vec3f(4.026069));
  v = agxCurve((v + 12.47393) / 16.499999);
  // a little more contrast and saturation, like Blender's "Medium High Contrast"
  v = pow(max(v, vec3f(0.0)), vec3f(1.12));
  let l = dot(v, vec3f(0.2126, 0.7152, 0.0722));
  v = mix(vec3f(l), v, 1.12);
  return clamp(inv * v, vec3f(0.0), vec3f(1.0));
}
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let px = i.uv * pu.res.xy;
  let s0 = textureSampleLevel(scene, samp, i.uv, 0.0);
  var c = s0.rgb;
  let three = pu.c.w > 0.5;
  // depth of field: sharp on the subject, softer with distance from it (in 3D, by depth)
  let dof = pu.a.x;
  if (dof > 0.001) {
    let q = max(max(pu.focus.xy - px, px - pu.focus.zw), vec2f(0.0));
    var coc = dof * smoothstep(0.0, 0.42 * min(pu.res.x, pu.res.y), length(q));
    if (three) { coc = dof * abs(s0.a); }
    if (coc > 0.002) {
      let b1 = tent(l1, i.uv);
      let b2 = tent(l2, i.uv);
      let b = mix(b1, b2, smoothstep(0.35, 1.0, coc));
      c = mix(c, b, smoothstep(0.0, 0.35, coc));
    }
  }
  // soft bloom from the wide levels
  let wide = tent(l3, i.uv) * 0.55 + tent(l4, i.uv) * 0.45;
  if (three && s0.a < 0.0) {
    // the panel blooms in display light, as the flat desktop does, so the hand-off between them has no step
    let wd = pow(max(wide, vec3f(0.0)), vec3f(1.0 / 2.2));
    let ld = max(dot(wd, vec3f(0.2126, 0.7152, 0.0722)), 1e-4);
    c = pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.2)) + wd * (max(ld - pu.a.z, 0.0) / ld) * pu.a.y;
  } else {
    let lw = max(dot(wide, vec3f(0.2126, 0.7152, 0.0722)), 1e-4);
    c += wide * (max(lw - pu.a.z, 0.0) / lw) * pu.a.y;
    if (three) { c = mix(pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.2)), agx(c), pu.c.z); }
  }
  // grade: exposure, warmth, saturation, contrast around mid grey
  c *= pu.b.z;
  c *= vec3f(1.0 + pu.c.x * 0.06, 1.0, 1.0 - pu.c.x * 0.08);
  let l = dot(c, vec3f(0.2126, 0.7152, 0.0722));
  c = mix(vec3f(l), c, pu.b.w);
  c = (c - 0.5) * pu.c.y + 0.5;
  // vignette, measured on the short side so phones and desktops darken alike
  let d = (i.uv - 0.5) * pu.res.xy / min(pu.res.x, pu.res.y);
  c *= 1.0 - pu.a.w * smoothstep(0.35, 1.05, length(d));
  // grain (moves every frame), stronger in the shadows; then a dither against banding
  let n = hash(px + vec2f(pu.b.y * 37.0, pu.b.y * 91.0)) + hash(px * 1.7 + vec2f(pu.b.y * 13.0, 7.0)) - 1.0;
  c += n * pu.b.x * (1.0 - 0.6 * clamp(l, 0.0, 1.0));
  c += (hash(px + 0.5) - 0.5) / 255.0;
  return vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), 1.0);
}
`

// The opening plate: a pre-rendered frame of the room drawn as a quad, and the live desktop on the
// panel's quad in it, mapped in perspective (each corner carries q, so uv/q interpolates projectively).
const QUAD = /* wgsl */ `
struct Q { c: array<vec4f, 4>, q: vec4f, k: vec4f }
@group(0) @binding(0) var<uniform> u: Q;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;
struct VO { @builtin(position) pos: vec4f, @location(0) uvq: vec3f }
const ORDER = array<u32, 6>(0, 1, 2, 0, 2, 3);
const UV = array<vec2f, 4>(vec2f(0, 0), vec2f(1, 0), vec2f(1, 1), vec2f(0, 1));
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VO {
  let i = ORDER[vi];
  let q = u.q[i];
  var o: VO;
  o.pos = vec4f(u.c[i].xy, 0.0, 1.0);
  o.uvq = vec3f(UV[i] * q, q);
  return o;
}
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let c = textureSample(tex, smp, i.uvq.xy / i.uvq.z).rgb;
  return vec4f(c * u.k.y * u.k.x, u.k.x);
}
`

// q per corner of a quad (tl, tr, br, bl): from the diagonals' crossing
function quadQ(p) {
  const [a, b, c, d] = p
  const den = (c[0] - a[0]) * (d[1] - b[1]) - (c[1] - a[1]) * (d[0] - b[0])
  if (Math.abs(den) < 1e-9) return [1, 1, 1, 1]
  const s = ((b[0] - a[0]) * (d[1] - b[1]) - (b[1] - a[1]) * (d[0] - b[0])) / den
  const x = [a[0] + s * (c[0] - a[0]), a[1] + s * (c[1] - a[1])]
  const dist = p.map((v) => Math.hypot(v[0] - x[0], v[1] - x[1]))
  return dist.map((di, i) => (di + dist[(i + 2) % 4]) / Math.max(1e-9, dist[(i + 2) % 4]))
}

export const POST_DEFAULT = { focus: null, dof: 0, bloom: 0.18, threshold: 0.62, vignette: 0.22, grain: 0.018, exposure: 1, saturation: 1, warmth: 0, contrast: 1 }

export async function createGPU(canvas) {
  if (!navigator.gpu) return null
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" })
  if (!adapter) return null
  const device = await adapter.requestDevice()
  const ctx = canvas.getContext("webgpu")
  if (!ctx) return null
  const format = navigator.gpu.getPreferredCanvasFormat()
  ctx.configure({ device, format, alphaMode: "opaque" })
  const HDR = "rgba16float"
  const blend = { color: { srcFactor: "one", dstFactor: "one-minus-src-alpha" }, alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" } }
  const primMod = device.createShaderModule({ code: PRIM })
  const downMod = device.createShaderModule({ code: DOWN })
  const postMod = device.createShaderModule({ code: POST })
  const [pipeline, down, post] = await Promise.all([
    device.createRenderPipelineAsync({ layout: "auto", vertex: { module: primMod, entryPoint: "vs" }, fragment: { module: primMod, entryPoint: "fs", targets: [{ format: HDR, blend }] }, primitive: { topology: "triangle-list" } }),
    device.createRenderPipelineAsync({ layout: "auto", vertex: { module: downMod, entryPoint: "vs" }, fragment: { module: downMod, entryPoint: "fs", targets: [{ format: HDR }] }, primitive: { topology: "triangle-list" } }),
    device.createRenderPipelineAsync({ layout: "auto", vertex: { module: postMod, entryPoint: "vs" }, fragment: { module: postMod, entryPoint: "fs", targets: [{ format }] }, primitive: { topology: "triangle-list" } }),
  ])
  const ubuf = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
  const pbuf = device.createBuffer({ size: 80, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
  let cap = 0
  let sbuf = null
  let bg0 = null
  let data = new Float32Array(0)
  const sampler = device.createSampler({ magFilter: "linear", minFilter: "linear" })
  const clampS = device.createSampler({ magFilter: "linear", minFilter: "linear", addressModeU: "clamp-to-edge", addressModeV: "clamp-to-edge" })
  const texs = new Map() // canvas -> { tex, bg, version, w, h }
  const white = document.createElement("canvas")
  white.width = white.height = 2
  const wc = white.getContext("2d")
  wc.fillStyle = "#fff"
  wc.fillRect(0, 0, 2, 2)
  white.version = 1
  let lost = false
  device.lost.then(() => (lost = true))

  // render targets, rebuilt when the canvas size changes
  let rt = null
  function targets(w, h) {
    if (rt && rt.w === w && rt.h === h) return rt
    if (rt) for (const t of [rt.scene, ...rt.levels]) t.destroy()
    const mk = (tw, th) => device.createTexture({ size: [Math.max(1, tw), Math.max(1, th)], format: HDR, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING })
    const scene = mk(w, h)
    const levels = []
    for (let k = 1; k <= LEVELS; k++) levels.push(mk(Math.ceil(w / 2 ** k), Math.ceil(h / 2 ** k)))
    const srcs = [scene, ...levels]
    const downBG = levels.map((_, k) => device.createBindGroup({ layout: down.getBindGroupLayout(0), entries: [{ binding: 0, resource: srcs[k].createView() }, { binding: 1, resource: clampS }] }))
    const postBG = device.createBindGroup({
      layout: post.getBindGroupLayout(0),
      entries: [{ binding: 0, resource: { buffer: pbuf } }, { binding: 1, resource: clampS }, ...srcs.map((t, k) => ({ binding: 2 + k, resource: t.createView() }))],
    })
    rt = { w, h, scene, levels, downBG, postBG }
    return rt
  }

  function grow(n) {
    if (n <= cap) return
    cap = Math.max(256, 1 << Math.ceil(Math.log2(n)))
    sbuf?.destroy()
    sbuf = device.createBuffer({ size: cap * FLOATS * 4, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST })
    data = new Float32Array(cap * FLOATS)
    bg0 = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [{ binding: 0, resource: { buffer: ubuf } }, { binding: 1, resource: { buffer: sbuf } }] })
  }

  function texFor(c) {
    let e = texs.get(c)
    if (e && e.version === c.version && e.w === c.width && e.h === c.height) {
      e.used = true
      return e
    }
    if (!e || e.w !== c.width || e.h !== c.height) {
      e?.tex.destroy()
      const tex = device.createTexture({ size: [Math.max(1, c.width), Math.max(1, c.height)], format: "rgba8unorm", usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT })
      e = { tex, w: c.width, h: c.height, version: -1, bg: device.createBindGroup({ layout: pipeline.getBindGroupLayout(1), entries: [{ binding: 0, resource: tex.createView() }, { binding: 1, resource: sampler }] }) }
      texs.set(c, e)
    }
    if (c.width && c.height) device.queue.copyExternalImageToTexture({ source: c }, { texture: e.tex, premultipliedAlpha: true }, [c.width, c.height])
    e.version = c.version
    e.used = true
    return e
  }

  // ── the desktop's texture for the panel, with its mips ──
  const ubufS = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
  const aniso = device.createSampler({ magFilter: "linear", minFilter: "linear", mipmapFilter: "linear", maxAnisotropy: 8, addressModeU: "clamp-to-edge", addressModeV: "clamp-to-edge" })
  let scr = null
  function screenTarget(tw, th) {
    if (scr && scr.w === tw && scr.h === th) return scr
    scr?.tex.destroy()
    const levels = Math.min(8, Math.floor(Math.log2(Math.max(tw, th))) + 1)
    const tex = device.createTexture({ size: [tw, th], format: HDR, mipLevelCount: levels, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING })
    const lv = (l) => tex.createView({ baseMipLevel: l, mipLevelCount: 1 })
    const levelsDown = []
    for (let l = 1; l < levels; l++) levelsDown.push({ view: lv(l), bg: device.createBindGroup({ layout: down.getBindGroupLayout(0), entries: [{ binding: 0, resource: lv(l - 1) }, { binding: 1, resource: clampS }] }) })
    scr = { w: tw, h: th, tex, view0: lv(0), all: tex.createView(), down: levelsDown, bgs: new Map() }
    return scr
  }

  // ── the plate ──
  let quadPipe = null
  const quadU = [0, 1].map(() => device.createBuffer({ size: 112, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST }))
  const quadBG = new Map()
  function quadBind(k, view, smp) {
    let e = quadBG.get(k)
    if (!e || e.view !== view) {
      e = { view, bg: device.createBindGroup({ layout: quadPipe.getBindGroupLayout(0), entries: [{ binding: 0, resource: { buffer: quadU[k] } }, { binding: 1, resource: view }, { binding: 2, resource: smp }] }) }
      quadBG.set(k, e)
    }
    return e.bg
  }
  function writeQuad(k, pts, W, H, alpha, gain) {
    const q = quadQ(pts)
    const c = pts.flatMap(([x, y]) => [(x / W) * 2 - 1, 1 - (y / H) * 2, 0, 0])
    device.queue.writeBuffer(quadU[k], 0, new Float32Array([...c, ...q, alpha, gain, 0, 0]))
  }

  let frame = 0
  return {
    // The opening: the plate (a video frame, drawn over rect), the desktop's list drawn into the
    // panel's texture (tw × th) and set on quad (screen corners tl, tr, br, bl) at alpha, then the
    // flat post pass. p: { video, rect: [x, y, w, h], quad, alpha, gain } in CSS pixels of W × H.
    async preparePlate() {
      quadPipe ||= await device.createRenderPipelineAsync({
        layout: "auto",
        vertex: { module: device.createShaderModule({ code: QUAD }), entryPoint: "vs" },
        fragment: { module: device.createShaderModule({ code: QUAD }), entryPoint: "fs", targets: [{ format: HDR, blend }] },
        primitive: { topology: "triangle-list" },
      })
    },
    renderPlate(list, tw, th, W, H, grade, fx, p) {
      if (!quadPipe) return false
      grow(Math.max(1, list.length))
      const groups = []
      let cur = null
      for (let i = 0; i < list.length; i++) {
        const e = list[i]
        const col = e.col || [1, 1, 1, 1]
        const cl = e.clip || [-1e9, -1e9, 1e9, 1e9]
        const uv = e.uv || [0, 0, 1, 1]
        data.set([e.x, e.y, e.w, e.h, col[0], col[1], col[2], col[3] ?? 1, e.r ?? 0, e.a ?? 1, e.shadow || 0, e.ring || 0, cl[0], cl[1], cl[2], cl[3], e.g || 0, e.tex ? 1 : 0, 0, 0, uv[0], uv[1], uv[2], uv[3]], i * FLOATS)
        const t = e.tex || white
        if (!cur || cur.t !== t) groups.push((cur = { t, first: i, n: 0 }))
        cur.n++
      }
      if (list.length) device.queue.writeBuffer(sbuf, 0, data, 0, list.length * FLOATS)
      device.queue.writeBuffer(ubufS, 0, new Float32Array([tw, th, 1, 0, grade[0], grade[1], grade[2], 0]))
      const S = screenTarget(tw, th)
      let bgS = S.bgs.get(sbuf)
      if (!bgS) {
        bgS = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [{ binding: 0, resource: { buffer: ubufS } }, { binding: 1, resource: { buffer: sbuf } }] })
        S.bgs.clear()
        S.bgs.set(sbuf, bgS)
      }
      const plate = texFor(p.video)
      plate.view ||= plate.tex.createView()
      const [rx, ry, rw, rh] = p.rect
      writeQuad(0, [[rx, ry], [rx + rw, ry], [rx + rw, ry + rh], [rx, ry + rh]], W, H, 1, 1)
      writeQuad(1, p.quad, W, H, p.alpha, p.gain ?? 1)
      const tex = ctx.getCurrentTexture()
      const R = targets(tex.width, tex.height)
      device.queue.writeBuffer(
        pbuf,
        0,
        new Float32Array([tex.width, tex.height, 1 / tex.width, 1 / tex.height, 0, 0, tex.width, tex.height, 0, fx.bloom ?? 0, fx.threshold ?? 0.6, fx.vignette ?? 0, fx.grain ?? 0, frame++ % 997, fx.exposure ?? 1, fx.saturation ?? 1, fx.warmth ?? 0, fx.contrast ?? 1, 0, 0]),
      )
      const enc = device.createCommandEncoder()
      const sp = enc.beginRenderPass({ colorAttachments: [{ view: S.view0, clearValue: { r: 0.03, g: 0.03, b: 0.035, a: 1 }, loadOp: "clear", storeOp: "store" }] })
      sp.setPipeline(pipeline)
      sp.setBindGroup(0, bgS)
      for (const g of groups) {
        sp.setBindGroup(1, texFor(g.t).bg)
        sp.draw(6, g.n, 0, g.first)
      }
      sp.end()
      for (const l of S.down) {
        const dp = enc.beginRenderPass({ colorAttachments: [{ view: l.view, loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }] })
        dp.setPipeline(down)
        dp.setBindGroup(0, l.bg)
        dp.draw(3)
        dp.end()
      }
      const rp = enc.beginRenderPass({ colorAttachments: [{ view: R.scene.createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 }, loadOp: "clear", storeOp: "store" }] })
      rp.setPipeline(quadPipe)
      rp.setBindGroup(0, quadBind(0, plate.view, clampS))
      rp.draw(6)
      if (p.alpha > 0.001) {
        rp.setBindGroup(0, quadBind(1, S.all, aniso))
        rp.draw(6)
      }
      rp.end()
      for (let l = 0; l < LEVELS; l++) {
        const dp = enc.beginRenderPass({ colorAttachments: [{ view: R.levels[l].createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }] })
        dp.setPipeline(down)
        dp.setBindGroup(0, R.downBG[l])
        dp.draw(3)
        dp.end()
      }
      const pp = enc.beginRenderPass({ colorAttachments: [{ view: tex.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }] })
      pp.setPipeline(post)
      pp.setBindGroup(0, R.postBG)
      pp.draw(3)
      pp.end()
      device.queue.submit([enc.finish()])
      return true
    },
    kind: "webgpu",
    get lost() {
      return lost
    },
    render(list, W, H, dpr, grade, fx = POST_DEFAULT) {
      grow(list.length)
      const groups = []
      let cur = null
      for (let i = 0; i < list.length; i++) {
        const p = list[i]
        const o = i * FLOATS
        const col = p.col || [1, 1, 1, 1]
        const cl = p.clip || [-1e9, -1e9, 1e9, 1e9]
        const uv = p.uv || [0, 0, 1, 1]
        data.set([p.x, p.y, p.w, p.h, col[0], col[1], col[2], col[3] ?? 1, p.r ?? 0, p.a ?? 1, p.shadow || 0, p.ring || 0, cl[0], cl[1], cl[2], cl[3], p.g || 0, p.tex ? 1 : 0, 0, 0, uv[0], uv[1], uv[2], uv[3]], o)
        const t = p.tex || white
        if (!cur || cur.t !== t) groups.push((cur = { t, first: i, n: 0 }))
        cur.n++
      }
      const tw = ctx.getCurrentTexture()
      const R = targets(tw.width, tw.height)
      const k = tw.width / W
      const f = fx.focus ? fx.focus.map((v) => v * k) : [0, 0, tw.width, tw.height]
      device.queue.writeBuffer(ubuf, 0, new Float32Array([W, H, dpr, 0, grade[0], grade[1], grade[2], 0]))
      device.queue.writeBuffer(
        pbuf,
        0,
        new Float32Array([tw.width, tw.height, 1 / tw.width, 1 / tw.height, ...f, fx.dof ?? 0, fx.bloom ?? 0, fx.threshold ?? 0.6, fx.vignette ?? 0, fx.grain ?? 0, frame++ % 997, fx.exposure ?? 1, fx.saturation ?? 1, fx.warmth ?? 0, fx.contrast ?? 1, 0, 0]),
      )
      if (list.length) device.queue.writeBuffer(sbuf, 0, data, 0, list.length * FLOATS)
      const enc = device.createCommandEncoder()
      const pass = enc.beginRenderPass({ colorAttachments: [{ view: R.scene.createView(), clearValue: { r: 0.03, g: 0.03, b: 0.035, a: 1 }, loadOp: "clear", storeOp: "store" }] })
      pass.setPipeline(pipeline)
      pass.setBindGroup(0, bg0)
      for (const g of groups) {
        pass.setBindGroup(1, texFor(g.t).bg)
        pass.draw(6, g.n, 0, g.first)
      }
      pass.end()
      for (let l = 0; l < LEVELS; l++) {
        const dp = enc.beginRenderPass({ colorAttachments: [{ view: R.levels[l].createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }] })
        dp.setPipeline(down)
        dp.setBindGroup(0, R.downBG[l])
        dp.draw(3)
        dp.end()
      }
      const pp = enc.beginRenderPass({ colorAttachments: [{ view: tw.createView(), loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 1 } }] })
      pp.setPipeline(post)
      pp.setBindGroup(0, R.postBG)
      pp.draw(3)
      pp.end()
      device.queue.submit([enc.finish()])
    },
    // Drop textures whose canvases are gone.
    sweep() {
      for (const [c, e] of texs) {
        if (!e.used) {
          e.tex.destroy()
          texs.delete(c)
        } else e.used = false
      }
    },
    done: () => device.queue.onSubmittedWorkDone(),
    destroy() {
      for (const e of texs.values()) e.tex.destroy()
      if (rt) for (const t of [rt.scene, ...rt.levels]) t.destroy()
      sbuf?.destroy()
      ubuf.destroy()
      pbuf.destroy()
      device.destroy()
    },
  }
}
