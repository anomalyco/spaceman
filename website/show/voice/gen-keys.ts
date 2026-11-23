// Takes for keys.py: laptop key presses (ka–kd) and trackpad clicks (tp, tq), three of each, from
// eleven_text_to_sound_v2 into raw/cand/. keys.py picks the clean single transients out of these.
// 2password run --env "ELEVENLABS_API_KEY=op://…" -- bun gen-keys.ts
import { mkdirSync, writeFileSync } from "node:fs"
import { join } from "node:path"
const key = process.env.ELEVENLABS_API_KEY!
const out = join(import.meta.dir, "raw", "cand")
mkdirSync(out, { recursive: true })
const P = {
  ka: "a single soft keystroke on a modern thin laptop keyboard, low-travel scissor switch key, close microphone, completely dry, anechoic, no reverb, no other sounds",
  kb: "one quick light tap on a laptop keyboard letter key, small crisp plastic tick, close mic, anechoic, dry, nothing else",
  kc: "a single key press and release on an aluminium laptop keyboard, soft muted click, recorded very close, dry studio, no room sound",
  kd: "one light fingertip keystroke on a flat chiclet keyboard, short soft clack, close mic, dry, no echo",
  tp: "one single soft click of a laptop glass trackpad, short haptic tick, very close microphone, completely dry, no reverb, nothing else",
  tq: "a single quiet click on a laptop touchpad, tiny crisp tick, close mic, dry, anechoic",
}
const jobs = []
for (const [id, text] of Object.entries(P)) for (let i = 0; i < 3; i++) jobs.push({ id: id + i, text })
let q = [...jobs]
async function run() {
  while (q.length) {
    const j = q.shift()!
    const res = await fetch("https://api.elevenlabs.io/v1/sound-generation?output_format=mp3_44100_192", { method: "POST", headers: { "xi-api-key": key, "content-type": "application/json" }, body: JSON.stringify({ text: j.text, model_id: "eleven_text_to_sound_v2", duration_seconds: 0.5, prompt_influence: 0.75 }) })
    if (!res.ok) { console.log("fail", j.id, res.status, (await res.text()).slice(0, 200)); continue }
    writeFileSync(out + "/" + j.id + ".mp3", Buffer.from(await res.arrayBuffer()))
    console.log("ok", j.id)
  }
}
await Promise.all([run(), run(), run(), run(), run()])
