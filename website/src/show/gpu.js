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
  let lw = max(dot(wide, vec3f(0.2126, 0.7152, 0.0722)), 1e-4);
  c += wide * (max(lw - pu.a.z, 0.0) / lw) * pu.a.y;
  if (three) { c = mix(pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.2)), agx(c), pu.c.z * select(1.0, 0.0, s0.a < 0.0)); }
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

// The room: world-space meshes with baked light. Baked materials add the screen's own light
// (baked separately) scaled by how bright the screen is; the screen samples the desktop.
const MESH = /* wgsl */ `
struct U3 { vp: mat4x4f, eye: vec4f, p: vec4f, sc: vec4f, d: vec4f }
struct M3 { k: vec4f, col: vec4f }
@group(0) @binding(0) var<uniform> u: U3;
@group(1) @binding(0) var<uniform> m: M3;
@group(1) @binding(1) var tb: texture_2d<f32>;
@group(1) @binding(2) var te: texture_2d<f32>;
@group(1) @binding(3) var smp: sampler;
struct VO { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) w: vec3f }
@vertex fn vs(@location(0) p: vec3f, @location(1) uv: vec2f) -> VO {
  var o: VO;
  o.pos = u.vp * vec4f(p, 1.0);
  o.uv = uv;
  o.w = p;
  return o;
}
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let b = textureSample(tb, smp, i.uv).rgb;
  let e = textureSample(te, smp, i.uv).rgb;
  var c = vec3f(0.0);
  let kind = m.k.x;
  if (kind < 0.5) {
    c = b * m.k.y + e * m.k.z * u.p.x * u.sc.rgb;
  } else if (kind < 1.5) {
    // the panel: the desktop it shows, or black glass when it sleeps; a faint sheen at grazing angles
    let lin = pow(max(b, vec3f(0.0)), vec3f(2.2));
    let v = normalize(u.eye.xyz - i.w);
    let f = pow(1.0 - abs(dot(v, m.col.xyz)), 5.0);
    c = lin * u.p.x * u.sc.w + vec3f(0.0018, 0.002, 0.0026) + vec3f(0.02, 0.022, 0.026) * f;
  } else {
    c = m.col.rgb * u.p.y;
  }
  c *= u.p.z;
  let z = distance(u.eye.xyz, i.w);
  let coc = clamp(abs(1.0 / u.d.x - 1.0 / max(z, 0.01)) * u.d.y, 0.0, 1.0);
  // the panel is a display, not a lit surface: its alpha goes negative so the tone curve leaves it alone
  if (kind > 0.5 && kind < 1.5) { return vec4f(c, -max(coc, 0.002)); }
  return vec4f(c, coc);
}
`

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

  // ── the room ──
  const MSAA = 4
  const meshMod = device.createShaderModule({ code: MESH })
  let mesh3 = null // pipeline, created on first scene load
  const u3 = device.createBuffer({ size: 128, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
  const ubufS = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
  const aniso = device.createSampler({ magFilter: "linear", minFilter: "linear", mipmapFilter: "linear", maxAnisotropy: 8, addressModeU: "clamp-to-edge", addressModeV: "clamp-to-edge" })
  const black = device.createTexture({ size: [1, 1], format: "rgba8unorm", usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST })
  device.queue.writeTexture({ texture: black }, new Uint8Array([0, 0, 0, 255]), {}, [1, 1])
  const scenes = new Map()
  let scr = null // the desktop's texture on the panel, with its mips
  let ms = null // multisampled colour and depth

  async function imageTexture(bmp) {
    const levels = Math.floor(Math.log2(Math.max(bmp.width, bmp.height))) + 1
    const tex = device.createTexture({ size: [bmp.width, bmp.height], format: "rgba8unorm-srgb", mipLevelCount: levels, usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT })
    device.queue.copyExternalImageToTexture({ source: bmp }, { texture: tex }, [bmp.width, bmp.height])
    for (let l = 1; l < levels; l++) {
      const w = Math.max(1, bmp.width >> l)
      const h = Math.max(1, bmp.height >> l)
      const small = await createImageBitmap(bmp, { resizeWidth: w, resizeHeight: h, resizeQuality: "high", colorSpaceConversion: "none" })
      device.queue.copyExternalImageToTexture({ source: small }, { texture: tex, mipLevel: l }, [w, h])
      small.close?.()
    }
    return tex
  }

  // corners are [bl, br, tr, tl]; u runs tl → tr, v runs tl → bl
  function panelUV(pos, [bl, , tr, tl]) {
    const ex = [tr[0] - tl[0], tr[1] - tl[1], tr[2] - tl[2]]
    const ey = [bl[0] - tl[0], bl[1] - tl[1], bl[2] - tl[2]]
    const lx = ex[0] ** 2 + ex[1] ** 2 + ex[2] ** 2
    const ly = ey[0] ** 2 + ey[1] ** 2 + ey[2] ** 2
    const uv = new Float32Array((pos.length / 3) * 2)
    for (let i = 0; i < pos.length / 3; i++) {
      const d = [pos[i * 3] - tl[0], pos[i * 3 + 1] - tl[1], pos[i * 3 + 2] - tl[2]]
      uv[i * 2] = (d[0] * ex[0] + d[1] * ex[1] + d[2] * ex[2]) / lx
      uv[i * 2 + 1] = (d[0] * ey[0] + d[1] * ey[1] + d[2] * ey[2]) / ly
    }
    return uv
  }

  function screenTarget(tw, th) {
    if (scr && scr.w === tw && scr.h === th) return scr
    scr?.tex.destroy()
    const levels = Math.min(8, Math.floor(Math.log2(Math.max(tw, th))) + 1)
    const tex = device.createTexture({ size: [tw, th], format: HDR, mipLevelCount: levels, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING })
    const lv = (l) => tex.createView({ baseMipLevel: l, mipLevelCount: 1 })
    const down = []
    for (let l = 1; l < levels; l++) down.push({ view: lv(l), bg: device.createBindGroup({ layout: downPipe.getBindGroupLayout(0), entries: [{ binding: 0, resource: lv(l - 1) }, { binding: 1, resource: clampS }] }) })
    scr = { w: tw, h: th, tex, view0: lv(0), all: tex.createView(), down, bgs: new Map() }
    return scr
  }
  const downPipe = down

  function msTargets(w, h) {
    if (ms && ms.w === w && ms.h === h) return ms
    ms?.color.destroy()
    ms?.depth.destroy()
    ms = {
      w,
      h,
      color: device.createTexture({ size: [w, h], format: HDR, sampleCount: MSAA, usage: GPUTextureUsage.RENDER_ATTACHMENT }),
      depth: device.createTexture({ size: [w, h], format: "depth24plus", sampleCount: MSAA, usage: GPUTextureUsage.RENDER_ATTACHMENT }),
    }
    return ms
  }

  let frame = 0
  return {
    // Uploads a loaded GLB (see glb.js). scale/emitScale undo the bake's JPEG normalisation per material.
    async loadScene(key, glb, meta) {
      if (scenes.has(key)) return
      if (!mesh3)
        mesh3 = await device.createRenderPipelineAsync({
          layout: "auto",
          vertex: { module: meshMod, entryPoint: "vs", buffers: [{ arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: "float32x3" }] }, { arrayStride: 8, attributes: [{ shaderLocation: 1, offset: 0, format: "float32x2" }] }] },
          fragment: { module: meshMod, entryPoint: "fs", targets: [{ format: HDR }] },
          primitive: { topology: "triangle-list", cullMode: "none" },
          depthStencil: { format: "depth24plus", depthWriteEnabled: true, depthCompare: "less" },
          multisample: { count: MSAA },
        })
      const draws = []
      for (const m of glb.meshes) {
        if (m.material === "Glass") continue
        const kind = m.material === "Screen" ? 1 : m.emit && !m.base ? 2 : m.base ? 0 : 2
        const vb = device.createBuffer({ size: m.pos.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
        device.queue.writeBuffer(vb, 0, m.pos)
        // the panel's UVs come from its corners (study.json), so the desktop lands on it exactly
        const uv = kind === 1 && meta.screen ? panelUV(m.pos, meta.screen) : m.uv
        const ub = device.createBuffer({ size: uv.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
        device.queue.writeBuffer(ub, 0, uv)
        const idx = m.index instanceof Uint16Array && m.index.length % 2 ? new Uint16Array([...m.index, 0]) : m.index
        const ib = device.createBuffer({ size: idx.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
        device.queue.writeBuffer(ib, 0, idx)
        const mb = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
        let normal = [0, 0, 1]
        if (kind === 1) {
          const p = m.pos
          const a = [p[3] - p[0], p[4] - p[1], p[5] - p[2]]
          const b = [p[6] - p[0], p[7] - p[1], p[8] - p[2]]
          const n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
          const l = Math.hypot(...n) || 1
          normal = n.map((v) => v / l)
        }
        const col = kind === 2 ? m.emitFactor.map((v) => v * (meta.emitStrength?.[m.material] ?? 3)) : normal
        device.queue.writeBuffer(mb, 0, new Float32Array([kind, 1 / (meta.scale?.[m.material] || 1), 1 / (meta.emitScale?.[m.material] || 1), 0, ...col, 0]))
        const tb = kind === 0 && m.base ? await imageTexture(m.base) : null
        const te = kind === 0 && m.emit ? await imageTexture(m.emit) : null
        draws.push({ name: m.name, kind, vb, ub, ib, fmt: idx instanceof Uint16Array ? "uint16" : "uint32", n: m.index.length, mb, tb, te, bg: null })
      }
      for (const d of draws)
        if (d.kind !== 1)
          d.bg = device.createBindGroup({ layout: mesh3.getBindGroupLayout(1), entries: [{ binding: 0, resource: { buffer: d.mb } }, { binding: 1, resource: (d.tb || black).createView() }, { binding: 2, resource: (d.te || black).createView() }, { binding: 3, resource: aniso }] })
      const bg0 = device.createBindGroup({ layout: mesh3.getBindGroupLayout(0), entries: [{ binding: 0, resource: { buffer: u3 } }] })
      scenes.set(key, { draws, bg0, hidden: new Set() })
    },
    hasScene: (key) => scenes.has(key),
    // Show or hide one of a scene's meshes by its node name (the basement's two states).
    setVisible(key, name, on) {
      const sc = scenes.get(key)
      if (sc) on ? sc.hidden.delete(name) : sc.hidden.add(name)
    },
    // A 3D frame: the desktop's list is drawn into the panel's texture (tw × th), then the room.
    // three: { key, vp, eye, glow, lit, exposure, screenCol, focus, aperture, tm }
    render3d(list, tw, th, W, H, grade, fx, three) {
      const sc = scenes.get(three.key)
      if (!sc) return
      grow(Math.max(1, list.length))
      const groups = []
      let cur = null
      for (let i = 0; i < list.length; i++) {
        const p = list[i]
        const col = p.col || [1, 1, 1, 1]
        const cl = p.clip || [-1e9, -1e9, 1e9, 1e9]
        const uv = p.uv || [0, 0, 1, 1]
        data.set([p.x, p.y, p.w, p.h, col[0], col[1], col[2], col[3] ?? 1, p.r ?? 0, p.a ?? 1, p.shadow || 0, p.ring || 0, cl[0], cl[1], cl[2], cl[3], p.g || 0, p.tex ? 1 : 0, 0, 0, uv[0], uv[1], uv[2], uv[3]], i * FLOATS)
        const t = p.tex || white
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
      for (const d of sc.draws)
        if (d.kind === 1 && d.scr !== S) {
          d.scr = S
          d.bg = device.createBindGroup({ layout: mesh3.getBindGroupLayout(1), entries: [{ binding: 0, resource: { buffer: d.mb } }, { binding: 1, resource: S.all }, { binding: 2, resource: black.createView() }, { binding: 3, resource: aniso }] })
        }
      const tex = ctx.getCurrentTexture()
      const R = targets(tex.width, tex.height)
      const M = msTargets(tex.width, tex.height)
      const t = three
      device.queue.writeBuffer(u3, 0, new Float32Array([...t.vp, ...t.eye, 0, t.glow, t.lit, t.exposure, 0, ...t.screenCol, t.focus, t.aperture, 0, 0]))
      const f = [0, 0, tex.width, tex.height]
      device.queue.writeBuffer(
        pbuf,
        0,
        new Float32Array([tex.width, tex.height, 1 / tex.width, 1 / tex.height, ...f, fx.dof ?? 0, fx.bloom ?? 0, fx.threshold ?? 0.6, fx.vignette ?? 0, fx.grain ?? 0, frame++ % 997, fx.exposure ?? 1, fx.saturation ?? 1, fx.warmth ?? 0, fx.contrast ?? 1, t.tm, 1]),
      )
      const enc = device.createCommandEncoder()
      // the desktop, then its mips (so it stays calm when it is small in frame)
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
      // the room
      const rp = enc.beginRenderPass({
        colorAttachments: [{ view: M.color.createView(), resolveTarget: R.scene.createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: "clear", storeOp: "discard" }],
        depthStencilAttachment: { view: M.depth.createView(), depthClearValue: 1, depthLoadOp: "clear", depthStoreOp: "discard" },
      })
      rp.setPipeline(mesh3)
      rp.setBindGroup(0, sc.bg0)
      for (const d of sc.draws) {
        if (sc.hidden.has(d.name)) continue
        rp.setBindGroup(1, d.bg)
        rp.setVertexBuffer(0, d.vb)
        rp.setVertexBuffer(1, d.ub)
        rp.setIndexBuffer(d.ib, d.fmt)
        rp.drawIndexed(d.n)
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
