// Generate the show's voice bank (eleven_v4, with timestamps) and SFX (eleven_text_to_sound_v2)
// into raw/. Skips files that already exist; delete a raw file to regenerate that take.
// 2password run --env "ELEVENLABS_API_KEY=op://…" -- bun gen.ts [--only id]
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"

const dir = import.meta.dir
const raw = join(dir, "raw")
mkdirSync(raw, { recursive: true })
const script = JSON.parse(readFileSync(join(dir, "script.json"), "utf8"))
const key = process.env.ELEVENLABS_API_KEY
if (!key) throw new Error("ELEVENLABS_API_KEY missing")
const only = process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : null
const API = "https://api.elevenlabs.io"

async function voice(line: any, take: number) {
  const file = join(raw, `${line.id}-${take}.json`)
  if (existsSync(file)) return
  const st = script.stages[line.stage]
  const body = {
    model_id: "eleven_v4",
    text: `${st.tag} ${line.text}`,
    voice_settings: { stability: st.stability, similarity_boost: 0.75 },
    use_pvc_as_ivc: false,
  }
  let res = await fetch(`${API}/v1/text-to-speech/${script.voice}/with-timestamps?output_format=mp3_44100_128`, {
    method: "POST",
    headers: { "xi-api-key": key!, "content-type": "application/json" },
    body: JSON.stringify(body),
  })
  if (!res.ok && st.stability !== 0.5) {
    body.voice_settings.stability = 0.5
    res = await fetch(`${API}/v1/text-to-speech/${script.voice}/with-timestamps?output_format=mp3_44100_128`, {
      method: "POST",
      headers: { "xi-api-key": key!, "content-type": "application/json" },
      body: JSON.stringify(body),
    })
  }
  if (!res.ok) throw new Error(`${line.id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
  const j: any = await res.json()
  writeFileSync(join(raw, `${line.id}-${take}.mp3`), Buffer.from(j.audio_base64, "base64"))
  writeFileSync(file, JSON.stringify({ text: body.text, words: line.text, settings: body.voice_settings, alignment: j.alignment, normalized: j.normalized_alignment, requestId: res.headers.get("request-id") }))
  console.log("voice", line.id, take)
}

async function sfx(s: any) {
  const file = join(raw, `sfx-${s.id}.mp3`)
  if (existsSync(file)) return
  const res = await fetch(`${API}/v1/sound-generation?output_format=mp3_44100_128`, {
    method: "POST",
    headers: { "xi-api-key": key!, "content-type": "application/json" },
    body: JSON.stringify({ text: s.text, model_id: "eleven_text_to_sound_v2", duration_seconds: s.dur, prompt_influence: 0.5, loop: !!s.loop }),
  })
  if (!res.ok) throw new Error(`sfx ${s.id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
  writeFileSync(file, Buffer.from(await res.arrayBuffer()))
  console.log("sfx", s.id)
}

const jobs: (() => Promise<void>)[] = []
for (const line of script.lines) {
  if (only && line.id !== only) continue
  for (let t = 0; t < script.stages[line.stage].takes; t++) jobs.push(() => voice(line, t))
}
for (const s of script.sfx) if (!only || only === s.id) jobs.push(() => sfx(s))
let failed = 0
const pool = async () => {
  while (jobs.length) {
    const j = jobs.shift()!
    try {
      await j()
    } catch (e) {
      failed++
      console.error(String(e))
    }
  }
}
await Promise.all([pool(), pool(), pool()])
console.log(failed ? `${failed} failed` : "all done")
