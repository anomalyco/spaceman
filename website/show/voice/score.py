# The film's two non-voice tracks, built from ElevenLabs takes in raw/score (music.ts):
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


def stems(A, end):
    """Where each take sits and how it fades, from the story's anchors (show time, from the tape's words):
    (take, its first sample in show time, fade in (at, secs), fade out (at, secs), lift [(t, dB)]).
    The arc: a brooding cello at the door camera; the neighbours, the hedge, the bins and their pulse and
    rising violins (dread, then urgent, which climbs to Alan); a held breath under the three questions;
    and on the switch to seven a full string stab that keeps climbing, cresting on the cut."""
    return [
        ("cello-1", A["door"] + 1.6, (A["door"] + 1.6, 0.5), (A["hedge"] - 3.0, 4.0), []),
        # dread-1's first bars are faint: lift them so the hedge keeps climbing from the cello
        ("dread-1", A["hedge"] - 2.0, (A["hedge"] - 2.0, 3.0), (A["bins"] + 1.0, 4.0), [(A["hedge"] - 2.0, 6), (A["hedge"] + 12.0, 0)]),
        # urgent-0 climbs for 18 s: its top lands on "I moved Alan in June"
        ("urgent-0", A["mark"] - 17.0, (A["mark"] - 17.0, 3.0), (A["markEnd"] + 0.3, 1.4), [(A["mark"] - 17.0, -3), (A["mark"], 1)]),
        # the held breath: a thin high harmonic, gone just before the switch
        ("hush-0", A["markEnd"] + 0.2, (A["markEnd"] + 0.2, 1.0), (A["seven7"] - 1.4, 1.2), [(0, -9), (999, -9)]),
        # the stab: climax-0 speaks 0.17 s in; then it eases back under the build
        ("climax-0", A["seven7"] - 0.17, None, (A["seven7"] + 2.6, 3.0), [(A["seven7"], 2), (A["seven7"] + 2.0, -2)]),
        # the build: climax-2 crests 11.8 s in, peak-1 21.3 s in: both on the cut
        ("climax-2", end - 11.8, (end - 11.8, 0.6), None, [(end - 11.8, -2), (end - 1.0, 3), (end, 4)]),
        ("peak-1", end - 21.3, (end - 7.0, 4.0), None, [(end - 7.0, -4), (end - 1.0, 2), (end, 3)]),
    ]


def load(path, ch=2):
    pcm = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-f", "f32le", "-ac", str(ch), "-ar", str(SR), "-"], check=True, capture_output=True).stdout
    return np.frombuffer(pcm, dtype=np.float32).reshape(-1, ch).copy()


def db(x):
    return 10 ** (x / 20)


KEY_DB = -12.0  # the play key's peak in the finished tape (dBFS); process.py trims it after loudnorm


def recorder(dest):
    """The recorder's play key: one dry press (key-3, ElevenLabs SFX), from just before it lands to its
    settle, gently low-passed so nothing in it is harsh. No motor: a looped whir under the opening silence
    sounded awful. It goes into the tape after the compressor, so nothing pumps it."""
    x = load(os.path.join(raw, "key-3.mp3"), 1)[:, 0]
    i = int(np.argmax(np.abs(x)))
    x = x[max(0, i - int(0.03 * SR)): max(0, i - int(0.03 * SR)) + int(1.1 * SR)].copy()
    t = np.arange(len(x)) / SR
    # in over 3 ms, out from 0.35 s to 1.0 s: the settle trails away into the room
    x *= np.clip(t / 0.003, 0, 1) * np.clip((1.0 - t) / 0.65, 0, 1) ** 2
    x /= np.abs(x).max()
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-f", "f32le", "-ac", "1", "-ar", str(SR), "-i", "-",
                    "-af", "highpass=f=110,lowpass=f=6000:p=2,lowpass=f=9000", dest], input=x.astype(np.float32).tobytes(), check=True)
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


def bandpass(x, f, octaves):
    """x through ffmpeg's constant-0 dB-peak band-pass, so x - k·bp(x) is a k-deep bell cut at f."""
    pcm = subprocess.run(["ffmpeg", "-v", "error", "-f", "f32le", "-ac", "2", "-ar", str(SR), "-i", "-", "-af", f"bandpass=f={f}:width_type=o:w={octaves}:csg=0",
                          "-f", "f32le", "-ac", "2", "-ar", str(SR), "-"], input=x.astype(np.float32).tobytes(), check=True, capture_output=True).stdout
    y = np.frombuffer(pcm, dtype=np.float32).reshape(-1, 2)
    out = np.zeros_like(x)
    out[: len(y)] = y[: len(x)]
    return out


