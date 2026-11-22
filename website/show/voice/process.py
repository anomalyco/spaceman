#!/usr/bin/env python3
# Build the show's audio: the monologue as one recorded police interview (the recorder's tone, the
# detective resuming the interview, his one long v4 take with pauses opened in the silences between
# sentences, the room, the detective suspending it, the tone again), and the SFX. Writes
# public/show/audio, manifest.json and src/show/tape.js for the film's version (mono.json "film").
# usage: process.py                  the film's version (needs numpy for keys.py:
#                                    uv run -q --with numpy python process.py)
#        process.py B --alt <dir>    one alternate version's tape only, to <dir>/tape-B.mp3 (+ .json)
import array, glob, json, math, os, re, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
raw = os.path.join(here, "raw")
out = os.path.abspath(os.path.join(here, "../../public/show/audio"))
sys.path.insert(0, here)
import score  # noqa: E402
os.makedirs(out, exist_ok=True)
script = json.load(open(os.path.join(here, "script.json")))
mono = json.load(open(os.path.join(here, "mono.json")))
args = sys.argv[1:]
ALT = args[args.index("--alt") + 1] if "--alt" in args else None
VERSION = next((a for a in args if a in mono["versions"]), mono["film"])
VER = mono["versions"][VERSION]
TAKE = VER["pick"]
BEEP = 1.1  # the interview recorder's tone, at the start and the end


def dur(f):
    return float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", f], capture_output=True, text=True).stdout)


# the detective's lines (his two at either end, and any he puts in between), trimmed of their own
# silence at both ends
INTER = {}
# a version can open cold on his voice and end on his last word ("bookends": false): no detective at
# either end and no recorder tones, so it isn't clear it's an interview until the detective's questions
ENDS = VER.get("bookends", True)
for k in (*(("open", "close") if ENDS else ()), *[x["line"] for x in VER.get("inserts", [])]):
    f = os.path.join(raw, "inter", k + "-trim.wav")
    pick = mono["interviewer"].get("pick", {}).get(k, k + "-1")
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", os.path.join(raw, "inter", pick if "." in pick else pick + ".mp3"),
                    "-af", "silenceremove=start_periods=1:start_threshold=-42dB,areverse,silenceremove=start_periods=1:start_threshold=-42dB,areverse",
                    "-ac", "1", "-ar", "44100", f], check=True)
    INTER[k] = {"file": f, "dur": dur(f)}
OPEN_AT = 0.35 + BEEP + 0.75
HEAD = OPEN_AT + INTER["open"]["dur"] + 0.85 if ENDS else VER["headAt"]  # he starts after the detective
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
# where the aligner collapsed a phrase onto the next one (it does on the take's first words), its real
# times in the take, measured from the waveform
for fx in VER.get("retime", []):
    i = find(words, fx["phrase"])
    toks = fx["phrase"].split()
    step = (fx["end"] - fx["t"]) / len(toks)
    for k in range(len(toks)):
        words[i + k]["t"] = round(fx["t"] + k * step, 3)
        words[i + k]["end"] = round(fx["t"] + (k + 1) * step, 3)
src = os.path.join(raw, "mono", TAKE + ".mp3")
# his first words from another take of the same voice and direction ("head": {"take": "s3", "cut": 2.38,
# "from": 0.8, "gain": -1.6, "words": [[text, t, end], ...]}): that take up to "cut", then this take from
# "from", both in silence. The head's word times are measured from its waveform (the aligner collapses
# a take's first words), and its level is matched on the sentence both takes share.
head_wav = None
if VER.get("head"):
    hd = VER["head"]
    head_wav = os.path.join(raw, "mono", TAKE + "-head.wav")
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", os.path.join(raw, "mono", hd["take"] + ".mp3"), "-i", src, "-filter_complex",
                    f"[0:a]atrim=0:{hd['cut']},asetpts=PTS-STARTPTS,aformat=sample_rates=44100:channel_layouts=mono,volume={hd['gain']}dB[h];"
                    f"[1:a]atrim={hd['from']},asetpts=PTS-STARTPTS,aformat=sample_rates=44100:channel_layouts=mono[b];[h][b]concat=n=2:v=0:a=1",
                    "-ac", "1", "-ar", "44100", head_wav], check=True)
    d = hd["cut"] - hd["from"]
    words = [{"text": a, "t": b, "end": c} for a, b, c in hd["words"]] + [{**w, "t": w["t"] + d, "end": w["end"] + d} for w in words if w["t"] >= hd["from"]]
    src = head_wav
