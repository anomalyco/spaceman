#!/usr/bin/env python3
# Build the show's audio: the monologue as one cassette recording (one long v4 take, pauses opened
# in the silences between sentences, a tape bed and a tape chain), and the SFX. Writes
# public/show/audio, manifest.json and src/show/tape.js for the film's version (mono.json "film").
# usage: process.py                  the film's version
#        process.py B --alt <dir>    one alternate version's tape only, to <dir>/tape-B.mp3 (+ .json)
import array, glob, json, math, os, re, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
raw = os.path.join(here, "raw")
out = os.path.abspath(os.path.join(here, "../../public/show/audio"))
os.makedirs(out, exist_ok=True)
script = json.load(open(os.path.join(here, "script.json")))
mono = json.load(open(os.path.join(here, "mono.json")))
args = sys.argv[1:]
ALT = args[args.index("--alt") + 1] if "--alt" in args else None
VERSION = next((a for a in args if a in mono["versions"]), mono["film"])
VER = mono["versions"][VERSION]
TAKE = VER["pick"]
HEAD = 0.95  # the record button, then the tape runs before he speaks
GAPS = VER["gaps"]  # [{"after": "everything is.", "len": 6.2, "chair": true, "sfx": "plastic"}, ...]
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


def find(words, phrase, last=False):
    toks = phrase.split()
    rng = range(len(words) - len(toks) + 1)
    for i in (reversed(rng) if last else rng):
        if all(words[i + k]["text"].lower().strip(".,?!") == toks[k].lower().strip(".,?!") for k in range(len(toks))):
            return i
    raise KeyError(phrase)


# ── the monologue ──
meta = json.load(open(os.path.join(raw, "mono", TAKE + ".json")))
words = words_of(meta)
src = os.path.join(raw, "mono", TAKE + ".mp3")
# the take's own audio, to put each cut in the quietest moment between its two words (the alignment
# can run a few hundred ms late, most of all on the last words)
_pcm = array.array("h", subprocess.run(["ffmpeg", "-v", "error", "-i", src, "-f", "s16le", "-ac", "1", "-ar", "8000", "-"], check=True, capture_output=True).stdout)


def quietest(a, b, win=0.03):
    best, bt = None, (a + b) / 2
    t = a
    while t + win <= b:
        seg = _pcm[int(t * 8000):int((t + win) * 8000)]
        e = sum(x * x for x in seg) / max(1, len(seg))
        if best is None or e < best:
            best, bt = e, t + win / 2
        t += 0.01
    return bt


# a breath between paragraphs, where the take ran them together
if VER.get("paraGap"):
    k = 0
    paras = [p for p in re.split(r"\n\s*\n", mono[VER["textKey"]]) if p.strip()]
    have = {find(words, g["after"], g.get("last", False)) + len(g["after"].split()) - 1 for g in GAPS}
    for p in paras[:-1]:
        n = len([w for w in re.findall(r"[^\s—]+", re.sub(r"\[[^\]]*\]", " ", p)) if re.search(r"[A-Za-z0-9]", w)])
        k += n
        i = k - 1
        if i not in have and words[i + 1]["t"] - words[i]["end"] < VER["paraGap"] + 0.3:
            GAPS.append({"after": None, "i": i, "len": VER["paraGap"]})


def gap_index(g):
    return g["i"] if g.get("after") is None else find(words, g["after"], g.get("last", False)) + len(g["after"].split()) - 1


cuts = []
for g in sorted(GAPS, key=gap_index):
    i = gap_index(g)
    cuts.append({**g, "at": quietest(words[i]["end"] - 0.2, max(words[i]["end"], words[i + 1]["t"] - 0.35) + 0.05), "i": i})
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
for l in VER.get("lift", []):
    i = find(words, l["phrase"])
    j = i + len(l["phrase"].split()) - 1
    lifts.append(f"volume=enable='between(t,{shift(words[i]['t']) - 0.08:.3f},{shift(words[j]['end']) + 0.12:.3f})':volume={10 ** (l['db'] / 20):.3f}")
fc.append(f"{''.join(labels)}concat=n={len(labels)}:v=0:a=1,aformat=channel_layouts=mono,adelay={int(HEAD * 1000)}{''.join(',' + x for x in lifts)}[voice]")


total = shift(end_t)
chair_cut = next(c for c in cuts if c.get("chair"))
chair_at = shift(chair_cut["at"]) + mono.get("chairAt", 0.9)
# things on the tape that aren't him: the other man's one "hm", and what happens in the last silence
extra_in, extra_fc, extra_lab = [], [], []
if VER.get("hm"):
    h = VER["hm"]
    i = find(words, h["after"]) + len(h["after"].split()) - 1
    extra_in.append(os.path.join(raw, "mono", h.get("file", "hm-3") + ".mp3"))
    at = shift(words[i]["end"]) + h["delay"]
    extra_fc.append(f"[{4 + len(extra_lab)}:a]aformat=channel_layouts=mono,lowpass=f=2400,volume={10 ** (h['db'] / 20):.3f},adelay={int(at * 1000)}[x{len(extra_lab)}]")
    extra_lab.append(f"[x{len(extra_lab)}]")
for c in cuts:
    if c.get("sfx"):
        extra_in.append(os.path.join(raw, f"sfx-{c['sfx']}.mp3"))
        at = shift(c["at"]) + c.get("sfxAt", 0.6)
        extra_fc.append(f"[{4 + len(extra_lab)}:a]aformat=channel_layouts=mono,lowpass=f=3200,volume={c.get('sfxVol', 0.35)},adelay={int(at * 1000)}[x{len(extra_lab)}]")
        extra_lab.append(f"[x{len(extra_lab)}]")
