#!/usr/bin/env python3
# Build the show's audio: the monologue as one cassette recording (one long v4 take, two pauses
# opened in the silences between sentences, a tape bed and a tape chain), and the SFX. Writes
# public/show/audio and manifest.json with the word times of the processed tape.
import array, glob, json, math, os, re, subprocess

here = os.path.dirname(os.path.abspath(__file__))
raw = os.path.join(here, "raw")
out = os.path.abspath(os.path.join(here, "../../public/show/audio"))
os.makedirs(out, exist_ok=True)
script = json.load(open(os.path.join(here, "script.json")))
mono = json.load(open(os.path.join(here, "mono.json")))
TAKE = mono["pick"]
HEAD = 0.95  # the record button, then the tape runs before he speaks
GAPS = mono["gaps"]  # [{"after": "everything is.", "len": 6.2}, ...]
TAIL = mono["tail"]


def run(args):
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def words_of(meta):
    a = meta["alignment"]
    chars, starts, ends = a["characters"], a["character_start_times_seconds"], a["character_end_times_seconds"]
    full = "".join(chars)
    keep = [True] * len(full)
    for m in re.finditer(r"\[[^\]]*\]", full):
        for i in range(m.start(), m.end()):
            keep[i] = False
    spoken = "".join(c if k else " " for c, k in zip(full, keep))
    words = []
    for m in re.finditer(r"[^\s—]+", spoken):
        w = m.group(0)
        if not re.search(r"[A-Za-z0-9]", w):
            continue
        words.append({"text": w, "t": starts[m.start()], "end": ends[m.end() - 1]})
    return words


def find(words, phrase):
    toks = phrase.split()
    for i in range(len(words) - len(toks) + 1):
        if all(words[i + k]["text"].lower().strip(".,?!") == toks[k].lower().strip(".,?!") for k in range(len(toks))):
            return i
    raise KeyError(phrase)


# ── the monologue ──
meta = json.load(open(os.path.join(raw, "mono", TAKE + ".json")))
words = words_of(meta)
src = os.path.join(raw, "mono", TAKE + ".mp3")
cuts = []
for g in GAPS:
    i = find(words, g["after"]) + len(g["after"].split()) - 1
    cuts.append({"at": (words[i]["end"] + words[i + 1]["t"]) / 2, "len": g["len"], "i": i})
end_t = words[-1]["end"] + TAIL

# voice: segments with silence between, delayed by HEAD
parts, labels, t0 = [], [], 0.0
fc = []
for k, c in enumerate(cuts + [{"at": end_t, "len": 0}]):
    fc.append(f"[0:a]atrim={t0:.3f}:{c['at']:.3f},asetpts=PTS-STARTPTS[s{k}]")
    labels.append(f"[s{k}]")
    if c["len"]:
        fc.append(f"anullsrc=r=44100:cl=mono,atrim=0:{c['len']:.3f}[z{k}]")
        labels.append(f"[z{k}]")
    t0 = c["at"]


def shift(t):
    return HEAD + t + sum(c["len"] for c in cuts if t > c["at"])


# lift single soft words so no passage drops out of the mix (the old "Mail went to two." problem)
lifts = []
for l in mono.get("lift", []):
    i = find(words, l["phrase"])
    j = i + len(l["phrase"].split()) - 1
    lifts.append(f"volume=enable='between(t,{shift(words[i]['t']) - 0.08:.3f},{shift(words[j]['end']) + 0.12:.3f})':volume={10 ** (l['db'] / 20):.3f}")
fc.append(f"{''.join(labels)}concat=n={len(labels)}:v=0:a=1,aformat=channel_layouts=mono,adelay={int(HEAD * 1000)}{''.join(',' + x for x in lifts)}[voice]")