meta_words = words
# a phrase said twice that should sound the same both times: the second reading is replaced by the
# first one from the same take, levelled to where it lands ("echo": {"src": [a, b], "dst": [a, b]},
# seconds in the take, cut in silence)
if VER.get("echo"):
    (a1, b1), (a2, b2) = VER["echo"]["src"], VER["echo"]["dst"]
    def level(a, b):
        m = subprocess.run(["ffmpeg", "-hide_banner", "-i", src, "-af", f"atrim={a}:{b},volumedetect", "-f", "null", "-"], capture_output=True, text=True).stderr
        return float(re.search(r"mean_volume: (-?[\d.]+)", m).group(1))
    gain = level(a2, b2) - level(a1, b1)
    echo = os.path.join(raw, "mono", TAKE + "-echo.wav")
    run_fc = f"[0:a]atrim=0:{a2},asetpts=PTS-STARTPTS[p];[0:a]atrim={a1}:{b1},asetpts=PTS-STARTPTS,volume={gain}dB,afade=t=in:d=0.01,afade=t=out:st={b1 - a1 - 0.01}:d=0.01[e];[0:a]atrim={b2},asetpts=PTS-STARTPTS[q];[p][e][q]concat=n=3:v=0:a=1"
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", src, "-filter_complex", run_fc, "-ac", "1", "-ar", "44100", echo], check=True)
    d = (b1 - a1) - (b2 - a2)
    first = [w for w in words if a1 <= w["t"] < b1]
    i2 = next(k for k, w in enumerate(words) if w["t"] >= a2)
    for k, w in enumerate(first):
        words[i2 + k] = {**words[i2 + k], "t": a2 + w["t"] - a1, "end": min(a2 + b1 - a1, a2 + w["end"] - a1)}
    for w in words[i2 + len(first):]:
        w["t"] += d
        w["end"] += d
    src = echo
# no dead air ("tighten": {"para": 0.75, "sentence": 0.6, "comma": 0.4}): the take's silences after a
# paragraph, a sentence or a comma longer than that lose their middles, measured from the waveform, so
# his speech itself is untouched
tight = None
if VER.get("tighten"):
    cap = VER["tighten"]
    det = subprocess.run(["ffmpeg", "-hide_banner", "-i", src, "-af", "silencedetect=n=-45dB:d=0.2", "-f", "null", "-"], capture_output=True, text=True).stderr
    sil = [(float(a), float(b)) for a, b in zip(re.findall(r"silence_start: ([\d.]+)", det), re.findall(r"silence_end: ([\d.]+)", det))]
    ends, n = set(), 0
    for p in [p for p in re.split(r"\n\s*\n", mono[VER["textKey"]]) if p.strip()]:
        n += len([w for w in re.findall(r"[^\s—]+", re.sub(r"\[[^\]]*\]", " ", p)) if re.search(r"[A-Za-z0-9]", w)])
        ends.add(n - 1)
    cut = []
    for a, b in sil:
        m = (a + b) / 2
        k = min(range(len(words) - 1), key=lambda k: abs((words[k]["end"] + words[k + 1]["t"]) / 2 - m))
        w = words[k]["text"]
        c = cap["para"] if k in ends else cap["sentence"] if re.search(r"[.?!]$", w) else cap["comma"] if re.search(r"[,;:]$", w) else None
        if c is not None and b - a > c + 0.05:
            cut.append((a + c / 2, b - c / 2))
    tight = os.path.join(raw, "mono", TAKE + "-tight.wav")
    keep, t0 = [], 0.0
    for a, b in cut:
        keep.append((t0, a))
        t0 = b
    keep.append((t0, None))
    fc_t = ";".join(f"[0:a]atrim={a:.3f}" + (f":{b:.3f}" if b else "") + f",asetpts=PTS-STARTPTS[k{i}]" for i, (a, b) in enumerate(keep))
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", src, "-filter_complex", fc_t + ";" + "".join(f"[k{i}]" for i in range(len(keep))) + f"concat=n={len(keep)}:v=0:a=1", "-ac", "1", "-ar", "44100", tight], check=True)
    def moved(t):
        return t - sum(min(b, max(a, t)) - a for a, b in cut)
    for w in words:
        w["t"], w["end"] = moved(w["t"]), moved(w["end"])
    src = tight
    print(f"tighten: {len(cut)} silences, -{sum(b - a for a, b in cut):.1f}s")
