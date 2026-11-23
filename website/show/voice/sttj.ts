// Transcribe takes with ElevenLabs Scribe (word timings and audio events) to <file>.stt.json.
// usage: bun sttj.ts <file.mp3>...
const key = process.env.ELEVENLABS_API_KEY!
await Promise.all(process.argv.slice(2).map(async (f) => {
  const fd = new FormData()
  fd.append("model_id", "scribe_v1")
  fd.append("tag_audio_events", "true")
  fd.append("file", new Blob([await Bun.file(f).arrayBuffer()]), f)
  const r = await fetch("https://api.elevenlabs.io/v1/speech-to-text", { method: "POST", headers: { "xi-api-key": key }, body: fd })
  const j: any = await r.json()
  if (!r.ok) return console.error(f, JSON.stringify(j).slice(0, 200))
  await Bun.write(f.replace(/\.mp3$/, ".stt.json"), JSON.stringify(j))
  console.log(f, "→", j.text)
}))
