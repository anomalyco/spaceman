// Web Audio for the show. Nothing is fetched before the key press. The bank is small (about
// 500 KB), so it is fetched in one go on start; the show begins once the first lines are decoded
// or after 2.5 s, whichever comes first. Every play() is logged so captures can be mixed offline.
export class Audio {
  constructor(base, { log = false } = {}) {
    this.base = base
    this.log = log ? [] : null
    this.ctx = null
    this.bufs = new Map()
    this.bytes = new Map()
    this.muted = false
    this.hidden = false
    this.manifest = null
    this.room = null
    this.live = new Set()
  }

  attach(ctx, muted) {
    this.muted = muted
    if (ctx) this.build(ctx)
  }

  build(ctx) {
    if (this.ctx) return
    this.ctx = ctx
    this.master = ctx.createGain()
    this.master.gain.value = 0.9
    const comp = ctx.createDynamicsCompressor()
    comp.threshold.value = -10
    comp.ratio.value = 4
    this.master.connect(comp).connect(ctx.destination)
    this.roomGain = ctx.createGain()
    this.roomGain.gain.value = 0
    this.roomGain.connect(this.master)
    for (const f of this.bytes.keys()) void this.decode(f)
    this.sync()
  }

  async load(manifest) {
    this.manifest = manifest
    const files = new Set(Object.values(manifest.sfx || {}).map((s) => s.file || s))
    for (const line of Object.values(manifest.voice)) files.add(line.takes[line.pick ?? 0]?.file ?? line.takes[0].file)
    const all = Promise.all([...files].map((f) => this.fetchOne(f)))
    await Promise.race([all, new Promise((r) => setTimeout(r, 2500))])
  }

  async fetchOne(file) {
    if (this.bytes.has(file)) return
    try {
      const res = await fetch(this.base + file)
      if (!res.ok) return
      this.bytes.set(file, await res.arrayBuffer())
      if (this.ctx) await this.decode(file)
    } catch {}
  }

  async decode(file) {
    const b = this.bytes.get(file)
    if (!(b instanceof ArrayBuffer) || this.bufs.has(file)) return
    this.bufs.set(file, null)
    try {
      this.bufs.set(file, await this.ctx.decodeAudioData(b.slice(0)))
    } catch {
      this.bufs.delete(file)
    }
  }

  get on() {
    return !!this.ctx && !this.muted && !this.hidden
  }

  setMuted(muted, ctx) {
    this.muted = muted
    if (!muted && ctx) this.build(ctx)
    this.sync()
  }

  setHidden(hidden) {
    this.hidden = hidden
    this.sync()
  }

  sync() {
    if (!this.ctx) return
    if (this.muted || this.hidden) this.ctx.suspend().catch(() => {})
    else this.ctx.resume().catch(() => {})
  }

  // t is show time, for the log. offset starts a clip part-way through (after an unmute).
  play(file, { t = 0, gain = 1, offset = 0, rate = 1, loop = false, kind = "sfx" } = {}) {
    this.log?.push({ t: +t.toFixed(3), file, gain, offset: +offset.toFixed(3), rate, loop, kind })
    if (!this.on) return null
    const buf = this.bufs.get(file)
    if (!buf || offset >= buf.duration) return null
    const src = this.ctx.createBufferSource()
    src.buffer = buf
    src.loop = loop
    src.playbackRate.value = rate
    const g = this.ctx.createGain()
    g.gain.value = gain
    src.connect(g).connect(loop ? this.roomGain : this.master)
    src.start(0, offset)
    const h = { src, g, file }
    this.live.add(h)
    src.onended = () => this.live.delete(h)
    return h
  }

  voiceFile(id) {
    const line = this.manifest?.voice[id]
    if (!line) return null
    return line.takes[line.pick ?? 0] ?? line.takes[0]
  }

  // The room: a loop whose level the show rides.
  roomLevel(v, t, tc = 0.6) {
    this.log?.push({ t: +t.toFixed(3), file: "room-level", gain: v, kind: "level" })
    if (!this.on) return
    if (!this.room) this.room = this.play("sfx-room.mp3", { t, loop: true, gain: 1, kind: "room" })
    this.roomGain.gain.setTargetAtTime(v, this.ctx.currentTime, tc)
  }

  stopAll() {
    for (const h of this.live) {
      try {
        h.src.stop()
      } catch {}
    }
    this.live.clear()
    this.room = null
  }

  close() {
    this.stopAll()
    this.ctx?.close().catch(() => {})
    this.ctx = null
  }
}