# tape bed: hiss, mains hum, the kitchen; the record button and the chair are on the tape too
fc.append(f"anoisesrc=r=44100:color=pink:amplitude=0.012:seed=7,highpass=f=1800,atrim=0:{total:.3f}[hiss]")
fc.append(f"sine=f=60:r=44100,volume=0.0016,atrim=0:{total:.3f}[hum]")
fc.append(f"[3:a]aloop=loop=-1:size=2e9,lowpass=f=1200,volume=0.32,atrim=0:{total:.3f}[kitchen]")
fc.append("[1:a]atrim=0:1.1,volume=0.7[click]")
fc.append(f"[2:a]volume=0.2,lowpass=f=3000,adelay={int(chair_at * 1000)}[chair]")
fc.extend(extra_fc)
fc.append(f"[voice][hiss][hum][kitchen][click][chair]{''.join(extra_lab)}amix=inputs={6 + len(extra_lab)}:normalize=0:duration=first[mix]")
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
     *[x for f in extra_in for x in ("-i", f)], "-filter_complex", ";".join(fc), "-map", "[tape]", "-ac", "1", "-ar", "44100", tmp])
# two-pass loudness to the same level as the other clips
m = subprocess.run(["ffmpeg", "-hide_banner", "-i", tmp, "-af", "loudnorm=I=-17.5:TP=-1.5:LRA=11:print_format=json", "-f", "null", "-"], capture_output=True, text=True).stderr
mj = json.loads(m[m.rindex("{"):m.rindex("}") + 1])
ln = f"loudnorm=I=-17.5:TP=-1.5:LRA=11:measured_I={mj['input_i']}:measured_TP={mj['input_tp']}:measured_LRA={mj['input_lra']}:measured_thresh={mj['input_thresh']}:offset={mj['target_offset']}:linear=true"
name = "voice-tape.mp3"
if ALT:
    out = ALT
    name = f"tape-{VERSION}.mp3"
run(["ffmpeg", "-y", "-i", tmp, "-af", ln, "-ac", "1", "-ar", "22050", "-b:a", "40k" if not ALT else "64k", os.path.join(out, name)])
os.remove(tmp)
pcm = subprocess.run(["ffmpeg", "-v", "error", "-i", os.path.join(out, name), "-f", "s16le", "-ac", "1", "-ar", "22050", "-"], check=True, capture_output=True).stdout
a16 = array.array("h", pcm)
hop = 441
rms = [math.sqrt(sum(x * x for x in a16[k:k + hop]) / max(1, len(a16[k:k + hop]))) for k in range(0, len(a16), hop)]
peak = max(rms) or 1
env = [min(99, round(99 * math.sqrt(r / peak))) for r in rms]
tw = [{"text": w["text"], "t": round(shift(w["t"]), 3), "end": round(shift(w["end"]), 3)} for w in words]
marks = {"gaps": [[round(shift(c["at"]), 3), c["len"]] for c in cuts], "chair": round(chair_at, 3), "head": HEAD}
# caption lines: the script's own sentences (with their dashes), timed by their words
spoken = re.sub(r"\s+", " ", re.sub(r"\[[^\]]*\]", " ", mono[VER["textKey"]])).strip()
lines, k = [], 0
for s in re.split(r"(?<=[.?!])\s+(?=[^a-z])", spoken):
    n = len([w for w in re.findall(r"[^\s—]+", s) if re.search(r"[A-Za-z0-9]", w)])
    if not n:
        continue
    lines.append({"text": s.strip(), "t": tw[k]["t"], "end": tw[k + n - 1]["end"]})
    k += n
assert k == len(tw), (k, len(tw))
if ALT:
    json.dump({"version": VERSION, "take": TAKE, "dur": round(total, 3), "lines": lines, "note": VER.get("note", "")}, open(os.path.join(ALT, f"tape-{VERSION}.json"), "w"), ensure_ascii=False)
    print(f"{VERSION}: {TAKE} {total:.2f}s")
    sys.exit(0)
manifest = {"lines": lines, "voice": {"tape": {"stage": "tape", "text": " ".join(w["text"] for w in words), "takes": [{"file": name, "dur": round(total, 3), "env": env, "words": tw, "marks": marks}]}}, "sfx": {}}

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
# the cut's clock: the engine times the picture and captions to these, with or without audio
with open(os.path.join(here, "../../src/show/tape.js"), "w") as f:
    f.write("// Generated by show/voice/process.py from the processed tape. Times are seconds after the key press.\n")
    f.write(f"export const TAPE = {json.dumps({'dur': round(total, 3), 'marks': marks})}\n")
    f.write("export const LINES = " + json.dumps([[l["text"], l["t"], l["end"]] for l in lines], ensure_ascii=False) + "\n")
    f.write("export const WORDS = " + json.dumps([[w["text"], w["t"], w["end"]] for w in tw], ensure_ascii=False) + "\n")
used = {name} | set(manifest["sfx"].values()) | {"manifest.json"}
for f in os.listdir(out):
    if f not in used:
        os.remove(os.path.join(out, f))
total_kb = sum(os.path.getsize(f) for f in glob.glob(os.path.join(out, "*"))) / 1024
print(f"tape {total:.2f}s, gaps {marks['gaps']}, {len(manifest['sfx'])} sfx, {total_kb:.0f} KB")