total = shift(end_t)
chair_at = shift(cuts[-1]["at"]) + mono.get("chairAt", 0.9)
# tape bed: hiss, mains hum, the kitchen; the record button and the chair are on the tape too
fc.append(f"anoisesrc=r=44100:color=pink:amplitude=0.012:seed=7,highpass=f=1800,atrim=0:{total:.3f}[hiss]")
fc.append(f"sine=f=60:r=44100,volume=0.0016,atrim=0:{total:.3f}[hum]")
fc.append(f"[3:a]aloop=loop=-1:size=2e9,lowpass=f=1200,volume=0.32,atrim=0:{total:.3f}[kitchen]")
fc.append("[1:a]atrim=0:1.1,volume=0.7[click]")
fc.append(f"[2:a]volume=0.2,lowpass=f=3000,adelay={int(chair_at * 1000)}[chair]")
fc.append("[voice][hiss][hum][kitchen][click][chair]amix=inputs=6:normalize=0:duration=first[mix]")
chain = ",".join([
    "highpass=f=140", "lowpass=f=5600", "equalizer=f=2600:t=q:w=1.2:g=2.5",
    "acompressor=threshold=-26dB:ratio=3:attack=8:release=220:makeup=4",
    "vibrato=f=0.55:d=0.18", "vibrato=f=7.3:d=0.008",
    "asoftclip=type=tanh",
    f"atrim=0:{total:.3f}",
])
fc.append(f"[mix]{chain}[tape]")
tmp = os.path.join(raw, "mono", "tape.wav")
run(["ffmpeg", "-y", "-i", src, "-i", os.path.join(raw, "sfx-tapeclick.mp3"), "-i", os.path.join(raw, "sfx-chair.mp3"), "-i", os.path.join(raw, "sfx-room.mp3"),
     "-filter_complex", ";".join(fc), "-map", "[tape]", "-ac", "1", "-ar", "44100", tmp])
# two-pass loudness to the same level as the other clips
m = subprocess.run(["ffmpeg", "-hide_banner", "-i", tmp, "-af", "loudnorm=I=-17.5:TP=-1.5:LRA=11:print_format=json", "-f", "null", "-"], capture_output=True, text=True).stderr
mj = json.loads(m[m.rindex("{"):m.rindex("}") + 1])
ln = f"loudnorm=I=-17.5:TP=-1.5:LRA=11:measured_I={mj['input_i']}:measured_TP={mj['input_tp']}:measured_LRA={mj['input_lra']}:measured_thresh={mj['input_thresh']}:offset={mj['target_offset']}:linear=true"
name = "voice-tape.mp3"
run(["ffmpeg", "-y", "-i", tmp, "-af", ln, "-ac", "1", "-ar", "22050", "-b:a", "40k", os.path.join(out, name)])
os.remove(tmp)
pcm = subprocess.run(["ffmpeg", "-v", "error", "-i", os.path.join(out, name), "-f", "s16le", "-ac", "1", "-ar", "22050", "-"], check=True, capture_output=True).stdout
a16 = array.array("h", pcm)
hop = 441
rms = [math.sqrt(sum(x * x for x in a16[k:k + hop]) / max(1, len(a16[k:k + hop]))) for k in range(0, len(a16), hop)]
peak = max(rms) or 1
env = [min(99, round(99 * math.sqrt(r / peak))) for r in rms]
tw = [{"text": w["text"], "t": round(shift(w["t"]), 3), "end": round(shift(w["end"]), 3)} for w in words]
marks = {"gaps": [[round(shift(c["at"]), 3), c["len"]] for c in cuts], "chair": round(chair_at, 3), "head": HEAD}
manifest = {"voice": {"tape": {"stage": "tape", "text": " ".join(w["text"] for w in words), "takes": [{"file": name, "dur": round(total, 3), "env": env, "words": tw, "marks": marks}]}}, "sfx": {}}

# ── sound effects ──
trims = {"thunk": 0.35, "close": 0.4, "key": 0.18, "key2": 0.25, "shutter": 0.3}
for s in script["sfx"]:
    if s.get("tape"):
        continue
    sname = f"sfx-{s['id']}.mp3"
    dur = trims.get(s["id"], s["dur"])
    af = "loudnorm=I=-30:TP=-6" if s.get("loop") else None
    run(["ffmpeg", "-y", "-i", os.path.join(raw, sname),
         "-af", af or f"silenceremove=start_periods=1:start_threshold=-45dB,atrim=0:{dur},afade=t=out:st={max(0, dur - 0.06):.3f}:d=0.06,loudnorm=I=-16:TP=-1.5",
         "-ac", "1", "-ar", "44100", "-b:a", "64k", os.path.join(out, sname)])
    manifest["sfx"][s["id"]] = sname

json.dump(manifest, open(os.path.join(out, "manifest.json"), "w"))
used = {name} | set(manifest["sfx"].values()) | {"manifest.json"}
for f in os.listdir(out):
    if f not in used:
        os.remove(os.path.join(out, f))
total_kb = sum(os.path.getsize(f) for f in glob.glob(os.path.join(out, "*"))) / 1024
print(f"tape {total:.2f}s, gaps {marks['gaps']}, {len(manifest['sfx'])} sfx, {total_kb:.0f} KB")
