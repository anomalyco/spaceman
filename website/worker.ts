// Serves Spaceman's static site (dist/) under /spaceman on anoma.ly. The route also matches
// paths like /spacemanx, which aren't ours. /spaceman redirects to /spaceman/ so the page's
// relative asset URLs resolve under it.
import { parseTargetString, renderBounceHtml, resolveBounceRoute } from "./src/deepLinks.ts"

const BASE = "/spaceman"
const RELEASE_SERVICE = "https://anoma.ly/release"

type Env = { ASSETS: { fetch(request: Request): Promise<Response> } }

// The path inside dist/ for a path under BASE, or undefined when it isn't ours.
function stripBase(pathname: string): string | undefined {
  if (pathname === BASE || pathname === `${BASE}/`) return "/"
  if (pathname.startsWith(`${BASE}/`)) return pathname.slice(BASE.length)
  return undefined
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url)
    if (url.pathname === BASE) {
      url.pathname = `${BASE}/`
      return Response.redirect(url.toString(), 301)
    }
    const path = stripBase(url.pathname)
    if (path === undefined) return new Response("Not found", { status: 404 })
    if (path.startsWith("/download/")) return download(request, path.slice("/download/".length))
    const bounce = resolveBounceRoute(path, url.searchParams)
    if (bounce) {
      if (request.method !== "GET" && request.method !== "HEAD") {
        return new Response("Method not allowed", { status: 405, headers: { Allow: "GET, HEAD" } })
      }
      const isOk = bounce.kind === "ok"
      const target = isOk ? bounce.target : parseTargetString("settings")!
      const html = renderBounceHtml(target, { allowQueryOverride: false, invalidNote: !isOk })
      const headers = new Headers({
        "Content-Type": "text/html; charset=utf-8",
        "Cache-Control": isOk ? "public, max-age=0, must-revalidate" : "no-store",
      })
      secure(headers)
      return new Response(request.method === "HEAD" ? null : html, { status: isOk ? 200 : 400, headers })
    }
    const assetUrl = new URL(url)
    assetUrl.pathname = path
    const response = await env.ASSETS.fetch(new Request(assetUrl, request))
    const headers = new Headers(response.headers)
    // asset redirects (trailing slashes, /index.html) point at paths without the prefix
    const location = headers.get("Location")
    if (location?.startsWith("/")) headers.set("Location", BASE + location)
    // Vite's hashed bundles never change
    if (path.startsWith("/assets/")) headers.set("Cache-Control", "public, max-age=31536000, immutable")
    // the page and its unhashed files are checked on every visit, so a deploy shows at once
    else if (path === "/" || path.endsWith(".html") || /\.(txt|md|xml|webmanifest)$/.test(path)) headers.set("Cache-Control", "public, max-age=0, must-revalidate")
    // the film's media, icons and the og:image keep their names across deploys: a day, then check
    else headers.set("Cache-Control", "public, max-age=86400, stale-while-revalidate=604800")
    headers.set("Strict-Transport-Security", "max-age=31536000")
    headers.set("X-Content-Type-Options", "nosniff")
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")
    if (path.endsWith(".md")) headers.set("Content-Type", "text/markdown; charset=utf-8")
    if (/\.(mp4|webm|mp3)$/.test(path)) return ranged(request, response, headers)
    return new Response(response.body, { status: response.status, statusText: response.statusText, headers })
  },
}

// Static assets ignore Range, but Safari and AVFoundation need byte ranges to play video (as in
// Hex's site worker).
async function ranged(request: Request, response: Response, headers: Headers): Promise<Response> {
  headers.set("Accept-Ranges", "bytes")
  const match = /^bytes=(\d*)-(\d*)$/.exec(request.headers.get("Range") ?? "")
  if (response.status !== 200 || !match || (!match[1] && !match[2])) {
    return new Response(request.method === "HEAD" ? null : response.body, { status: response.status, statusText: response.statusText, headers })
  }
  const body = await response.arrayBuffer()
  const size = body.byteLength
  const start = match[1] ? Number(match[1]) : Math.max(0, size - Number(match[2]))
  const end = match[1] && match[2] ? Math.min(Number(match[2]), size - 1) : size - 1
  if (start > end) {
    headers.set("Content-Range", `bytes */${size}`)
    return new Response(null, { status: 416, headers })
  }
  headers.set("Content-Range", `bytes ${start}-${end}/${size}`)
  headers.set("Content-Length", String(end - start + 1))
  return new Response(request.method === "HEAD" ? null : body.slice(start, end + 1), { status: 206, headers })
}

type ReleaseManifest = {
  release?: { targets?: Record<string, { sha256?: string; url?: string }> }
}

function secure(headers: Headers) {
  headers.set("Strict-Transport-Security", "max-age=31536000")
  headers.set("X-Content-Type-Options", "nosniff")
  headers.set("Referrer-Policy", "strict-origin-when-cross-origin")
}

// Redirects /spaceman/download/macos-arm64 to the Anomaly release service's stable alias (or dev
// until a stable release is published), and serves its live SHA-256 at .../macos-arm64.sha256.
async function download(request: Request, rest: string): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") {
    return new Response("Method not allowed", { status: 405, headers: { Allow: "GET, HEAD" } })
  }
  const dev = rest.startsWith("dev/")
  const target = dev ? rest.slice("dev/".length) : rest
  const isSha = target === "macos-arm64.sha256"
  if (target !== "macos-arm64" && !isSha) return new Response("Not found", { status: 404 })

  let channel: "stable" | "dev" = "dev"
  let manifest: ReleaseManifest | undefined
  if (!dev) {
    try {
      const res = await fetch(`${RELEASE_SERVICE}/spaceman/desktop/stable.json`, {
        headers: { "User-Agent": "spaceman-site" },
        cache: "no-store",
      })
      if (res.ok) {
        channel = "stable"
        if (isSha) manifest = (await res.json()) as ReleaseManifest
      }
    } catch {
      // Fall through to dev or error visibly below.
    }
  }

  if (isSha) {
    if (!manifest) {
      const res = await fetch(`${RELEASE_SERVICE}/spaceman/desktop/${channel}.json`, {
        headers: { "User-Agent": "spaceman-site" },
        cache: "no-store",
      })
      if (!res.ok) return new Response("Release checksum unavailable", { status: 502 })
      manifest = (await res.json()) as ReleaseManifest
    }
    const sha = manifest.release?.targets?.["macos-arm64"]?.sha256
    if (!sha) return new Response("Checksum not found", { status: 404 })
    const headers = new Headers({
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "no-store",
    })
    secure(headers)
    return new Response(request.method === "HEAD" ? null : `${sha}  Spaceman.zip\n`, {
      status: 200,
      headers,
    })
  }

  const headers = new Headers({
    Location: `${RELEASE_SERVICE}/spaceman/desktop/${channel}/macos-arm64`,
    "Cache-Control": "no-store",
  })
  secure(headers)
  return new Response(null, { status: 302, headers })
}