def build(manifest, words, A, end, out):
    """words: [(t, end)] of everything said; A: the story's anchors (show time); end: the last cut."""
    n = int((end + 0.05 - SCORE_AT) * SR)
    t = SCORE_AT + np.arange(n) / SR
    mix = np.zeros((n, 2), dtype=np.float32)
    for name, at, fin, fout, lift in stems(A, end):
        x = load(os.path.join(raw, name + ".mp3"))
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
    # the arc (dB on the stems' -20 dBFS): (time, under his voice, in the pauses). It climbs act by act,
    # drops to a held breath for the three questions, and steps straight up on the switch to seven.
    s7 = A["seven7"]
    keys = [(30, -16, -10), (A["door"], -15, -9), (A["bought"], -14, -7), (A["hedge"], -13, -6), (A["bins"], -11, -5),
            (A["seven"], -10, -4), (A["mark"], -9, -3), (A["markEnd"] + 0.4, -13, -10), (s7 - 0.06, -13, -10),
            (s7, -6, 1), (end - 1.0, -5, 4), (end, -2, 5)]
    kt = np.array([k[0] for k in keys], dtype=float)
    under = np.interp(t, kt, [k[1] for k in keys])
    clear = np.interp(t, kt, [k[2] for k in keys])
    talk = speaking(words, n)
    # the duck is smoothed (quick down, slow swell back); the arc itself isn't, so the switch is a hit
    depth = smooth_db((clear - under) * talk, up=0.18, down=0.9)
    mix *= db(clear - depth)[:, None]
    # under his voice in the climax, carve 1-3 kHz out of the strings so he stays easy and clear
    carve = smooth_db(talk * np.clip((t - (s7 - 0.5)) / 0.5, 0, 1) * 0.6, up=0.05, down=0.3).astype(np.float32)
    mix -= bandpass(mix, 1800, 1.6) * carve[:, None]
    # soft-limit the crest rather than turning the whole score down for it
    mix = (0.89 * np.tanh(mix / 0.89)).astype(np.float32)
    # the last cut: 4 ms to nothing, so it doesn't click
    c = int((end - SCORE_AT) * SR)
    mix[c:] = 0
    mix[max(0, c - int(0.004 * SR)): c] *= np.linspace(1, 0, c - max(0, c - int(0.004 * SR)))[:, None]
    name = "score.mp3"
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-f", "f32le", "-ac", "2", "-ar", str(SR), "-i", "-", "-c:a", "libmp3lame", "-b:a", "96k", os.path.join(out, name)], input=mix.astype(np.float32).tobytes(), check=True)
    manifest["score"] = {"file": name, "at": SCORE_AT, "end": round(end, 3)}
    return name


YAY_LUFS = -15.0


def yay(manifest, out):
    """The end card's cheer (yay-0: a children's party "Yaaay!"), trimmed to its first sound so it lands on
    the card's frame, faded to nothing, at -15 LUFS: it bursts out of the black's dead silence."""
    x = load(os.path.join(raw, "yay-0.mp3"), 1)[:, 0]
    on = int(np.argmax(np.abs(x) > db(-40)))
    x = x[max(0, on - int(0.005 * SR)):].copy()
    tt = np.arange(len(x)) / SR
    x *= np.clip(tt / 0.004, 0, 1) * np.clip((len(x) / SR - tt) / 0.25, 0, 1)
    name = "sfx-yay.mp3"
    raw_wav = os.path.join(out, ".yay.wav")
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-f", "f32le", "-ac", "1", "-ar", str(SR), "-i", "-", raw_wav], input=x.astype(np.float32).tobytes(), check=True)
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", raw_wav, "-af", f"loudnorm=I={YAY_LUFS}:TP=-1.5:LRA=11", "-ar", "44100", "-ac", "1", "-b:a", "128k", os.path.join(out, name)], check=True)
    os.remove(raw_wav)
    manifest["sfx"]["yay"] = name
    return name