# a version that stops early ("endAfter": "It was her idea."): the words after it are left out
if VER.get("endAfter"):
    words = words[:find(words, VER["endAfter"], True) + len(VER["endAfter"].split())]
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
    have = {find(words, g["after"], g.get("last", False)) + len(g["after"].split()) - 1 for g in GAPS + VER.get("inserts", [])}
    have |= {find(words, p) + len(p.split()) - 1 for p in VER.get("noBreath", [])}
    for p in paras[:-1]:
        n = len([w for w in re.findall(r"[^\s—]+", re.sub(r"\[[^\]]*\]", " ", p)) if re.search(r"[A-Za-z0-9]", w)])
        k += n
        i = k - 1
        if i + 1 >= len(words):
            break
        if i not in have and words[i + 1]["t"] - words[i]["end"] < VER["paraGap"] + 0.3:
            GAPS.append({"after": None, "i": i, "len": VER["paraGap"]})


# the detective speaking into a silence opened for him
for x in VER.get("inserts", []):
    GAPS.append({"after": x["after"], "last": x.get("last", False), "len": x.get("pre", 0.8) + INTER[x["line"]]["dur"] + x.get("post", 0.7), "insert": x})


def gap_index(g):
    return g["i"] if g.get("after") is None else find(words, g["after"], g.get("last", False)) + len(g["after"].split()) - 1


cuts = []
for g in sorted(GAPS, key=gap_index):
    i = gap_index(g)
    cuts.append({**g, "at": quietest(words[i]["end"] - 0.2, max(words[i]["end"], words[i + 1]["t"] - 0.35) + 0.05), "i": i})
# a passage left out (a shorter cut, "drop": [{"from": "Alan came over", "to": "the plastic down."}]):
# the take jumps from the quietest moment before its first word to the quietest after its last, and
# any pause opened inside it goes with it
for d in VER.get("drop", []):
    i = find(words, d["from"])
    j = find(words, d["to"]) + len(d["to"].split()) - 1
    a = quietest(words[i - 1]["end"] - 0.2, max(words[i - 1]["end"], words[i]["t"] - 0.35) + 0.05)
    b = quietest(words[j]["end"] - 0.2, max(words[j]["end"], words[j + 1]["t"] - 0.35) + 0.05)
    cuts = [c for c in cuts if not a < c["at"] < b] + [{"at": a, "len": 0, "skip": b - a, "i": i - 1, "drop": [i, j]}]
cuts.sort(key=lambda c: (c["at"], 1 if c.get("skip") else 0))
# the aligner's last-letter times run late; a word never ends after the cut that follows it
for c in cuts:
    words[c["i"]]["end"] = min(words[c["i"]]["end"], c["at"])
end_t = words[-1]["end"] + TAIL
if VER.get("endAfter"):
    nxt = meta_words[len(words)]["t"]
    end_t = quietest(words[-1]["end"] - 0.1, max(words[-1]["end"], nxt - 0.05))

# voice: segments with silence between, delayed by HEAD
parts, labels, t0 = [], [], 0.0
fc = []
for k, c in enumerate(cuts + [{"at": end_t, "len": 0}]):
    if c["at"] - t0 > 0.001:
        fc.append(f"[0:a]atrim={t0:.3f}:{c['at']:.3f},asetpts=PTS-STARTPTS[s{k}]")
        labels.append(f"[s{k}]")
    if c["len"]:
        fc.append(f"anullsrc=r=44100:cl=mono,atrim=0:{c['len']:.3f}[z{k}]")
        labels.append(f"[z{k}]")
    t0 = c["at"] + c.get("skip", 0)


def shift(t):
    return HEAD + t + sum(c["len"] - c.get("skip", 0) for c in cuts if t > c["at"])


dropped = {k for c in cuts if c.get("drop") for k in range(c["drop"][0], c["drop"][1] + 1)}
words = [w for k, w in enumerate(words) if k not in dropped]


