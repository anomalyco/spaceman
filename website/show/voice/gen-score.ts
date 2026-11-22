// The film's string score (ElevenLabs Music) and the tape recorder's start (ElevenLabs sound effects),
// into raw/score/. Skips files that already exist. At most two requests at a time (the music API's limit).
// 2password run --env "ELEVENLABS_API_KEY=op://…" -- bun gen-score.ts
import { existsSync, mkdirSync } from "node:fs"
import { join } from "node:path"

const out = join(import.meta.dir, "raw", "score")
mkdirSync(out, { recursive: true })
const key = process.env.ELEVENLABS_API_KEY
if (!key) throw new Error("ELEVENLABS_API_KEY missing")
const API = "https://api.elevenlabs.io"

const CHAMBER = "Solo chamber strings recorded close and dry in a small room, realistic bowing and rosin noise, no percussion, no piano, no synth, no choir, no melody hook, film underscore."
export const STEMS: Record<string, { prompt: string; secs: number }> = {
  // the walker, fourteen cameras, the neighbours: a cello alone, from nothing
  cello: { secs: 26, prompt: `A single solo cello holding one very low sustained note, a somber drone that creeps in from complete silence, extremely quiet and restrained, slow long bows, the note barely shifting by a semitone now and then. Dark, patient, unresolved. ${CHAMBER}` },
  // the hedge, the bins, the dark houses: cellos swell, high violins creep in
  dread: { secs: 38, prompt: `Dark chamber strings building dread: two low cellos swelling slowly on a minor drone, eerie high violins entering with quiet bowed tremolo and sul ponticello, tense dissonant minor-second clusters, growing gradually louder and more anxious throughout, never resolving. ${CHAMBER}` },
  // seven, the questions, the cellar: a taut swell to its peak
  peak: { secs: 28, prompt: `A taut, chilling string swell: cellos and violins together in a sustained dissonant cluster, slowly crescendoing to an intense, shivering peak and holding it, high violin harmonics screeching faintly on top, no resolution, horror thriller underscore. ${CHAMBER}` },
}
const FOLEY: Record<string, { prompt: string; secs: number }> = {
  whir: { secs: 3, prompt: "Close recording of a cassette tape recorder already running: the steady soft whir of the capstan motor and the reels turning, with faint even tape hiss. Continuous and constant, no clicks, no button presses, no voices, no music, quiet dry room." },
  tape: { secs: 2.2, prompt: "A police interview cassette tape recorder: one firm mechanical play-button press clunk, then the tape reels start turning with a soft whir and faint tape hiss settling in. Close, realistic, dry room, no beep, no music." },
}

async function music(id: string, take: number, s: { prompt: string; secs: number }) {
  const file = join(out, `${id}-${take}.mp3`)
  if (existsSync(file)) return
  const res = await fetch(`${API}/v1/music?output_format=mp3_44100_192`, {
    method: "POST",
    headers: { "xi-api-key": key!, "content-type": "application/json" },
    body: JSON.stringify({ prompt: s.prompt, music_length_ms: s.secs * 1000, model_id: "music_v1", force_instrumental: true }),
  })
  if (!res.ok) throw new Error(`${id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
  await Bun.write(file, await res.arrayBuffer())
  console.log("music", id, take)
}

async function foley(id: string, take: number, s: { prompt: string; secs: number }) {
  const file = join(out, `${id}-${take}.mp3`)
  if (existsSync(file)) return
  const res = await fetch(`${API}/v1/sound-generation?output_format=mp3_44100_128`, {
    method: "POST",
    headers: { "xi-api-key": key!, "content-type": "application/json" },
    body: JSON.stringify({ text: s.prompt, model_id: "eleven_text_to_sound_v2", duration_seconds: s.secs, prompt_influence: 0.6 }),
  })
  if (!res.ok) throw new Error(`foley ${id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
  await Bun.write(file, await res.arrayBuffer())
  console.log("foley", id, take)
}

const jobs: (() => Promise<void>)[] = []
for (const [id, s] of Object.entries(STEMS)) for (const t of [0, 1]) jobs.push(() => music(id, t, s))
for (const [id, s] of Object.entries(FOLEY)) for (const t of [0, 1, 2, 3]) jobs.push(() => foley(id, t, s))
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
await Promise.all([pool(), pool()])
console.log(failed ? `${failed} failed` : "all done")
