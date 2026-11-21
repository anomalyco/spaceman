// Deterministic capture of the show from a static build in website/dist. Never plays audio.
// Steps the clock frame by frame, screenshots each frame, encodes a 30 fps clip with an offline
// mix of the audio play log, and writes keyframe strips for the WebGPU, Canvas 2D, reduced-motion
// and phone paths, plus real-time frame stats for the busiest section. Raw frames are deleted.
// usage: bun capture.ts <dist-dir> <out-dir> [--quick]
import { chromium, type Page } from "playwright"
import { mkdirSync, rmSync, writeFileSync, existsSync } from "node:fs"
import { join, resolve } from "node:path"
import { $ } from "bun"
import { encode, grid } from "./encode.ts"
import { END as END_, HOLD, word } from "../../src/show/director.js"
import { LINES } from "../../src/show/tape.js"

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
const END = END_ + 0.6
// keyframes: the middle of every other sentence, and every sentence from the signatures on
const sig = word("Three of them signed", 60)
const KEYS = LINES.filter((l: any, i: number) => i % 2 === 0 || l[1] >= sig - 0.1).map((l: any) => +((l[1] + l[2]) / 2).toFixed(2))
const X2 = [1.5, 4.5, ...["tiled themselves", "There's a bar", "Chatter opens", "Four years", "six foot two", "photograph attached", "Three of them signed", "Mark was in June", "They're both on six", "last thing", "I went over", "His light", "plastic down", "He's on six"].map((p) => +(word(p, p === "Three of them signed" ? 60 : 0) + 0.9).toFixed(2))]
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
  for (const [name, q] of [["gpu", ""], ["c2d", "canvas2d"]] as const) {
    const { page, errors, ctx } = await open({ w: 1440, h: 900, muted: true, query: q })
    if (name === "gpu") await shot(page, join(out, "still.jpg"))
    await press(page)
    for (const t of times) {
      await page.evaluate((t) => (window as any).__show.seek(t), t)
      await shot(page, join(out, name + "-" + String(t).replace(".", "_") + ".jpg"))
    }
    report[name] = { errors, state: await state(page) }
    await ctx.close()
  }
  writeFileSync(join(out, "report.json"), JSON.stringify(report, null, 1))
  console.log(JSON.stringify(report, null, 1))
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
    await shot(page, join(frames, String(f).padStart(5, "0") + ".jpg"))
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

  await encode(out, total, FPS, audioDir)
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
await keyframes("canvas2d", { query: "canvas2d" })
await keyframes("reduced", { reduced: true })
await keyframes("phone", { w: 390, h: 844, dpr: 3, mobile: true })

// ── 2b · Full-size 2x stills, to check edges and text at 100% ───────────
{
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