# lift single soft words so no passage drops out of the mix (the old "Mail went to two." problem)
lifts = []
for l in VER.get("lift", []):
    i = find(words, l["phrase"])
    j = i + len(l["phrase"].split()) - 1
    lifts.append(f"volume=enable='between(t,{shift(words[i]['t']) - 0.08:.3f},{shift(words[j]['end']) + 0.12:.3f})':volume={10 ** (l['db'] / 20):.3f}")
fc.append(f"{''.join(labels)}concat=n={len(labels)}:v=0:a=1,aformat=channel_layouts=mono,adelay={int(HEAD * 1000)}{''.join(',' + x for x in lifts)}[voice]")


said = shift(end_t)
if ENDS:
    CLOSE_AT = said + VER.get("closeGap", 2.6)
    BEEP2 = CLOSE_AT + INTER["close"]["dur"] + 0.9
    total = BEEP2 + BEEP + 0.5
else:
    # the room runs on a moment after his last word, and stops
    CLOSE_AT = BEEP2 = None
    total = said + VER.get("roomAfter", 2.5)
chair_cut = next((c for c in cuts if c.get("chair")), None)
chair_at = shift(chair_cut["at"]) + mono.get("chairAt", 0.9) if chair_cut else None
# things on the tape that aren't him: the other man's one "hm", and what happens in the last silence
extra_in, extra_fc, extra_lab = [], [], []
if VER.get("hm"):
    h = VER["hm"]
    i = find(words, h["after"]) + len(h["after"].split()) - 1
    extra_in.append(os.path.join(raw, "mono", h.get("file", "hm-3") + ".mp3"))
    at = shift(words[i]["end"]) + h["delay"]
    extra_fc.append(f"[{6 + len(extra_lab)}:a]aformat=channel_layouts=mono,lowpass=f=2400,volume={10 ** (h['db'] / 20):.3f},adelay={int(at * 1000)}[x{len(extra_lab)}]")
    extra_lab.append(f"[x{len(extra_lab)}]")
# the recorder's play key and its reels, under the room from the first frame
extra_in.append(score.recorder(os.path.join(raw, "mono", "recorder.wav")))
extra_fc.append(f"[{6 + len(extra_lab)}:a]aformat=channel_layouts=mono[x{len(extra_lab)}]")
extra_lab.append(f"[x{len(extra_lab)}]")
for c in cuts:
    if c.get("sfx"):
        extra_in.append(os.path.join(raw, f"sfx-{c['sfx']}.mp3"))
        at = shift(c["at"]) + c.get("sfxAt", 0.6)
        extra_fc.append(f"[{6 + len(extra_lab)}:a]aformat=channel_layouts=mono,lowpass=f=3200,volume={c.get('sfxVol', 0.35)},adelay={int(at * 1000)}[x{len(extra_lab)}]")
        extra_lab.append(f"[x{len(extra_lab)}]")
# the interview room: ventilation, the strip light's buzz, the recorder's own floor; the tones at
# either end; the detective further from the microphone than he is
fc.append(f"anoisesrc=r=44100:color=brown:amplitude=0.016:seed=11,lowpass=f=420,highpass=f=60,atrim=0:{total:.3f}[vent]")
fc.append(f"anoisesrc=r=44100:color=pink:amplitude=0.0025:seed=7,highpass=f=2500,atrim=0:{total:.3f}[hiss]")
fc.append(f"sine=f=100:r=44100,volume=0.0011,atrim=0:{total:.3f}[hum1]")
fc.append(f"sine=f=300:r=44100,volume=0.0004,atrim=0:{total:.3f}[hum3]")
tone = "sine=f=1000:r=44100,atrim=0:{d},afade=t=in:d=0.01,afade=t=out:st={o}:d=0.02,volume=0.085,adelay={ms}"
far = "aformat=channel_layouts=mono,highpass=f=200,lowpass=f=4000,aecho=0.8:0.6:19|37|61:0.36|0.26|0.14,volume=0.25"
if ENDS:
    fc.append(tone.format(d=BEEP, o=BEEP - 0.02, ms=350) + "[beep1]")
    fc.append(tone.format(d=BEEP, o=BEEP - 0.02, ms=int(BEEP2 * 1000)) + "[beep2]")
    fc.append(f"[1:a]{far},adelay={int(OPEN_AT * 1000)}[iopen]")
    fc.append(f"[2:a]{far},adelay={int(CLOSE_AT * 1000)}[iclose]")
