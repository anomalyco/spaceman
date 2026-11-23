// Deterministic capture of the show from a static build in website/dist. Never plays audio.
// Steps the clock frame by frame, screenshots each frame, encodes a 30 fps clip with an offline
// mix of the audio play log, and writes keyframe strips for the WebGPU, Canvas 2D, reduced-motion
// and phone paths, plus real-time frame stats for the busiest section. Raw frames are deleted.
// usage: bun capture.ts <dist-dir> <out-dir> [--quick]
import { chromium, type Page } from "playwright"
import { mkdirSync, rmSync, writeFileSync, existsSync } from "node:fs"
import { join, resolve } from "node:path"
import { $ } from "bun"
import { encode, grid, mixAudio } from "./encode.ts"
import { END as END_, HOLD, word } from "../../src/show/director.js"
import { LINES } from "../../src/show/tape.js"

const [distArg = "../../dist", outArg = "cap"] = process.argv.slice(2).filter((a) => !a.startsWith("--"))
const quick = process.argv.includes("--quick")
// --trace: step the whole film without screenshots (motion trace + trace-check), then the real-time perf pass
const traceOnly = process.argv.includes("--trace")
const dist = resolve(distArg)
const out = resolve(outArg)
const audioDir = join(dist, "show/audio")
mkdirSync(out, { recursive: true })

const server = Bun.serve({
  port: 0,
  async fetch(req) {
    let p = decodeURIComponent(new URL(req.url).pathname)
    if (p.endsWith("/")) p += "index.html"
    const f = Bun.file(join(dist, p))
    return (await f.exists()) ? new Response(f) : new Response("not found", { status: 404 })
  },
})
const site = `http://127.0.0.1:${server.port}`
const browser = await chromium.launch({ headless: true, channel: "chromium", args: ["--enable-unsafe-webgpu", "--enable-features=Vulkan", "--use-angle=metal", "--mute-audio", "--autoplay-policy=no-user-gesture-required"] })
const report: Record<string, unknown> = { site }
const FPS = 30
const END = END_ + 0.6
// keyframes: the middle of every other sentence, and every sentence from seven on
const sig = word("I'd kept seven", 0)
const KEYS = LINES.filter((l: any, i: number) => i % 2 === 0 || l[1] >= sig - 0.1).map((l: any) => +((l[1] + l[2]) / 2).toFixed(2))
const X2 = [1.5, 4.5, ...["It tiles", "numbered workspaces", "front door", "pleasing", "So I bought", "Mark is four", "Joan is five", "Alan's hedge", "for the record", "bins go out", "I'd kept seven", "in March", "her cat", "in June", "other two", "Only the one", "You should try"].map((p) => +(word(p, 0) + 0.9).toFixed(2))]
// --probe t1,t2,... : only these moments, 1440×900 WebGPU and Canvas 2D, then stop
const probeArg = process.argv.find((a) => a.startsWith("--probe="))

type Opts = { w?: number; h?: number; dpr?: number; query?: string; mobile?: boolean; muted?: boolean; reduced?: boolean }
async function open(o: Opts = {}) {
  const ctx = await browser.newContext({
    viewport: { width: o.w ?? 1280, height: o.h ?? 800 },
    deviceScaleFactor: o.dpr ?? 1,
    isMobile: o.mobile,
    hasTouch: o.mobile,
    reducedMotion: o.reduced ? "reduce" : "no-preference",
  })
  if (o.muted) await ctx.addInitScript(() => localStorage.setItem("spaceman-show-muted", "1"))
  const page = await ctx.newPage()
  const errors: string[] = []
  page.on("pageerror", (e) => errors.push(e.message))
  page.on("console", (m) => (m.type() === "error" || m.type() === "warning") && errors.push(m.type() + ": " + m.text()))
  const requests: string[] = []
  page.on("request", (r) => requests.push(r.url().replace(site, "")))
  await page.goto(`${site}/?capture${o.query ? "&" + o.query : ""}`)
  await page.waitForFunction(() => "__show" in window)
  await page.evaluate(() => document.fonts.ready)
  return { page, errors, requests, ctx }
}
const S = (page: Page, js: string) => page.evaluate(js)
const step = (page: Page, dt: number, n = 1) => page.evaluate(([dt, n]) => (window as any).__show.step(dt, n), [dt, n])
const state = (page: Page) => page.evaluate(() => (window as any).__show.state())
async function press(page: Page, mobile = false) {
  if (mobile) await page.tap('section [role="button"][aria-label]')
  else await page.keyboard.press("Alt+Digit1")
  await page.waitForFunction(() => document.querySelector("section")?.dataset.phase === "live", null, { timeout: 20000 })
}
const shot = (page: Page, path: string) => page.screenshot({ path, type: "jpeg", quality: 86 })

