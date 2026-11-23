// Transcribe processed clips with ElevenLabs Scribe to check each line.
// usage: bun stt.ts <file.mp3>...
const key = process.env.ELEVENLABS_API_KEY!
for (const f of process.argv.slice(2)) {
  const fd = new FormData()
  fd.append("model_id", "scribe_v1")
  fd.append("file", new Blob([await Bun.file(f).arrayBuffer()]), f)
  const r = await fetch("https://api.elevenlabs.io/v1/speech-to-text", { method: "POST", headers: { "xi-api-key": key }, body: fd })
  const j: any = await r.json()
  console.log(f, "→", r.ok ? j.text : JSON.stringify(j).slice(0, 200))
}