inserts = [c for c in cuts if c.get("insert")]
for k, c in enumerate(inserts):
    c["said_at"] = shift(c["at"]) + c["insert"].get("pre", 0.8)
    fc.append(f"[{6 + len(extra_in) + k}:a]{far},adelay={int(c['said_at'] * 1000)}[imid{k}]")
fc.append("[voice]aecho=0.9:0.4:13|29:0.16|0.08[near]")
mixins = ["[near]", "[vent]", "[hiss]", "[hum1]", "[hum3]", *(["[beep1]", "[beep2]"] if ENDS else [])]
if chair_at is not None:
    fc.append(f"[3:a]volume=0.2,lowpass=f=3000,adelay={int(chair_at * 1000)}[chair]")
    mixins.append("[chair]")
fc.extend(extra_fc)
fc.append(f"{''.join(mixins)}{''.join(extra_lab)}amix=inputs={len(mixins) + len(extra_lab)}:normalize=0:duration=longest[mix]")
chain = ",".join([
    "highpass=f=90", "lowpass=f=7200", "equalizer=f=3000:t=q:w=1.0:g=1.5", "equalizer=f=220:t=q:w=1.0:g=1.5",
    "acompressor=threshold=-24dB:ratio=3:attack=6:release=200:makeup=3.5",
    "alimiter=limit=0.9",
    f"apad=whole_dur={total:.3f},atrim=0:{total:.3f}",
])
# the detective goes in after the compressor, so he stays as far off as he was placed
fc.append(f"[mix]{chain}[near_tape]")
ends_in = "[iopen][iclose]" if ENDS else ""
fc.append(f"[near_tape]{ends_in}{''.join(f'[imid{k}]' for k in range(len(inserts)))}amix=inputs={1 + (2 if ENDS else 0) + len(inserts)}:normalize=0:duration=first[tape]")
tmp = os.path.join(raw, "mono", "tape.wav")
run(["ffmpeg", "-y", "-i", src, "-i", INTER["open"]["file"] if ENDS else src, "-i", INTER["close"]["file"] if ENDS else src, "-i", os.path.join(raw, "sfx-chair.mp3"), "-i", os.path.join(raw, "sfx-room.mp3"), "-i", os.path.join(raw, "sfx-tapeclick.mp3"),
     *[x for f in extra_in for x in ("-i", f)], *[x for c in inserts for x in ("-i", INTER[c["insert"]["line"]]["file"])], "-filter_complex", ";".join(fc), "-map", "[tape]", "-ac", "1", "-ar", "44100", tmp])
# two-pass loudness to the same level as the other clips
m = subprocess.run(["ffmpeg", "-hide_banner", "-i", tmp, "-af", "loudnorm=I=-17.5:TP=-1.5:LRA=11:print_format=json", "-f", "null", "-"], capture_output=True, text=True).stderr
mj = json.loads(m[m.rindex("{"):m.rindex("}") + 1])
ln = f"loudnorm=I=-17.5:TP=-1.5:LRA=11:measured_I={mj['input_i']}:measured_TP={mj['input_tp']}:measured_LRA={mj['input_lra']}:measured_thresh={mj['input_thresh']}:offset={mj['target_offset']}:linear=true"
name = "voice-tape.mp3"
if ALT:
    out = ALT
    name = f"tape-{VERSION}.mp3"
if VER.get("echo") or tight or head_wav:
    os.remove(src)
if head_wav and os.path.exists(head_wav):
    os.remove(head_wav)
run(["ffmpeg", "-y", "-i", tmp, "-af", ln, "-ac", "1", "-ar", "22050", "-b:a", "40k" if not ALT else "64k", os.path.join(out, name)])
os.remove(tmp)
pcm = subprocess.run(["ffmpeg", "-v", "error", "-i", os.path.join(out, name), "-f", "s16le", "-ac", "1", "-ar", "22050", "-"], check=True, capture_output=True).stdout
a16 = array.array("h", pcm)
hop = 441
rms = [math.sqrt(sum(x * x for x in a16[k:k + hop]) / max(1, len(a16[k:k + hop]))) for k in range(0, len(a16), hop)]
peak = max(rms) or 1
env = [min(99, round(99 * math.sqrt(r / peak))) for r in rms]
tw = [{"text": w["text"], "t": round(shift(w["t"]), 3), "end": round(shift(w["end"]), 3)} for w in words]
marks = {"gaps": [[round(shift(c["at"]), 3), c["len"]] for c in cuts], "chair": round(chair_at, 3) if chair_at else None, "head": round(HEAD, 3),
         "open": round(OPEN_AT, 3) if ENDS else None, "close": round(CLOSE_AT, 3) if ENDS else None, "beep2": round(BEEP2, 3) if ENDS else None, "said": round(said, 3),
         # the last cut: a held second on seven after his last word
         "end": round(tw[-1]["end"] + VER.get("hold", 1.0), 3)}
