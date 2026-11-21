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
@fragment fn fs(i: VO) -> @location(0) vec4f {
  let px = i.uv * pu.res.xy;
  var c = textureSampleLevel(scene, samp, i.uv, 0.0).rgb;
  // depth of field: sharp on the subject, softer with distance from it
  let dof = pu.a.x;
  if (dof > 0.001) {
    let q = max(max(pu.focus.xy - px, px - pu.focus.zw), vec2f(0.0));
    let coc = dof * smoothstep(0.0, 0.42 * min(pu.res.x, pu.res.y), length(q));
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

  let frame = 0
  return {
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
