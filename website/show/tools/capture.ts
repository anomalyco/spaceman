// Deterministic capture of the show from a static build in website/dist. Never plays audio.
// Steps the clock frame by frame, screenshots each frame, encodes a 30 fps clip with an offline
// mix of the audio play log, and writes keyframe strips for the WebGPU, Canvas 2D, reduced-motion
// and phone paths, plus real-time frame stats for the busiest section. Raw frames are deleted.
// usage: bun capture.ts <dist-dir> <out-dir> [--quick]
import { chromium, type Page } from "playwright"
import { mkdirSync, rmSync, writeFileSync, existsSync } from "node:fs"
import { join, resolve } from "node:path"
import { $ } from "bun"

const [distArg = "../../dist", outArg = "cap"] = process.argv.slice(2).filter((a) => !a.startsWith("--"))
const quick = process.argv.includes("--quick")
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
const END = 103.4

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
  if (mobile) await page.tap("section button[aria-label]")
  else await page.keyboard.press("Alt+Digit1")
  await page.waitForFunction(() => document.querySelector("section")?.dataset.phase === "live", null, { timeout: 20000 })
}
const shot = (page: Page, path: string) => page.screenshot({ path, type: "jpeg", quality: 86 })

// ── 1 · The film, WebGPU, captions on ───────────────────────────────────
{
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
    if (f === Math.round(25 * FPS)) await page.keyboard.press("Alt+Digit2")
    if (f === Math.round(26.4 * FPS)) await page.keyboard.press("Alt+Digit3")
    if (f === Math.round(27.6 * FPS)) await page.keyboard.press("Alt+Digit1")
    await shot(page, join(frames, String(f).padStart(5, "0") + ".jpg"))
    await step(page, 1 / FPS)
    if (f % 15 === 0) states.push({ f, t, ...(await state(page)) })
    if (f % 300 === 0) console.log("frame", f)
  }
  report.afterPress = [...new Set(requests)].filter((r) => !(report.beforePress as string[]).includes(r))
  report.states = states
  report.errors = errors
  const log = await S(page, "__show.audioLog()")
  writeFileSync(join(out, "audio-log.json"), JSON.stringify(log))
  await ctx.close()

  // contact sheet every 2 s, plus the clip with an offline mix
  const every = 2 * FPS
  const picks: string[] = []
  for (let f = 0; f < total; f += every) picks.push(join(frames, String(f).padStart(5, "0") + ".jpg"))
  for (let i = 0; i < picks.length; i += 20) {
    const part = picks.slice(i, i + 20)
    await $`magick montage ${part} -tile 5x -geometry 512x320+4+4 -background #222 -pointsize 18 -fill white ${join(out, `sheet-${String(i / 20 + 1)}.jpg`)}`.quiet()
  }
  const mix = await mixAudio(log as any[], total / FPS)
  await $`ffmpeg -y -loglevel error -framerate ${FPS} -i ${frames}/%05d.jpg ${mix ? ["-i", mix] : []} -c:v libx264 -pix_fmt yuv420p -crf 22 -preset veryfast ${mix ? ["-c:a", "aac", "-b:a", "160k", "-shortest"] : []} ${join(out, "film.mp4")}`
  rmSync(frames, { recursive: true, force: true })
}

// ── 2 · Keyframes on the other paths ────────────────────────────────────
const KEYS = [1, 5, 10, 15, 22, 33, 42, 49, 54, 60, 66, 72, 77, 84, 90, 95, 97, 99, 100.6, 101.2]
async function keyframes(name: string, o: Opts) {
  const { page, errors, ctx } = await open({ muted: true, ...o })
  await press(page, o.mobile)
  const dir = join(out, "k-" + name)
  mkdirSync(dir, { recursive: true })
  const files: string[] = []
  for (const t of quick ? KEYS.slice(0, 5) : KEYS) {
    await page.evaluate((t) => (window as any).__show.seek(t), t)
    const p = join(dir, String(t).replace(".", "_") + ".jpg")
    await shot(page, p)
    files.push(p)
  }
  const geo = o.mobile ? "195x422+4+4" : "448x280+4+4"
  await $`magick montage ${files} -tile ${o.mobile ? "10x" : "5x"} -geometry ${geo} -background #222 ${join(out, "strip-" + name + ".jpg")}`.quiet()
  rmSync(dir, { recursive: true, force: true })
  ;(report.paths ??= {} as any)[name] = { errors, renderer: (await state(page).catch(() => ({}))).renderer }
  await ctx.close()
}
await keyframes("canvas2d", { query: "canvas2d" })
await keyframes("reduced", { reduced: true })
await keyframes("phone", { w: 390, h: 844, dpr: 3, mobile: true })

// ── 3 · Real time: the busiest section at 1440×900 @2x ──────────────────
{
  const { page, errors, ctx } = await open({ w: 1440, h: 900, dpr: 2 })
  await press(page)
  await page.evaluate(() => (window as any).__show.seek(54))
  report.perfBusy = await page.evaluate(() => (window as any).__show.perf(20000))
  await page.evaluate(() => (window as any).__show.seek(93))
  report.perfEnding = await page.evaluate(() => (window as any).__show.perf(8000))
  report.perfErrors = errors
  await ctx.close()
}

writeFileSync(join(out, "report.json"), JSON.stringify(report, null, 1))
console.log(JSON.stringify({ ...report, states: undefined }, null, 1))
await browser.close()
server.stop()

// Offline mix of the play log with ffmpeg: every clip delayed to its show time; the room loop
// follows its logged levels as a stepped volume envelope.
async function mixAudio(log: any[], secs: number) {
  const clips = log.filter((e) => e.kind === "voice" || e.kind === "sfx")
  if (!clips.length) return null
  const levels = log.filter((e) => e.kind === "level")
  const inputs: string[] = []
  const parts: string[] = []
  let n = 0
  clips.forEach((e, i) => {
    const f = join(audioDir, e.file)
    if (!existsSync(f)) return
    inputs.push("-i", f)
    const k = n++
    const off = e.offset || 0
    parts.push(`[${k}:a]atrim=start=${off},asetpts=PTS-STARTPTS,volume=${e.gain ?? 1},adelay=${Math.round(e.t * 1000)}:all=1[a${i}]`)
  })
  const labels = parts.map((p) => p.slice(p.lastIndexOf("[")))
  const room = join(audioDir, "sfx-room.mp3")
  if (levels.length && existsSync(room)) {
    inputs.push("-stream_loop", "-1", "-i", room)
    const k = n++
    let expr = "0"
    for (const l of levels) expr = `if(gte(t\\,${l.t})\\,${l.gain}\\,${expr})`
    parts.push(`[${k}:a]atrim=0:${secs},volume='${expr}':eval=frame[room]`)
    labels.push("[room]")
  }
  const graph = parts.join(";") + `;${labels.join("")}amix=inputs=${labels.length}:normalize=0,atrim=0:${secs},loudnorm=I=-16:TP=-1.5[out]`
  const file = join(out, "mix.m4a")
  writeFileSync(join(out, "mix-graph.txt"), graph)
  const p = Bun.spawnSync(["ffmpeg", "-y", "-loglevel", "error", ...inputs, "-filter_complex_script", join(out, "mix-graph.txt"), "-map", "[out]", "-c:a", "aac", "-b:a", "160k", file])
  if (p.exitCode !== 0) {
    console.log("mix failed", p.stderr.toString().slice(0, 2000))
    return null
  }
  return file
}