if (probeArg) {
  const times = probeArg.slice(8).split(",").map(Number)
  // --phone adds a pass at an iPhone's size
  const passes: [string, string, any][] = [["gpu", "", { w: 1440, h: 900 }], ["c2d", "canvas2d", { w: 1440, h: 900 }]]
  if (process.argv.includes("--phone")) passes.push(["phone", "", { w: 390, h: 844, dpr: 3, mobile: true }])
  // --x2: the WebGPU pass at 2x, as PNG, to set beside full-size targets at 100%
  const x2 = process.argv.includes("--x2")
  if (x2) passes[0][2] = { w: 1440, h: 900, dpr: 2 }
  for (const [name, q, size] of passes) {
    const { page, errors, ctx } = await open({ ...size, muted: true, query: q })
    if (name === "gpu") await shot(page, join(out, "still.jpg"))
    await press(page)
    await page.addStyleTag({ content: "[data-ui=hint],[data-ui=controls]{display:none!important}" })
    for (const t of times) {
      await page.evaluate((t) => (window as any).__show.seek(t), t)
      // past the end: the card, with its confetti at that moment of the burst
      if (t > END_ + 0.75) {
        await page.waitForTimeout(120)
        await page.evaluate((s) => (window as any).__confetti?.(s), t - END_ - 0.75)
      }
      const file = join(out, name + "-" + String(t).replace(".", "_"))
      if (x2 && name === "gpu") await page.screenshot({ path: file + ".png" })
      else await shot(page, file + ".jpg")
    }
    report[name] = { errors, state: await state(page) }
    // written after each pass, so a crash in the next one still leaves this one's report
    writeFileSync(join(out, "report.json"), JSON.stringify(report, null, 1))
    await ctx.close()
  }
  console.log(JSON.stringify(report, null, 1))
  await browser.close()
  server.stop()
  process.exit(0)
}

