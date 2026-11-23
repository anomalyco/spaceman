// The film's string score (ElevenLabs Music) and the recorder's play key (ElevenLabs sound effects),
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
export const STEMS: Record<string, { prompt: string; secs: number; takes?: number }> = {
  // the walker, fourteen cameras, the neighbours: a cello alone, from nothing
  cello: { secs: 26, prompt: `A single solo cello holding one very low sustained note, a somber drone that creeps in from complete silence, extremely quiet and restrained, slow long bows, the note barely shifting by a semitone now and then. Dark, patient, unresolved. ${CHAMBER}` },
  // the hedge, the bins, the dark houses: cellos swell, high violins creep in
  dread: { secs: 38, prompt: `Dark chamber strings building dread: two low cellos swelling slowly on a minor drone, eerie high violins entering with quiet bowed tremolo and sul ponticello, tense dissonant minor-second clusters, growing gradually louder and more anxious throughout, never resolving. ${CHAMBER}` },
  // seven, the questions, the cellar: a taut swell to its peak
  // seven, Joan, the cat, Mark: urgent pulsing cellos, violins climbing
  urgent: { secs: 22, takes: 2, prompt: `Urgent, relentless horror strings: low cellos playing a driving repeated pulsing ostinato that slowly accelerates, violins climbing in uneasy rising chromatic lines and tremolo, dissonant and dense, steadily getting louder and more anxious from start to finish, never resolving. ${CHAMBER}` },
  // the three questions: a held breath
  hush: { secs: 12, takes: 2, prompt: `Near silence: one very quiet, thin, high violin harmonic held and barely trembling, a faint low cello note under it, almost inaudible, breathless suspense, no movement, no swell. ${CHAMBER}` },
  // the switch to seven, the cellar: the climax
  climax: { secs: 16, takes: 3, prompt: "A huge horror film string climax that begins instantly with a massive, violent full string orchestra stab on the very first beat, then roaring low cellos and double basses and shrieking dissonant violin clusters with fast tremolo, relentlessly crescendoing for the whole length to an enormous, terrifying fortissimo peak at the very end, no resolution. Full orchestral string section, cinematic, no percussion, no brass, no choir, no synth." },
  peak: { secs: 28, prompt: `A taut, chilling string swell: cellos and violins together in a sustained dissonant cluster, slowly crescendoing to an intense, shivering peak and holding it, high violin harmonics screeching faintly on top, no resolution, horror thriller underscore. ${CHAMBER}` },
}
// the play key alone: one press, its settle, nothing running after it (a looped motor sounded awful)
const FOLEY: Record<string, { prompt: string; secs: number }> = {
  // the end card: a children's party "yay" (a tiny popper, then the chorus), out of dead silence
  yay: { secs: 2, prompt: "A small group of young children cheering one short, happy, bright \"Yaaay!\" in unison, right after a tiny party popper pop, close and dry, joyful birthday party, then silence. No music, no adults, no applause after." },
  key: { secs: 1, prompt: "A single press of the play button on an old portable cassette recorder: one crisp, dry mechanical ka-clack of the plastic key latching down, recorded close in a quiet room. Then silence. No motor, no whir, no hum, no buzz, no music, no voices, no beep." },
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
    body: JSON.stringify({ text: s.prompt, model_id: "eleven_text_to_sound_v2", duration_seconds: s.secs, prompt_influence: 0.8 }),
  })
  if (!res.ok) throw new Error(`foley ${id}: ${res.status} ${(await res.text()).slice(0, 300)}`)
  await Bun.write(file, await res.arrayBuffer())
  console.log("foley", id, take)
}

const jobs: (() => Promise<void>)[] = []
for (const [id, s] of Object.entries(STEMS)) for (let t = 0; t < (s.takes ?? 2); t++) jobs.push(() => music(id, t, s))
for (const [id, s] of Object.entries(FOLEY)) for (const t of [0, 1, 2, 3, 4, 5]) if (id !== "yay" || t < 4) jobs.push(() => foley(id, t, s))
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
