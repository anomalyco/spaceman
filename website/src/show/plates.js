// Pictures inside his apps and the room's stills: Blender renders of our own scenes
// (show/scenes/*, public/show/renders), fetched on the key press. Each plate is an ImageBitmap with a .version, so both renderers can
// use it as a texture; blurOf() gives a soft copy for frosted materials.
export const plates = {}
const waiting = new Map()

export function loadPlate(base, name) {
  if (plates[name]) return Promise.resolve(plates[name])
  if (waiting.has(name)) return waiting.get(name)
  const p = fetch(base + "renders/" + name + ".jpg")
    .then((r) => (r.ok ? r.blob() : Promise.reject(new Error(String(r.status)))))
    .then((b) => createImageBitmap(b))
    .then((bm) => {
      bm.version = 1
      plates[name] = bm
      return bm
    })
    .catch(() => null)
  waiting.set(name, p)
  return p
}

const blurs = new Map()
// A blurred copy, the way a frosted sidebar sees the desktop.
export function blurOf(img, w = 320) {
  let c = blurs.get(img)
  if (c) return c
  const h = Math.round((w * img.height) / img.width)
  const tiny = document.createElement("canvas")
  tiny.width = Math.max(8, Math.round(w / 10))
  tiny.height = Math.max(5, Math.round(h / 10))
  const t = tiny.getContext("2d")
  t.imageSmoothingQuality = "high"
  t.drawImage(img, 0, 0, tiny.width, tiny.height)
  c = document.createElement("canvas")
  c.width = w
  c.height = h
  const x = c.getContext("2d")
  x.imageSmoothingQuality = "high"
  // two upscales from the tiny copy: a cheap, wide, smooth blur
  const mid = document.createElement("canvas")
  mid.width = tiny.width * 3
  mid.height = tiny.height * 3
  const m = mid.getContext("2d")
  m.imageSmoothingQuality = "high"
  m.drawImage(tiny, 0, 0, mid.width, mid.height)
  x.drawImage(mid, 0, 0, w, h)
  c.version = 1
  blurs.set(img, c)
  return c
}

// The uv rect that covers an aw x ah area with an image, centred on (fx, fy).
export function coverUV(img, aw, ah, fx = 0.5, fy = 0.5) {
  const ia = img.width / img.height
  const a = aw / ah
  if (a > ia) {
    const vh = ia / a
    const v0 = Math.min(1 - vh, Math.max(0, fy - vh / 2))
    return [0, v0, 1, v0 + vh]
  }
  const uw = a / ia
  const u0 = Math.min(1 - uw, Math.max(0, fx - uw / 2))
  return [u0, 0, u0 + uw, 1]
}
