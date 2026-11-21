// A small GLB reader for the film's own scenes (show/scenes/*): world-space meshes with their
// materials' base and emissive images, and the cameras. Only what our exporter writes is supported:
// float positions and UVs, 16/32-bit indices, embedded JPEGs.
const COMP = { 5126: Float32Array, 5125: Uint32Array, 5123: Uint16Array, 5121: Uint8Array }
const SIZE = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 }

export async function loadGLB(url) {
  const buf = await (await fetch(url)).arrayBuffer()
  const dv = new DataView(buf)
  if (dv.getUint32(0, true) !== 0x46546c67) throw new Error("not a glb: " + url)
  let off = 12
  let json = null
  let bin = null
  while (off < buf.byteLength) {
    const len = dv.getUint32(off, true)
    const type = dv.getUint32(off + 4, true)
    if (type === 0x4e4f534a) json = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, off + 8, len)))
    else if (type === 0x004e4942) bin = new Uint8Array(buf, off + 8, len)
    off += 8 + len
  }
  const view = (i) => {
    const v = json.bufferViews[i]
    return new Uint8Array(bin.buffer, bin.byteOffset + (v.byteOffset || 0), v.byteLength)
  }
  const accessor = (i) => {
    const a = json.accessors[i]
    const v = json.bufferViews[a.bufferView]
    const T = COMP[a.componentType]
    const n = a.count * SIZE[a.type]
    const start = bin.byteOffset + (v.byteOffset || 0) + (a.byteOffset || 0)
    if (v.byteStride && v.byteStride !== SIZE[a.type] * T.BYTES_PER_ELEMENT) throw new Error("strided accessor")
    return new T(bin.buffer.slice(start, start + n * T.BYTES_PER_ELEMENT))
  }
  const images = new Map()
  const image = (texInfo) => {
    if (!texInfo) return null
    const src = json.textures[texInfo.index].source
    if (!images.has(src)) {
      const im = json.images[src]
      images.set(src, createImageBitmap(new Blob([view(im.bufferView)], { type: im.mimeType }), { colorSpaceConversion: "none" }))
    }
    return images.get(src)
  }
  const meshes = []
  const cameras = {}
  const walk = (ni, parent) => {
    const n = json.nodes[ni]
    const m = mul(parent, local(n))
    if (n.mesh != null)
      for (const p of json.meshes[n.mesh].primitives) {
        const mat = json.materials?.[p.material] || {}
        const pos = accessor(p.attributes.POSITION)
        for (let k = 0; k < pos.length; k += 3) {
          const [x, y, z] = [pos[k], pos[k + 1], pos[k + 2]]
          pos[k] = m[0] * x + m[4] * y + m[8] * z + m[12]
          pos[k + 1] = m[1] * x + m[5] * y + m[9] * z + m[13]
          pos[k + 2] = m[2] * x + m[6] * y + m[10] * z + m[14]
        }
        meshes.push({
          name: n.name,
          material: mat.name || "",
          pos,
          uv: p.attributes.TEXCOORD_0 != null ? accessor(p.attributes.TEXCOORD_0) : new Float32Array((pos.length / 3) * 2),
          index: accessor(p.indices),
          base: image(mat.pbrMetallicRoughness?.baseColorTexture),
          emit: image(mat.emissiveTexture),
          baseFactor: mat.pbrMetallicRoughness?.baseColorFactor || [1, 1, 1, 1],
          emitFactor: mat.emissiveFactor || [0, 0, 0],
        })
      }
    if (n.camera != null) {
      const c = json.cameras[n.camera].perspective
      cameras[n.name] = { pos: [m[12], m[13], m[14]], dir: norm([-m[8], -m[9], -m[10]]), up: norm([m[4], m[5], m[6]]), vfov: c.yfov }
    }
    for (const c of n.children || []) walk(c, m)
  }
  for (const ni of json.scenes[json.scene || 0].nodes) walk(ni, IDENT)
  await Promise.all(meshes.flatMap((m) => [m.base, m.emit].filter(Boolean)))
  for (const m of meshes) {
    m.base = m.base && (await m.base)
    m.emit = m.emit && (await m.emit)
  }
  return { meshes, cameras }
}

const IDENT = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
function local(n) {
  if (n.matrix) return n.matrix
  const [x, y, z, w] = n.rotation || [0, 0, 0, 1]
  const [sx, sy, sz] = n.scale || [1, 1, 1]
  const [tx, ty, tz] = n.translation || [0, 0, 0]
  return [
    (1 - 2 * (y * y + z * z)) * sx, 2 * (x * y + z * w) * sx, 2 * (x * z - y * w) * sx, 0,
    2 * (x * y - z * w) * sy, (1 - 2 * (x * x + z * z)) * sy, 2 * (y * z + x * w) * sy, 0,
    2 * (x * z + y * w) * sz, 2 * (y * z - x * w) * sz, (1 - 2 * (x * x + y * y)) * sz, 0,
    tx, ty, tz, 1,
  ]
}
export function mul(a, b) {
  const o = new Array(16)
  for (let c = 0; c < 4; c++)
    for (let r = 0; r < 4; r++) o[c * 4 + r] = a[r] * b[c * 4] + a[4 + r] * b[c * 4 + 1] + a[8 + r] * b[c * 4 + 2] + a[12 + r] * b[c * 4 + 3]
  return o
}
export const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
export const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
export const scale = (a, s) => [a[0] * s, a[1] * s, a[2] * s]
export const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
export const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
export function norm(a) {
  const l = Math.hypot(a[0], a[1], a[2]) || 1
  return [a[0] / l, a[1] / l, a[2] / l]
}
export const lerp3 = (a, b, u) => [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u]

// WebGPU clip space (depth 0..1), column-major
export function perspective(vfov, aspect, near, far) {
  const f = 1 / Math.tan(vfov / 2)
  const nf = 1 / (near - far)
  return [f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, far * nf, -1, 0, 0, far * near * nf, 0]
}
export function lookAt(eye, target, up) {
  const z = norm(sub(eye, target))
  const x = norm(cross(up, z))
  const y = cross(z, x)
  return [x[0], y[0], z[0], 0, x[1], y[1], z[1], 0, x[2], y[2], z[2], 0, -dot(x, eye), -dot(y, eye), -dot(z, eye), 1]
}
