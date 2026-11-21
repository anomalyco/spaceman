#!/usr/bin/env python3
# Trim voice takes to their alignment (drops leading silence and any
# repeated tail), loudness-normalize, encode small mono MP3, write manifest.json with word times.
import json, os, subprocess, glob

here = os.path.dirname(os.path.abspath(__file__))
raw = os.path.join(here, "raw")
out = os.path.abspath(os.path.join(here, "../../public/show/audio"))
os.makedirs(out, exist_ok=True)
script = json.load(open(os.path.join(here, "script.json")))

def run(args):
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

manifest = {"voice": {}, "sfx": {}}
for line in script["lines"]:
    st = script["stages"][line["stage"]]
    takes = []
    for t in range(st["takes"]):
        meta = json.load(open(os.path.join(raw, f"{line['id']}-{t}.json")))
        a = meta["alignment"]
        chars = a["characters"]
        starts = a["character_start_times_seconds"]
        ends = a["character_end_times_seconds"]
        full = "".join(chars)
        off = full.rfind("]") + 1
        while off < len(full) and full[off] == " ":
            off += 1
        body = full[off:]
        words = []
        import re
        for m in re.finditer(r"[^ \-]+-?", body):
            words.append({"text": m.group(0).rstrip("-") , "t": starts[off + m.start()]})
        t0 = max(0.0, words[0]["t"] - 0.06) 
        t1 = ends[len(full) - 1] + 0.18
        name = f"{line['id']}-{t}.mp3"
        target = "-17"
        # Natively fast delivery, tightened further with pitch-preserving time-stretch (WSOLA).
        tempo = st.get("tempo", 1.0)
        span = (t1 - t0) / tempo
        run(["ffmpeg", "-y", "-ss", f"{t0:.3f}", "-to", f"{t1:.3f}", "-i", os.path.join(raw, f"{line['id']}-{t}.mp3"),
             "-af", (f"atempo={tempo}," if tempo != 1 else "") + f"loudnorm=I={target}:TP=-1.5:LRA=11,afade=t=out:st={max(0, span - 0.08):.3f}:d=0.08",
             "-ac", "1", "-ar", "44100", "-b:a", "56k", os.path.join(out, name)])
        pcm = subprocess.run(["ffmpeg", "-v", "error", "-i", os.path.join(out, name), "-f", "s16le", "-ac", "1", "-ar", "44100", "-"], check=True, capture_output=True).stdout
        import array, math
        a16 = array.array("h", pcm)
        hop = 882  # 50 Hz mouth envelope
        rms = [math.sqrt(sum(x * x for x in a16[k:k + hop]) / max(1, len(a16[k:k + hop]))) for k in range(0, len(a16), hop)]
        peak = max(rms) or 1
        env = [min(99, round(99 * math.sqrt(r / peak))) for r in rms]
        takes.append({"file": name, "dur": round(span, 3), "env": env, "words": [{"text": w["text"], "t": round((w["t"] - t0) / tempo, 3)} for w in words]})
    manifest["voice"][line["id"]] = {"stage": line["stage"], "text": line["text"], "takes": takes[line.get("pick", 0):][:1] or takes[:1]}

trims = {"thunk": 0.35, "close": 0.4, "key": 0.18, "key2": 0.25, "shutter": 0.3}
for s in script["sfx"]:
    name = f"sfx-{s['id']}.mp3"
    dur = trims.get(s["id"], s["dur"])
    af = "loudnorm=I=-30:TP=-6" if s.get("loop") else None
    run(["ffmpeg", "-y", "-i", os.path.join(raw, name),
         "-af", af or f"silenceremove=start_periods=1:start_threshold=-45dB,atrim=0:{dur},afade=t=out:st={max(0, dur - 0.06):.3f}:d=0.06,loudnorm=I=-16:TP=-1.5",
         "-ac", "1", "-ar", "44100", "-b:a", "64k", os.path.join(out, name)])
    manifest["sfx"][s["id"]] = name

json.dump(manifest, open(os.path.join(out, "manifest.json"), "w"))
used = {t["file"] for v in manifest["voice"].values() for t in v["takes"]} | set(manifest["sfx"].values()) | {"manifest.json"}
for f in os.listdir(out):
    if f not in used:
        os.remove(os.path.join(out, f))
total = sum(os.path.getsize(f) for f in glob.glob(os.path.join(out, "*")))
print(f"{len(manifest['voice'])} lines, {len(manifest['sfx'])} sfx, {total/1024:.0f} KB")