// --master: the whole film as a file to share, at 1440×900 @2x (2880×1800), straight into the encoder
// (no frames on disk): film-master.mp4 (HEVC Main 10, CRF 17, AAC 256k) and film-share.mp4 (H.264,
// 1440×900, CRF 23). No hint or CC button; the end is the live one: a hard cut, 0.75 s of black and silence, the card, its confetti and its cheer.
// (--master=T starts at T s, to check the end quickly)
const masterArg = process.argv.find((a) => a.startsWith("--master"))
if (masterArg) {
  const { page, errors, ctx } = await open({ w: 1440, h: 900, dpr: 2, muted: true })
  await page.addStyleTag({ content: "[data-ui=hint],[data-ui=cc],[data-ui=again],[data-ui=controls]{display:none!important}html[data-black] [data-ui=card]{visibility:hidden!important}" })
  await press(page)
  const cut = Math.round(END_ * FPS), card = Math.round((END_ + 0.75) * FPS), total = Math.round((END_ + 0.75 + 4) * FPS)
  const raw = join(out, "master-v.mp4")
  const color = ["-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv"]
  const ff = Bun.spawn(["ffmpeg", "-y", "-loglevel", "error", "-f", "image2pipe", "-c:v", "mjpeg", "-framerate", String(FPS), "-i", "-",
    "-vf", "scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int,format=yuv420p10le",
    "-c:v", "libx265", "-pix_fmt", "yuv420p10le", "-preset", "medium", "-crf", "17", "-tag:v", "hvc1",
    "-x265-params", "log-level=error:aq-mode=3:no-sao=1:colorprim=bt709:transfer=bt709:colormatrix=bt709:range=limited", ...color, raw], { stdin: "pipe", stderr: "inherit" })
  const t0 = Date.now()
  const from = Math.round(Number(masterArg.split("=")[1] ?? 0) * FPS)
  if (from) await page.evaluate((t) => (window as any).__show.seek(t), from / FPS)
  for (let f = from; f < total; f++) {
    if (f === Math.round((HOLD[0] + 1.6) * FPS)) await page.keyboard.press("Alt+Digit2")
    if (f === Math.round((HOLD[0] + 3.1) * FPS)) await page.keyboard.press("Alt+Digit3")
    if (f === Math.round((HOLD[0] + 4.6) * FPS)) await page.keyboard.press("Alt+Digit1")
    if (f === cut) await page.evaluate(() => (document.documentElement.dataset.black = "1"))
    if (f === card) await page.evaluate(() => delete document.documentElement.dataset.black)
    if (f >= card) await page.evaluate((s) => (window as any).__confetti?.(s), (f - card) / FPS)
    ff.stdin.write(await page.screenshot({ type: "jpeg", quality: 95 }))
    await ff.stdin.flush()
    await step(page, 1 / FPS)
    if (f % 300 === 0) console.log("frame", f, "of", total, Math.round((Date.now() - t0) / 1000) + "s")
  }
  ff.stdin.end()
  if ((await ff.exited) !== 0) throw new Error("master encode failed")
  const log = await S(page, "__show.audioLog()")
  writeFileSync(join(out, "audio-log.json"), JSON.stringify(log))
  report.master = { errors, frames: total - from, seconds: (Date.now() - t0) / 1000 }
  await ctx.close()
  // started before the first frame: joined part-way through
  const shifted = (log as any[]).map((e) => (e.t >= from / FPS ? { ...e, t: e.t - from / FPS } : { ...e, t: 0, offset: (e.offset || 0) + (from / FPS - e.t) }))
  const mix = await mixAudio(out, shifted, (total - from) / FPS, audioDir, { cut: END_ - from / FPS, kbps: 256 })
  if (!mix) throw new Error("no mix")
  const master = join(out, "film-master.mp4"), share = join(out, "film-share.mp4")
  await $`ffmpeg -y -loglevel error -i ${raw} -i ${mix} -map 0:v -map 1:a -c copy -movflags +faststart ${master}`
  // for phones and social: H.264 High 4.1, yuv420p, stereo AAC, faststart, well under 40 MB
  await $`ffmpeg -y -loglevel error -i ${master} -vf scale=1440:900:flags=lanczos -c:v libx264 -preset slow -crf 23 -profile:v high -level:v 4.1 -pix_fmt yuv420p ${color} -c:a aac -b:a 160k -ac 2 -movflags +faststart ${share}`
  rmSync(raw)
  for (const f of [master, share]) (report.master as any)[f.split("/").pop()!] = (Bun.file(f).size / 1e6).toFixed(1) + " MB"
  writeFileSync(join(out, "report.json"), JSON.stringify(report, null, 1))
  console.log(JSON.stringify(report.master, null, 1))
  await browser.close()
  server.stop()
  process.exit(0)
}

// ── 1 · The film, WebGPU, captions on (--quick skips it: keyframes only) ─
if (!quick) {
  const { page, errors, requests, ctx } = await open({ muted: true })
  await page.waitForTimeout(300)
  await shot(page, join(out, "still.jpg"))
  report.beforePress = [...new Set(requests)]
  await press(page)
  report.renderer = (await state(page)).renderer
  const frames = join(out, "frames")
  mkdirSync(frames, { recursive: true })
  const total = Math.round((quick ? 34 : END) * FPS)
  const states: any[] = []
  for (let f = 0; f < total; f++) {
    const t = f / FPS
    // the visitor's turn: Option+2, Option+3, Option+1
    if (f === Math.round((HOLD[0] + 1.6) * FPS)) await page.keyboard.press("Alt+Digit2")
    if (f === Math.round((HOLD[0] + 3.1) * FPS)) await page.keyboard.press("Alt+Digit3")
    if (f === Math.round((HOLD[0] + 4.6) * FPS)) await page.keyboard.press("Alt+Digit1")
    if (!traceOnly) await shot(page, join(frames, String(f).padStart(5, "0") + ".jpg"))
    await step(page, 1 / FPS)
    if (f % 15 === 0) states.push({ f, t, ...(await state(page)) })
    if (f % 300 === 0) console.log("frame", f)
  }
  report.afterPress = [...new Set(requests)].filter((r) => !(report.beforePress as string[]).includes(r))
  report.states = states
  report.errors = errors
  const trace = await S(page, "__show.trace()")
  writeFileSync(join(out, "trace.json"), JSON.stringify(trace))
  const log = await S(page, "__show.audioLog()")
  writeFileSync(join(out, "audio-log.json"), JSON.stringify(log))
  await ctx.close()

  if (!traceOnly) await encode(out, total, FPS, audioDir)
  else rmSync(frames, { recursive: true, force: true })
  await $`bun ${join(import.meta.dir, "trace-check.ts")} ${join(out, "trace.json")} --json ${join(out, "trace-check.json")}`.nothrow()
}

