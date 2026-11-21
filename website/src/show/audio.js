// Web Audio for the show. Nothing is fetched before the key press. On start the tape and the
// effects are fetched in one go (about 830 KB); the film begins once they are decoded or after
// 2.5 s, whichever is first, and the tape joins late at the right offset if it was still on its
// way. Every play() is logged so captures can be mixed offline.
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
    this.beds = new Map() // file -> { gain, h, level }
    this.live = new Set()
    this.onReady = null
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
    for (const f of this.bytes.keys()) void this.decode(f)
    this.sync()
  }

  get tape() {
    return this.manifest?.voice?.tape?.takes?.[0] ?? null
  }

  async load(manifest) {
    this.manifest = manifest
    const files = new Set(Object.values(manifest.sfx || {}).map((s) => s.file || s))
    if (this.tape) files.add(this.tape.file)
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
      this.onReady?.(file)
    } catch {
      this.bufs.delete(file)
    }
  }

  get on() {
    return !!this.ctx && !this.muted && !this.hidden
  }

  has(file) {
    return !!this.bufs.get(file)
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

  // t is show time, for the log. offset starts a clip part-way through.
  play(file, { t = 0, gain = 1, offset = 0, rate = 1, loop = false, kind = "sfx", dest = null } = {}) {
    this.log?.push({ t: +t.toFixed(3), file, gain, offset: +offset.toFixed(3), rate, loop, kind })
    if (!this.on) return null
    const buf = this.bufs.get(file)
    if (!buf || (!loop && offset >= buf.duration)) return null
    const src = this.ctx.createBufferSource()
    src.buffer = buf
    src.loop = loop
    src.playbackRate.value = rate
    const g = this.ctx.createGain()
    g.gain.value = gain
    src.connect(g).connect(dest || this.master)
    src.start(0, loop ? offset % buf.duration : offset)
    const h = { src, g, file }
    this.live.add(h)
    src.onended = () => this.live.delete(h)
    return h
  }

  // A bed: a loop whose level the show rides (the study, the night outside).
  bed(file, v, t, tc = 0.6) {
    this.log?.push({ t: +t.toFixed(3), file, gain: v, kind: "level" })
    let b = this.beds.get(file)
    if (!b) this.beds.set(file, (b = { level: 0, h: null, gain: null }))
    b.level = v
    if (!this.on) return
    if (!b.gain) {
      b.gain = this.ctx.createGain()
      b.gain.gain.value = 0
      b.gain.connect(this.master)
    }
    if (!b.h && this.has(file)) b.h = this.play(file, { t, loop: true, kind: "bed", dest: b.gain })
    b.gain.gain.setTargetAtTime(v, this.ctx.currentTime, tc)
  }

  // After an unmute or a late decode: bring the beds back at their levels.
  restoreBeds(t) {
    for (const [file, b] of this.beds) if (b.level > 0) this.bed(file, b.level, t, 0.3)
  }

  stopAll() {
    for (const h of this.live) {
      try {
        h.src.stop()
      } catch {}
    }
    this.live.clear()
    for (const b of this.beds.values()) {
      b.h = null
      b.gain?.disconnect()
      b.gain = null
    }
  }

  close() {
    this.stopAll()
    this.ctx?.close().catch(() => {})
    this.ctx = null
  }
}
