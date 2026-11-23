# The paper on his desk and wall, drawn as textures for scene.py (study/clues/): the legal pad of
# timings, the sticky note on Mark's drill, the four index cards on the corkboard, the printed labels on
# his binders, the spines of two hardbacks and the elevation drawing framed by the window. Handwriting is
# Caveat and print is IBM Plex Sans Condensed and Plex Mono (SIL Open Font License, google/fonts),
# fetched to a cache on first run. Only the PNGs are committed.
# usage: uv run --with pillow python clues.py
import os, random, urllib.request
from PIL import Image, ImageDraw, ImageFont, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "clues")
os.makedirs(OUT, exist_ok=True)
CACHE = os.path.expanduser("~/.cache/spaceman-show")
os.makedirs(CACHE, exist_ok=True)
GF = "https://github.com/google/fonts/raw/main/ofl/"


def font_file(name, path):
    f = os.path.join(CACHE, name)
    if not os.path.exists(f):
        urllib.request.urlretrieve(GF + path, f)
    return f


FONT = font_file("Caveat.ttf", "caveat/Caveat%5Bwght%5D.ttf")
PLEX = font_file("IBMPlexSansCondensed-SemiBold.ttf", "ibmplexsanscondensed/IBMPlexSansCondensed-SemiBold.ttf")
PLEX_MED = font_file("IBMPlexSansCondensed-Medium.ttf", "ibmplexsanscondensed/IBMPlexSansCondensed-Medium.ttf")
MONO = font_file("IBMPlexMono-Regular.ttf", "ibmplexmono/IBMPlexMono-Regular.ttf")
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


# ── his binders: laser-printed spine labels, the same template on each, slipped into the pockets ──
BINDERS = ["PINECREST HOA", "UTILITIES", "TAXES 2024", "RECEIPTS", "MANUALS", "PROPERTY", "INSURANCE", "MORTGAGE",
           "TAXES 2023", "CAR", "MEDICAL", "WARRANTIES", "BANK", "PENSION", "HOA MINUTES", "PLANNING", "ARCHIVE 2021",
           "ARCHIVE 2020", "ARCHIVE 2019", "GUARANTEES"]


