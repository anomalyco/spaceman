// The production worker (worker.ts) in front of dist/, locally: http://127.0.0.1:<port>/spaceman/
// usage: bun show/tools/serve-site.ts [port]   (after bun run build)
import { join, resolve } from "node:path"
import worker from "../../worker.ts"

const dist = resolve(import.meta.dir, "../../dist")
const ASSETS = {
  async fetch(request: Request) {
    const path = decodeURIComponent(new URL(request.url).pathname)
    const file = Bun.file(join(dist, path.endsWith("/") ? path + "index.html" : path))
    return (await file.exists()) ? new Response(file) : new Response("Not found", { status: 404 })
  },
}
const server = Bun.serve({ port: Number(process.argv[2] ?? 0), fetch: (request) => worker.fetch(request, { ASSETS }) })
console.log(`http://127.0.0.1:${server.port}/spaceman/`)
