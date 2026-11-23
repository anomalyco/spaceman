# The paper on his desk and wall, drawn as textures for scene.py (study/clues/): the legal pad of
# timings, the sticky note on Mark's drill and the four index cards on the corkboard. Handwriting is Caveat (SIL Open Font License, google/fonts),
# fetched to a cache on first run. Only the PNGs are committed.
# usage: uv run --with pillow python clues.py
import os, random, urllib.request
from PIL import Image, ImageDraw, ImageFont, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "clues")
os.makedirs(OUT, exist_ok=True)
CACHE = os.path.expanduser("~/.cache/spaceman-show")
os.makedirs(CACHE, exist_ok=True)
FONT = os.path.join(CACHE, "Caveat.ttf")
if not os.path.exists(FONT):
    urllib.request.urlretrieve("https://github.com/google/fonts/raw/main/ofl/caveat/Caveat%5Bwght%5D.ttf", FONT)
rng = random.Random(12)
# the neighbours on four, five and six (the film's names; one place to change them)
NAMES = dict(zip("456", os.environ.get("NEIGHBOURS", "Alan,Juliana,Dax").split(",")))


def hand(size, weight=500):
    f = ImageFont.truetype(FONT, size)
    try:
        f.set_variation_by_axes([weight])
    except Exception:
        pass
    return f


def ink_text(d, xy, s, font, fill, jitter=1.2):
    """Handwriting, a letter at a time, each a hair off the line, so it isn't type."""
    x, y = xy
    for ch in s:
        d.text((x + rng.uniform(-jitter, jitter) * 0.3, y + rng.uniform(-jitter, jitter)), ch, font=font, fill=fill)
        x += d.textlength(ch, font=font) * rng.uniform(0.97, 1.02)
    return x


def paper(w, h, base, grain=6):
    im = Image.new("RGB", (w, h), base)
    px = im.load()
    for _ in range(w * h // 40):
        x, y = rng.randrange(w), rng.randrange(h)
        v = rng.randint(-grain, grain)
        r, g, b = px[x, y]
        px[x, y] = (max(0, min(255, r + v)), max(0, min(255, g + v)), max(0, min(255, b + v)))
    return im.filter(ImageFilter.GaussianBlur(0.6))


# ── the legal pad: what each house does, and when, in his tidiest hand ──
def legal_pad():
    W, H = 1080, 1780
    im = paper(W, H, (246, 228, 128))
    d = ImageDraw.Draw(im)
    top = 150
    step = 46
    for y in range(top, H - 20, step):
        d.line([(0, y), (W, y)], fill=(120, 165, 205), width=2)
    for x in (150, 158):
        d.line([(x, 0), (x, H)], fill=(214, 96, 92), width=2)
    ink = (28, 40, 92)
    f = hand(46, 520)
    fh = hand(52, 700)
    ink_text(d, (175, top - 66), "No. 10 / 16 / 14 \u2014 wk 47", fh, ink)
    rows = [
        ("22:31", "WED", "5", "lights off"), ("22:40", "WED", "6", "at window"), ("23:02", "WED", "4", "car in"),
        ("06:10", "THU", "6", "up"), ("06:12", "THU", "6", "curtains"), ("06:50", "THU", "5", "cat out"),
        ("07:42", "THU", "4", "out"), ("18:15", "THU", "4", "back"), ("19:00", "THU", "4", "gym"),
        ("21:05", "THU", "5", "cat in"), ("22:31", "THU", "5", "lights off"), ("23:00", "THU", "6", "at window"),
        ("06:10", "FRI", "6", "up"), ("06:50", "FRI", "5", "cat out"), ("07:42", "FRI", "4", "out"),
        ("14:00", "SUN", "5", "daughter"), ("16:10", "SUN", "5", "daughter gone"), ("22:31", "SUN", "5", "lights off"),
        ("23:00", "SUN", "6", "at window"), ("23:01", "SUN", "6", "looks across"),
    ]
    for i, (t, day, n, what) in enumerate(rows):
        y = top + i * step + 4
        ink_text(d, (175, y), t, f, ink)
        ink_text(d, (330, y), day, f, ink)
        ink_text(d, (450, y), n, f, ink)
        ink_text(d, (510, y), what, f, ink)
    im.save(os.path.join(OUT, "legalpad.png"))


def sticky():
    W = 600
    im = paper(W, W, (247, 222, 92), 8)
    d = ImageDraw.Draw(im)
    ink = (20, 20, 24)
    ink_text(d, (70, 120), NAMES["4"].upper(), hand(190 if len(NAMES["4"]) <= 4 else 150, 700), ink, 2)
    ink_text(d, (80, 360), "No. 10", hand(110, 500), ink, 2)
    d.line([(78, 330), (470, 318)], fill=ink, width=7)
    im.save(os.path.join(OUT, "sticky.png"))


def card(n, lines):
    W, H = 900, 540
    im = paper(W, H, (246, 244, 236), 5)
    d = ImageDraw.Draw(im)
    d.line([(0, 96), (W, 96)], fill=(214, 96, 92), width=3)
    for y in range(150, H, 54):
        d.line([(0, y), (W, y)], fill=(150, 185, 215), width=2)
    ink = (24, 28, 60)
    ink_text(d, (40, 6), n, hand(96, 700), ink, 1.5)
    for i, s in enumerate(lines):
        ink_text(d, (48, 108 + i * 54), s, hand(46, 500), ink)
    im.save(os.path.join(OUT, "card%s.png" % n))


legal_pad()
sticky()
card("4", [NAMES["4"], "out 07:42  back 18:15", "gym Tue / Thu"])
card("5", [NAMES["5"], "lights off 22:31", "cat out 06:50, in 21:05"])
card("6", [NAMES["6"], "up 06:10", "at his window most nights"])
card("7", [])
print(sorted(os.listdir(OUT)))