def label(k, title, aged):
    """A 42 × 120 mm label at 10 px/mm: a hairline border, the file's number at the top, its name set
    along the spine, reading bottom to top."""
    W, H = 420, 1200
    im = paper(W, H, (226, 218, 196) if aged else (238, 235, 226), 5)
    d = ImageDraw.Draw(im)
    ink = (30, 31, 36)
    d.rectangle([18, 18, W - 19, H - 19], outline=(70, 72, 78), width=2)
    num = ImageFont.truetype(PLEX_MED, 64)
    s = "%02d" % (k + 1)
    d.text(((W - d.textlength(s, font=num)) / 2, 50), s, font=num, fill=ink)
    d.line([(60, 150), (W - 60, 150)], fill=(70, 72, 78), width=2)
    # the name: as large as the label's width allows, set on its side
    size = 168
    f = ImageFont.truetype(PLEX, size)
    while f.getlength(title) > H - 300 and size > 60:
        size -= 4
        f = ImageFont.truetype(PLEX, size)
    tw = int(f.getlength(title))
    strip = Image.new("RGBA", (tw + 20, size + 40), (0, 0, 0, 0))
    ImageDraw.Draw(strip).text((10, 0), title, font=f, fill=(*ink, 255))
    strip = strip.rotate(90, expand=True)
    im.paste(strip, ((W - strip.width) // 2 + 8, 180 + (H - 210 - strip.height) // 2), strip)
    # toner isn't perfectly black, and the paper's tooth shows through it
    im = Image.blend(im, paper(W, H, (238, 235, 226), 10), 0.08).filter(ImageFilter.GaussianBlur(0.5))
    im.save(os.path.join(OUT, "label%02d.png" % k))


# ── two cloth hardbacks: foil titles on the spine ──
SPINES = [("THE ENGLISH TERRACE", "STEEN", (44, 52, 46), (196, 168, 110)), ("HOUSEHOLD LAW", "PEARCE", (70, 36, 32), (205, 180, 120))]


def spine(k, title, author, cloth, foil):
    W, H = 400, 2600
    im = paper(W, H, cloth, 7)
    d = ImageDraw.Draw(im)
    for y in (110, 124, H - 124, H - 110):
        d.line([(30, y), (W - 30, y)], fill=foil, width=4)
    f = ImageFont.truetype(PLEX, 150)
    strip = Image.new("RGBA", (int(f.getlength(title)) + 20, 190), (0, 0, 0, 0))
    ImageDraw.Draw(strip).text((10, 0), title, font=f, fill=(*foil, 255))
    strip = strip.rotate(-90, expand=True)
    im.paste(strip, ((W - strip.width) // 2, 260), strip)
    fa = ImageFont.truetype(PLEX_MED, 90)
    d.text(((W - d.textlength(author, font=fa)) / 2, H - 320), author, font=fa, fill=foil)
    im = im.filter(ImageFilter.GaussianBlur(0.7))
    im.save(os.path.join(OUT, "spine%d.png" % k))


# ── the print by the window: an architect's elevation of a terrace like his, pencil-fine on cream ──
def elevation():
    W, H = 1500, 1050
    im = paper(W, H, (236, 231, 218), 4)
    d = ImageDraw.Draw(im)
    ln = (88, 86, 82)
    lt = (150, 146, 138)
    g = 980  # the ground line
    d.line([(90, g), (W - 90, g)], fill=ln, width=3)
    houses = 3
    hw = 330
    x0 = (W - houses * hw) // 2
    eave, ridge = 430, 300
    for i in range(houses):
        x = x0 + i * hw
        d.rectangle([x, eave, x + hw, g], outline=ln, width=2)
        # brick coursing, faint
        for y in range(eave + 14, g, 14):
            d.line([(x + 2, y), (x + hw - 2, y)], fill=(212, 206, 194), width=1)
        # the roof, the chimney on the party wall
        d.polygon([(x - 6, eave), (x + hw + 6, eave), (x + hw + 6, ridge), (x - 6, ridge)], outline=ln)
        for y in range(ridge + 10, eave, 10):
            d.line([(x - 6, y), (x + hw + 6, y)], fill=lt, width=1)
        d.rectangle([x + hw - 30, ridge - 70, x + hw + 30, ridge], outline=ln, width=2)
        # two sash windows up, a bay window and a door down
        for wx in (x + 50, x + 190):
            d.rectangle([wx, eave + 60, wx + 90, eave + 220], outline=ln, width=2)
            d.line([(wx, eave + 140), (wx + 90, eave + 140)], fill=ln, width=2)
            d.line([(wx + 45, eave + 60), (wx + 45, eave + 220)], fill=lt, width=1)
            d.rectangle([wx - 8, eave + 220, wx + 98, eave + 232], outline=ln, width=1)
        d.rectangle([x + 40, eave + 330, x + 200, g - 40], outline=ln, width=2)
        for bx in (x + 40, x + 93, x + 146, x + 200):
            d.line([(bx, eave + 330), (bx, g - 40)], fill=ln, width=2)
        d.line([(x + 40, eave + 420), (x + 200, eave + 420)], fill=lt, width=1)
        d.rectangle([x + 235, eave + 300, x + 300, g], outline=ln, width=2)
        d.arc([x + 235, eave + 268, x + 300, eave + 332], 180, 360, fill=ln, width=2)
        d.ellipse([x + 286, eave + 420, x + 292, eave + 426], outline=ln)
    # dimension line under the terrace
    y = g + 30
    d.line([(x0, y), (x0 + houses * hw, y)], fill=lt, width=1)
    for t in range(houses + 1):
        d.line([(x0 + t * hw, y - 8), (x0 + t * hw, y + 8)], fill=lt, width=1)
    mono = ImageFont.truetype(MONO, 22)
    for t, n in enumerate(("No. 10", "No. 12", "No. 14")):
        s = n
        d.text((x0 + t * hw + (hw - d.textlength(s, font=mono)) / 2, y + 10), s, font=mono, fill=ln)
    # the title block
    tb = ImageFont.truetype(PLEX_MED, 26)
    d.text((90, 80), "PINECREST ROAD", font=ImageFont.truetype(PLEX, 40), fill=ln)
    d.text((90, 130), "FRONT ELEVATION  1:100", font=tb, fill=lt)
    im = im.filter(ImageFilter.GaussianBlur(0.6))
    im.save(os.path.join(OUT, "elevation.png"))


legal_pad()
sticky()
for k, t in enumerate(BINDERS):
    label(k, t, k >= 16 or k % 5 == 3)
for k, sp in enumerate(SPINES):
    spine(k, *sp)
elevation()
card("4", [NAMES["4"], "out 07:42  back 18:15", "gym Tue / Thu"])
card("5", [NAMES["5"], "lights off 22:31", "cat out 06:50, in 21:05"])
card("6", [NAMES["6"], "up 06:10", "at his window most nights"])
card("7", [])
print(sorted(os.listdir(OUT)))
