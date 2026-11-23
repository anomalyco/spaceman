// The other man's one sound: a faint, closed-mouth "hm" across the table (a stock v4 voice, not
// Kit's). Writes raw/mono/hm-<n>.mp3. usage: bun hm.ts
import { mkdirSync, writeFileSync, existsSync } from "node:fs"
import { join } from "node:path"

const key = process.env.ELEVENLABS_API_KEY
const out = join(import.meta.dir, "raw", "mono")
mkdirSync(out, { recursive: true })
const vs: any = await (await fetch("https://api.elevenlabs.io/v1/voices", { headers: { "xi-api-key": key! } })).json()
const male = vs.voices.filter((v: any) => v.category === "premade" && /male/i.test(v.labels?.gender ?? "") && !/female/i.test(v.labels?.gender ?? ""))
const voice = (male.find((v: any) => /middle|old/i.test(v.labels?.age ?? "")) ?? male[0]).voice_id
console.log("voice", voice, male.map((v: any) => v.name + ":" + v.labels?.age).join(", "))
const texts = ["[quietly, to himself, mouth closed] Hm.", "[barely audible, a short thoughtful hum] Hm.", "[a quiet, noncommittal] Hm."]
await Promise.all(texts.map(async (text, i) => {
  const f = join(out, `hm-${i + 1}.mp3`)
  if (existsSync(f)) return
  const res = await fetch(`https://api.elevenlabs.io/v1/text-to-speech/${voice}?output_format=mp3_44100_192`, {
    method: "POST",
    headers: { "xi-api-key": key!, "content-type": "application/json" },
    body: JSON.stringify({ model_id: "eleven_v4", text, voice_settings: { stability: 0.5, similarity_boost: 0.75 } }),
  })
  if (!res.ok) return console.error(i, res.status, (await res.text()).slice(0, 200))
  writeFileSync(f, Buffer.from(await res.arrayBuffer()))
  console.log("hm", i + 1)
}))
