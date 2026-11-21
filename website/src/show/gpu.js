// WebGPU renderer: one instanced rounded-rect pipeline. Primitives come from the display list in
// draw order; consecutive primitives sharing a texture go out in one draw call. Canvas textures are
// re-uploaded only when their version changes.
const FLOATS = 20 // five vec4f per primitive, matching struct P

const WGSL = /* wgsl */ `
struct U { view: vec4f, grade: vec4f }
struct P { rect: vec4f, col: vec4f, p: vec4f, clip: vec4f, q: vec4f }
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
      let uv = (w - p.rect.xy) / p.rect.zw;
      let t = textureSampleLevel(tex, samp, uv, 0.0);
      // texture is premultiplied; col tints it
      if (t.a > 0.0001) { rgb = t.rgb / t.a * p.col.rgb; } else { rgb = vec3f(0.0); }
      a = t.a * p.col.a;
    }
  }
  // grade: g.x darkness, g.y coolness, g.z desaturation
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

export async function createGPU(canvas) {
  if (!navigator.gpu) return null
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" })
  if (!adapter) return null
  const device = await adapter.requestDevice()
  const ctx = canvas.getContext("webgpu")
  if (!ctx) return null
  const format = navigator.gpu.getPreferredCanvasFormat()
  ctx.configure({ device, format, alphaMode: "opaque" })
  const module = device.createShaderModule({ code: WGSL })
  const blend = { color: { srcFactor: "one", dstFactor: "one-minus-src-alpha" }, alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" } }
  const pipeline = await device.createRenderPipelineAsync({
    layout: "auto",
    vertex: { module, entryPoint: "vs" },
    fragment: { module, entryPoint: "fs", targets: [{ format, blend }] },
    primitive: { topology: "triangle-list" },
  })
  const ubuf = device.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
  let cap = 0
  let sbuf = null
  let bg0 = null
  let data = new Float32Array(0)
  const sampler = device.createSampler({ magFilter: "linear", minFilter: "linear" })
  const texs = new Map() // canvas -> { tex, bg, version, w, h }
  const white = document.createElement("canvas")
  white.width = white.height = 2
  const wc = white.getContext("2d")
  wc.fillStyle = "#fff"
  wc.fillRect(0, 0, 2, 2)
  white.version = 1
  let lost = false
  device.lost.then(() => (lost = true))

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

  return {
    kind: "webgpu",
    get lost() {
      return lost
    },
    render(list, W, H, dpr, grade) {
      grow(list.length)
      const groups = []
      let cur = null
      for (let i = 0; i < list.length; i++) {
        const p = list[i]
        const o = i * FLOATS
        const col = p.col || [1, 1, 1, 1]
        const cl = p.clip || [-1e9, -1e9, 1e9, 1e9]
        data.set([p.x, p.y, p.w, p.h, col[0], col[1], col[2], col[3] ?? 1, p.r ?? 0, p.a ?? 1, p.shadow || 0, p.ring || 0, cl[0], cl[1], cl[2], cl[3], p.g || 0, p.tex ? 1 : 0, 0, 0], o)
        const t = p.tex || white
        if (!cur || cur.t !== t) groups.push((cur = { t, first: i, n: 0 }))
        cur.n++
      }
      device.queue.writeBuffer(ubuf, 0, new Float32Array([W, H, dpr, 0, grade[0], grade[1], grade[2], 0]))
      if (list.length) device.queue.writeBuffer(sbuf, 0, data, 0, list.length * FLOATS)
      const enc = device.createCommandEncoder()
      const pass = enc.beginRenderPass({ colorAttachments: [{ view: ctx.getCurrentTexture().createView(), clearValue: { r: 0.03, g: 0.03, b: 0.035, a: 1 }, loadOp: "clear", storeOp: "store" }] })
      pass.setPipeline(pipeline)
      pass.setBindGroup(0, bg0)
      for (const g of groups) {
        pass.setBindGroup(1, texFor(g.t).bg)
        pass.draw(6, g.n, 0, g.first)
      }
      pass.end()
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
      sbuf?.destroy()
      ubuf.destroy()
      device.destroy()
    },
  }
}
