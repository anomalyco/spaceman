// Turn a capture's frames into contact sheets and a 30 fps clip with an offline mix of the audio
// play log, then delete the frames. usage: bun encode.ts <out-dir> <frames> [fps] <audio-dir>
import { rmSync, writeFileSync, existsSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { $ } from "bun"

export async function encode(out: string, total: number, FPS: number, audioDir: string) {
  const frames = join(out, "frames")
  const every = 2 * FPS
  const picks: string[] = []
  for (let f = 0; f < total; f += every) picks.push(join(frames, String(f).padStart(5, "0") + ".jpg"))
  for (let i = 0; i < picks.length; i += 20) {
    const part = picks.slice(i, i + 20).filter(existsSync)
    if (part.length) await grid(part, 5, "512x320", join(out, `sheet-${String(i / 20 + 1)}.jpg`))
  }
  const log = JSON.parse(readFileSync(join(out, "audio-log.json"), "utf8"))
  const mix = await mixAudio(out, log, total / FPS, audioDir)
  await $`ffmpeg -y -loglevel error -framerate ${FPS} -i ${frames}/%05d.jpg ${mix ? ["-i", mix] : []} -c:v libx264 -pix_fmt yuv420p -crf 22 -preset veryfast ${mix ? ["-c:a", "aac", "-b:a", "160k", "-shortest"] : []} ${join(out, "film.mp4")}`
  rmSync(frames, { recursive: true, force: true })
}

if (import.meta.main) {
  const [out, total, fps = "30", audioDir, mode] = process.argv.slice(2)
  if (mode === "remix") {
    // frames are gone: mix again and mux into the existing clip
    const log = JSON.parse(readFileSync(join(out, "audio-log.json"), "utf8"))
    const mix = await mixAudio(out, log, Number(total) / Number(fps), audioDir)
    if (mix) {
      await $`ffmpeg -y -loglevel error -i ${join(out, "film.mp4")} -i ${mix} -map 0:v -map 1:a -c:v copy -c:a aac -b:a 160k -shortest ${join(out, "film-a.mp4")}`
      await $`mv ${join(out, "film-a.mp4")} ${join(out, "film.mp4")}`
    }
  } else await encode(out, Number(total), Number(fps), audioDir)
}

// Offline mix of the play log with ffmpeg: every clip delayed to its show time; the room loop
// follows its logged levels as a stepped volume envelope. cut: where everything stops dead (the
// film's last cut), with the live player's few milliseconds of ramp.
export async function mixAudio(out: string, log: any[], secs: number, audioDir: string, o: { cut?: number; kbps?: number } = {}) {
  const clips = log.filter((e) => e.kind === "voice" || e.kind === "sfx" || e.kind === "score" || e.kind === "card")
  if (!clips.length) return null
  const levels = log.filter((e) => e.kind === "level")
  const inputs: string[] = []
  const parts: string[] = []
  let n = 0
  clips.forEach((e, i) => {
    const f = join(audioDir, e.file)
    if (!existsSync(f)) return
    inputs.push("-i", f)
    const k = n++
    const off = e.offset || 0
    parts.push(`[${k}:a]atrim=start=${off},asetpts=PTS-STARTPTS,volume=${e.gain ?? 1},adelay=${Math.round(e.t * 1000)}:all=1[a${i}]`)
  })
  // the end card's sound comes after the cut, so it's mixed in past the cut's silence
  const card = clips.flatMap((e, i) => (e.kind === "card" ? [`[a${i}]`] : []))
  const labels = parts.map((p) => p.slice(p.lastIndexOf("["))).filter((l) => !card.includes(l))
  // each bed follows its own logged levels (older logs name only the room, as "room-level")
  const beds = new Map<string, any[]>()
  for (const l of levels) {
    const f = l.file === "room-level" ? "sfx-room.mp3" : l.file
    beds.set(f, [...(beds.get(f) || []), l])
  }
  let b = 0
  for (const [file, ls] of beds) {
    const f = join(audioDir, file)
    if (!existsSync(f)) continue
    inputs.push("-stream_loop", "-1", "-i", f)
    const k = n++
    // a stepped envelope, smoothed so level changes ramp like the live gain does
    let expr = "0"
    for (const l of ls) expr = `if(gte(t\\,${l.t})\\,${l.gain}\\,${expr})`
    parts.push(`[${k}:a]atrim=0:${secs},volume='${expr}':eval=frame,lowpass=f=12000[bed${b}]`)
    labels.push(`[bed${b++}]`)
  }
  const cutOut = card.length ? "[film]" : "[out]"
  const graph = parts.join(";") + `;${labels.join("")}amix=inputs=${labels.length}:normalize=0,atrim=0:${secs},loudnorm=I=-16:TP=-1.5${o.cut ? `,aresample=48000,volume='clip(1-(t-${o.cut})/0.01\\,0\\,1)':eval=frame` : ""}${cutOut}` +
    (card.length ? `;${card.map((l, k) => `${l}aresample=48000[c${k}]`).join(";")};[film]${card.map((_, k) => `[c${k}]`).join("")}amix=inputs=${1 + card.length}:normalize=0:duration=first[out]` : "")
  const file = join(out, "mix.m4a")
  writeFileSync(join(out, "mix-graph.txt"), graph)
  const p = Bun.spawnSync(["ffmpeg", "-y", "-loglevel", "error", ...inputs, "-filter_complex", graph, "-map", "[out]", "-c:a", "aac", "-b:a", `${o.kbps ?? 160}k`, file])
  if (p.exitCode !== 0) {
    console.log("mix failed", p.stderr.toString().slice(0, 2000))
    return null
  }
  return file
}

// A contact sheet without montage (which needs a font on some workers): rows of +append, then -append.
export async function grid(files: string[], cols: number, size: string, dest: string) {
  const rows: string[] = []
  for (let i = 0; i < files.length; i += cols) {
    const row = `${dest}.row${rows.length}.png`
    await $`magick ${files.slice(i, i + cols).map((f) => [f, "-resize", size]).flat()} -bordercolor ${"#222"} -border 3 +append ${row}`.quiet()
    rows.push(row)
  }
  await $`magick ${rows} -background ${"#222"} -append -quality 85 ${dest}`.quiet()
  for (const r of rows) rmSync(r)
}
