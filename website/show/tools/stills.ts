// Stills of the site for the report, and the og:image, from a production build served through worker.ts.
// usage (from website/show/tools, after bun run build): bun stills.ts <outDir>
// Writes og-image.png (og/og.html at 1200x630), site-desktop.jpg (1440x900, full page) and
// site-mobile.jpg (390x844 at 3x, full page). Never plays audio.
import { mkdirSync } from "node:fs"
import { join, resolve } from "node:path"
import { chromium } from "playwright"
import worker from "../../worker.ts"

const out = resolve(process.argv[2] ?? "stills")
mkdirSync(out, { recursive: true })
const root = resolve(import.meta.dir, "../..")
const dist = join(root, "dist")
const ASSETS = {
  async fetch(request: Request) {
    const path = decodeURIComponent(new URL(request.url).pathname)
    const file = Bun.file(join(dist, path.endsWith("/") ? path + "index.html" : path))
    return (await file.exists()) ? new Response(file) : new Response("Not found", { status: 404 })
  },
}
const server = Bun.serve({ port: 0, fetch: (request) => worker.fetch(request, { ASSETS }) })
const site = `http://127.0.0.1:${server.port}/spaceman/`

// a browser per shot: headless Chromium sometimes dies closing a page on some workers
const launch = () => chromium.launch({ headless: true, channel: "chromium", args: ["--enable-unsafe-webgpu", "--enable-features=Vulkan", "--use-angle=metal", "--mute-audio"] })

async function og() {
  const browser = await launch()
  const ctx = await browser.newContext({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 })
  const page = await ctx.newPage()
  await page.goto("file://" + join(root, "og/og.html"))
  await page.evaluate(() => document.fonts.ready)
  await page.waitForFunction(() => [...document.images].every((i) => i.complete && i.naturalWidth > 0))
  await page.screenshot({ path: join(out, "og-image.png"), type: "png" })
  await browser.close().catch(() => {})
}

async function full(name: string, opts: { width: number; height: number; dpr: number; mobile: boolean }) {
  const browser = await launch()
  const ctx = await browser.newContext({
    viewport: { width: opts.width, height: opts.height },
    deviceScaleFactor: opts.dpr,
    isMobile: opts.mobile,
    hasTouch: opts.mobile,
    // the desktop keeps Chromium's own user agent: Playwright derives client hints from an overridden one,
    // and "Intel Mac OS X" would read as an Intel Mac
    userAgent: opts.mobile
      ? "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1"
      : undefined,
    reducedMotion: "reduce",
  })
  const page = await ctx.newPage()
  const errors: string[] = []
  page.on("pageerror", (e) => errors.push(String(e)))
  await page.goto(site, { waitUntil: "networkidle" })
  await page.evaluate(() => document.fonts.ready)
  // walk down once so anything that waits to be seen has been seen, then back to the top
  const h = await page.evaluate(() => document.documentElement.scrollHeight)
  for (let y = 0; y < h; y += opts.height / 2) {
    await page.evaluate((y) => window.scrollTo(0, y), y)
    await page.waitForTimeout(120)
  }
  await page.evaluate(() => window.scrollTo(0, 0))
  await page.waitForTimeout(400)
  await page.screenshot({ path: join(out, `${name}-fold.jpg`), type: "jpeg", quality: 88 })
  await page.screenshot({ path: join(out, `${name}.jpg`), type: "jpeg", quality: 85, fullPage: true })
  if (errors.length) console.log(name, "page errors:", errors.join(" | "))
  await browser.close().catch(() => {})
}

await og()
await full("site-desktop", { width: 1440, height: 900, dpr: 1, mobile: false })
await full("site-mobile", { width: 390, height: 844, dpr: 3, mobile: true })
server.stop()
console.log("stills in", out)
