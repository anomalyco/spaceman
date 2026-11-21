// The interviewer's two lines (a stock ElevenLabs voice, an invented detective) into raw/inter/.
// Skips lines that already exist. usage: bun inter.ts
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"

const dir = import.meta.dir
const out = join(dir, "raw", "inter")
mkdirSync(out, { recursive: true })
const spec = JSON.parse(readFileSync(join(dir, "mono.json"), "utf8")).interviewer
const key = process.env.ELEVENLABS_API_KEY
if (!key) throw new Error("ELEVENLABS_API_KEY missing")

for (const [id, text] of Object.entries(spec.lines) as [string, string][])
  for (const n of [1, 2]) {
    const file = join(out, `${id}-${n}.mp3`)
    if (existsSync(file)) continue
    const res = await fetch(`https://api.elevenlabs.io/v1/text-to-speech/${spec.voice}?output_format=mp3_44100_192`, {
      method: "POST",
      headers: { "xi-api-key": key, "content-type": "application/json" },
      body: JSON.stringify({ model_id: "eleven_v4", text, voice_settings: { stability: 0.5, similarity_boost: 0.75 } }),
    })
    if (!res.ok) throw new Error(`${id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
    writeFileSync(file, Buffer.from(await res.arrayBuffer()))
    console.log("line", id, n)
  }
