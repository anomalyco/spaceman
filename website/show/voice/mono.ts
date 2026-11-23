// Generate long takes of the monologue (eleven_v4 TTS with timestamps) into raw/mono/.
// Skips takes that already exist. usage: bun mono.ts [--only m1]
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"

const dir = import.meta.dir
const out = join(dir, "raw", "mono")
mkdirSync(out, { recursive: true })
const spec = JSON.parse(readFileSync(join(dir, "mono.json"), "utf8"))
const key = process.env.ELEVENLABS_API_KEY
if (!key) throw new Error("ELEVENLABS_API_KEY missing")
const only = process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : null

async function take(t: { id: string; stability: number; text?: string; previous_text?: string }) {
  const file = join(out, `${t.id}.json`)
  if (existsSync(file)) return
  const body: any = {
    model_id: "eleven_v4",
    text: t.text?.startsWith("$") ? spec[t.text.slice(1)] : (t.text ?? spec.text),
    voice_settings: { stability: t.stability, similarity_boost: 0.75 },
    use_pvc_as_ivc: false,
  }
  if (t.previous_text) body.previous_text = t.previous_text
  const res = await fetch(`https://api.elevenlabs.io/v1/text-to-speech/${spec.voice}/with-timestamps?output_format=mp3_44100_192`, {
    method: "POST",
    headers: { "xi-api-key": key!, "content-type": "application/json" },
    body: JSON.stringify(body),
  })
  if (!res.ok) throw new Error(`${t.id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
  const j: any = await res.json()
  writeFileSync(join(out, `${t.id}.mp3`), Buffer.from(j.audio_base64, "base64"))
  writeFileSync(file, JSON.stringify({ text: body.text, settings: body.voice_settings, alignment: j.alignment, requestId: res.headers.get("request-id") }))
  console.log("take", t.id)
}

const jobs = spec.takes.filter((t: any) => !only || t.id === only)
await Promise.all(jobs.map((t: any) => take(t).catch((e) => console.error(String(e)))))