// ── 2 · Keyframes on the other paths ────────────────────────────────────
async function keyframes(name: string, o: Opts) {
  const { page, errors, ctx } = await open({ muted: true, ...o })
  await press(page, o.mobile)
  const dir = join(out, "k-" + name)
  mkdirSync(dir, { recursive: true })
  const files: string[] = []
  for (const t of KEYS) {
    await page.evaluate((t) => (window as any).__show.seek(t), t)
    const p = join(dir, String(t).replace(".", "_") + ".jpg")
    await shot(page, p)
    files.push(p)
  }
  await grid(files, o.mobile ? 10 : 5, o.mobile ? "195x422" : "448x280", join(out, "strip-" + name + ".jpg"))
  rmSync(dir, { recursive: true, force: true })
  ;(report.paths ??= {} as any)[name] = { errors, renderer: (await state(page).catch(() => ({}))).renderer }
  await ctx.close()
}
if (quick) {
  const { page, requests } = await open({ muted: true })
  await page.waitForTimeout(300)
  await shot(page, join(out, "still.jpg"))
  report.beforePress = [...new Set(requests)]
  await page.context().close()
  await keyframes("gpu", {})
}
if (!traceOnly) {
  await keyframes("canvas2d", { query: "canvas2d" })
  await keyframes("reduced", { reduced: true })
  await keyframes("phone", { w: 390, h: 844, dpr: 3, mobile: true })
}

// ── 2b · Full-size 2x stills, to check edges and text at 100% ───────────
if (!traceOnly) {
  const { page, ctx } = await open({ w: 1440, h: 900, dpr: 2, muted: true })
  await press(page)
  const dir = join(out, "x2")
  mkdirSync(dir, { recursive: true })
  for (const t of X2) {
    await page.evaluate((t) => (window as any).__show.seek(t), t)
    await page.screenshot({ path: join(dir, String(t).replace(".", "_") + ".png") })
  }
  await ctx.close()
}

// ── 2c · The page below the film, before any press (desktop and phone) ──
for (const [name, o] of traceOnly ? [] : ([["page", { w: 1440, h: 900 }], ["page-phone", { w: 390, h: 844, dpr: 3, mobile: true }]] as const)) {
  const { page, ctx } = await open({ muted: true, ...o })
  await page.waitForTimeout(400)
  await page.screenshot({ path: join(out, name + ".jpg"), type: "jpeg", quality: 80, fullPage: true })
  await ctx.close()
}

// ── 3 · Real time: the busiest section at 1440×900 @2x ──────────────────
if (!quick) {
  const { page, errors, ctx } = await open({ w: 1440, h: 900, dpr: 2 })
  await press(page)
  await page.evaluate((t) => (window as any).__show.seek(t), HOLD[1] + 2)
  report.perfBusy = await page.evaluate(() => (window as any).__show.perf(20000))
  await page.evaluate((t) => (window as any).__show.seek(t), END - 40)
  report.perfEnding = await page.evaluate(() => (window as any).__show.perf(12000))
  report.perfErrors = errors
  await ctx.close()
}

writeFileSync(join(out, "report.json"), JSON.stringify(report, null, 1))
console.log(JSON.stringify({ ...report, states: undefined }, null, 1))
await browser.close()
server.stop()