# caption lines: the script's own sentences (with their dashes), timed by their words
spoken = re.sub(r"\s+", " ", re.sub(r"\[[^\]]*\]", " ", mono[VER["textKey"]])).strip()
for d in VER.get("drop", []):
    spoken = re.sub(r"\s+", " ", re.sub(re.escape(d["from"]) + ".*?" + re.escape(d["to"]), " ", spoken)).strip()
lines, k = [], 0
for s in re.split(r"(?<=[.?!])\s+(?=[^a-z])", spoken):
    n = len([w for w in re.findall(r"[^\s—]+", s) if re.search(r"[A-Za-z0-9]", w)])
    if not n:
        continue
    if k + n > len(tw):
        break
    lines.append({"text": s.strip(), "t": tw[k]["t"], "end": tw[k + n - 1]["end"]})
    k += n
assert k == len(tw), (k, len(tw))
for k2, at in ((("open", OPEN_AT), ("close", CLOSE_AT)) if ENDS else ()):
    lines.append({"text": mono["interviewer"]["captions"][k2], "t": round(at, 3), "end": round(at + INTER[k2]["dur"], 3), "who": "detective"})
for c in inserts:
    k2 = c["insert"]["line"]
    lines.append({"text": mono["interviewer"]["captions"][k2], "t": round(c["said_at"], 3), "end": round(c["said_at"] + INTER[k2]["dur"], 3), "who": "detective"})
lines.sort(key=lambda l: l["t"])
if ALT:
    json.dump({"version": VERSION, "take": TAKE, "dur": round(total, 3), "lines": lines, "note": VER.get("note", "")}, open(os.path.join(ALT, f"tape-{VERSION}.json"), "w"), ensure_ascii=False)
    print(f"{VERSION}: {TAKE} {total:.2f}s")
    sys.exit(0)
manifest = {"lines": lines, "voice": {"tape": {"stage": "tape", "text": " ".join(w["text"] for w in words), "takes": [{"file": name, "dur": round(total, 3), "env": env, "words": tw, "marks": marks}]}}, "sfx": {}}

# ── sound effects ──
trims = {}
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
# the key presses and the trackpad click (keys.py)
import keys  # noqa: E402

keys.build(manifest)
# the strings, ducked under everything said on the tape
said_spans = [(w["t"], w["end"]) for w in tw] + [(c["said_at"], c["said_at"] + INTER[c["insert"]["line"]]["dur"]) for c in inserts]
score.build(manifest, said_spans, marks["end"], out)

json.dump(manifest, open(os.path.join(out, "manifest.json"), "w"))
# the cut's clock: the engine times the picture and captions to these, with or without audio
with open(os.path.join(here, "../../src/show/tape.js"), "w") as f:
    f.write("// Generated by show/voice/process.py from the processed tape. Times are seconds after the key press.\n")
    f.write(f"export const TAPE = {json.dumps({'dur': round(total, 3), 'marks': marks})}\n")
    f.write("export const LINES = " + json.dumps([[l["text"], l["t"], l["end"]] for l in lines], ensure_ascii=False) + "\n")
    f.write("export const WORDS = " + json.dumps([[w["text"], w["t"], w["end"]] for w in tw], ensure_ascii=False) + "\n")
used = {name} | {f for s in manifest["sfx"].values() for f in (s["variants"] if isinstance(s, dict) else [s])} | {"manifest.json", manifest["score"]["file"]}
for f in os.listdir(out):
    if f not in used:
        os.remove(os.path.join(out, f))
total_kb = sum(os.path.getsize(f) for f in glob.glob(os.path.join(out, "*"))) / 1024
print(f"tape {total:.2f}s, gaps {marks['gaps']}, {len(manifest['sfx'])} sfx, {total_kb:.0f} KB")
