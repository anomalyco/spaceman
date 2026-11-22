# The film's two non-voice tracks, built from ElevenLabs takes in raw/score (gen-score.ts):
#
#   recorder(): the interview recorder's play button and its reels settling in, mixed into the tape at 0.0
#   build():    the string score, one stereo file aligned to show time from SCORE_AT, ducked under every
#               word of the tape and swelling into the pauses; it stops dead at the film's last cut
#
# Both are deterministic: the same takes and words make the same files.
import os, subprocess
import numpy as np

here = os.path.dirname(os.path.abspath(__file__))
raw = os.path.join(here, "raw", "score")
SR = 44100
# the first half-minute is dry: the strings come in with the door camera
SCORE_AT = 30.0
# stem, take, where its first sample sits in show time, and its fades (show time)
# cello-1 is silent for 2 s and creeps in from nothing; dread-1 builds for 33 s; peak-1 crests 20-22 s in
# lift: extra dB over show time (the peak's first seventeen seconds are much quieter than its crest;
# on seven they hold a taut floor instead)
STEMS = [
    ("cello-1", 33.0, (33.0, 0.5), (53.5, 4.0), []),
    # "Where is Alan now?" falls into the hush between the dread's tail and the peak's first bow
    ("dread-1", 55.0, (55.0, 3.0), (88.0, 3.0), []),
    ("peak", None, (89.0, 2.0), None, [(91, 9), (103, 8), (107.5, 2), (110, 0)]),
]
PEAK_CREST = 21.3  # seconds into peak-1 where the cut should land


def load(path, ch=2):
    pcm = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-f", "f32le", "-ac", str(ch), "-ar", str(SR), "-"], check=True, capture_output=True).stdout
    return np.frombuffer(pcm, dtype=np.float32).reshape(-1, ch).copy()


def db(x):
    return 10 ** (x / 20)


def recorder(dest):
    """The play key's clunk (tape-0's first 0.6 s), then the reels' whir (whir-0) easing down to the
    tape's own hiss over five seconds."""
    clunk = load(os.path.join(raw, "tape-0.mp3"), 1)[:, 0][: int(0.62 * SR)]
    clunk = clunk * np.minimum(1, np.linspace(1, 0, len(clunk)) * 4) * db(-7)
    whir = load(os.path.join(raw, "whir-0.mp3"), 1)[:, 0][int(0.1 * SR): int(2.7 * SR)]
    # two passes, crossfaded, for five seconds of motor
    xf = int(0.4 * SR)
    ramp = np.linspace(0, 1, xf)
    loop = np.concatenate([whir[:-xf], whir[-xf:] * np.sqrt(1 - ramp) + whir[:xf] * np.sqrt(ramp), whir[xf:]])
    n = int(6.0 * SR)
    loop = np.resize(loop, n)
    t = np.arange(n) / SR
    # the motor comes up to speed just after the key, then settles under the room
    env = np.clip((t - 0.06) / 0.18, 0, 1) * (db(-5) * np.exp(-t / 1.2) + db(-17)) * np.clip((6.0 - t) / 2.5, 0, 1)
    out = np.zeros(n, dtype=np.float32)
    out[: len(clunk)] += clunk
    out += loop * env * db(4)
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-f", "f32le", "-ac", "1", "-ar", str(SR), "-i", "-", dest], input=out.astype(np.float32).tobytes(), check=True)
    return dest


def speaking(words, n):
    """1 while he (or the detective) is talking, 0 in the pauses; breaths shorter than 0.7 s count as talk."""
    iv = sorted((w[0] - 0.15, w[1] + 0.25) for w in words)
    merged = []
    for a, b in iv:
        if merged and a - merged[-1][1] < 0.7:
            merged[-1][1] = max(merged[-1][1], b)
        else:
            merged.append([a, b])
    m = np.zeros(n, dtype=np.float32)
    for a, b in merged:
        m[max(0, int((a - SCORE_AT) * SR)): max(0, int((b - SCORE_AT) * SR))] = 1
    return m


def smooth_db(target, up, down):
    """One-pole smoothing of a dB curve, faster down (the duck) than up (the swell)."""
    hop = 441
    k = target[::hop]
    out = np.empty_like(k)
    v = k[0]
    au, ad = 1 - np.exp(-hop / (SR * up)), 1 - np.exp(-hop / (SR * down))
    for i, x in enumerate(k):
        v += (x - v) * (au if x > v else ad)
        out[i] = v
    return np.repeat(out, hop)[: len(target)]


def build(manifest, words, end, out):
    """words: [(t, end)] of everything said; end: the film's last cut (show time)."""
    n = int((end + 0.05 - SCORE_AT) * SR)
    t = SCORE_AT + np.arange(n) / SR
    mix = np.zeros((n, 2), dtype=np.float32)
    for name, at, fin, fout, lift in STEMS:
        x = load(os.path.join(raw, name + ".mp3") if name != "peak" else os.path.join(raw, "peak-1.mp3"))
        if at is None:
            at = end - PEAK_CREST
        # level each stem by its loudest ten seconds, so the automation below means the same for all
        r = np.sqrt(np.convolve((x ** 2).mean(1), np.ones(SR) / SR, "valid"))
        x = x * (db(-20) / max(np.percentile(r, 90), 1e-6))
        a = int((at - SCORE_AT) * SR)
        seg = x[max(0, -a): max(0, -a) + n - max(0, a)]
        g = np.ones(len(seg), dtype=np.float32)
        ts = SCORE_AT + (max(0, a) + np.arange(len(seg))) / SR
        if fin:
            u = np.clip((ts - fin[0]) / fin[1], 0, 1)
            g *= np.sin(u * np.pi / 2)
        if fout:
            u = np.clip((ts - fout[0]) / fout[1], 0, 1)
            g *= np.cos(u * np.pi / 2)
        if lift:
            g *= db(np.interp(ts, [k[0] for k in lift], [k[1] for k in lift])).astype(np.float32)
        mix[max(0, a): max(0, a) + len(seg)] += seg * g[:, None]
    # the arc: how far under the voice the strings sit while he talks, and in the pauses (dB on the
    # stems' -20 dBFS); higher as the story darkens; unducked for the held second on seven
    keys = [(30, -16, -10), (46, -14, -7), (58, -13, -6), (72, -11, -5), (86, -10, -4), (97, -9, -4), (106, -8, -3), (end - 1.0, -6, 3), (end, -6, 3)]
    kt = np.array([k[0] for k in keys], dtype=float)
    under = np.interp(t, kt, [k[1] for k in keys])
    clear = np.interp(t, kt, [k[2] for k in keys])
    talk = speaking(words, n)
    lev = smooth_db(clear + (under - clear) * talk, up=0.9, down=0.18)
    mix *= db(lev)[:, None]
    # the last cut: 4 ms to nothing, so it doesn't click
    c = int((end - SCORE_AT) * SR)
    mix[c:] = 0
    mix[max(0, c - int(0.004 * SR)): c] *= np.linspace(1, 0, c - max(0, c - int(0.004 * SR)))[:, None]
    peak = np.abs(mix).max()
    if peak > 0.89:
        mix *= 0.89 / peak
    name = "score.mp3"
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-f", "f32le", "-ac", "2", "-ar", str(SR), "-i", "-", "-c:a", "libmp3lame", "-b:a", "96k", os.path.join(out, name)], input=mix.astype(np.float32).tobytes(), check=True)
    manifest["score"] = {"file": name, "at": SCORE_AT, "end": round(end, 3)}
    return name
