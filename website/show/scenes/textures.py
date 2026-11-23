# CC0 PBR textures from Poly Haven, fetched on first use into show/scenes/textures/ (not committed).
# Every texture used is listed, with its author and licence, in textures/LICENCES.md.
# Runs under Blender's Python or plain python3: `python3 textures.py` fetches them all.
import json, os, urllib.request

DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "textures")
RES = "2k"
# id -> the maps we use
SETS = {
    "wood_floor": ("diff", "rough", "disp"),
    "painted_plaster_wall": ("diff", "rough", "disp"),
    "plastered_wall_04": ("diff", "rough", "disp"),
    "oak_veneer_01": ("diff", "rough", "disp"),
    "poly_wool_herringbone": ("diff", "rough", "disp"),
    "knitted_fleece": ("diff", "rough", "disp"),
    "rough_linen": ("diff", "rough", "disp"),
}
KEYS = {"diff": "Diffuse", "rough": "Rough", "disp": "Displacement"}


def path(tid, kind):
    return os.path.join(DIR, "%s_%s_%s.jpg" % (tid, kind, RES))


def fetch(tid):
    os.makedirs(DIR, exist_ok=True)
    want = [k for k in SETS[tid] if not os.path.exists(path(tid, k))]
    if not want:
        return
    req = urllib.request.Request("https://api.polyhaven.com/files/" + tid, headers={"User-Agent": "spaceman-scenes"})
    files = json.load(urllib.request.urlopen(req, timeout=60))
    for k in want:
        url = files[KEYS[k]][RES]["jpg"]["url"]
        r = urllib.request.Request(url, headers={"User-Agent": "spaceman-scenes"})
        data = urllib.request.urlopen(r, timeout=120).read()
        with open(path(tid, k) + ".part", "wb") as f:
            f.write(data)
        os.replace(path(tid, k) + ".part", path(tid, k))
        print("texture", tid, k, len(data))


def ensure(tid):
    fetch(tid)
    return {k: path(tid, k) for k in SETS[tid]}


if __name__ == "__main__":
    for t in SETS:
        fetch(t)
