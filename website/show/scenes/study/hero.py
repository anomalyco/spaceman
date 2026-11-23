# Cycles renders of the study for the film's opening: the target stills, and the plate (the move from
# the asleep room into the screen as an image sequence, with the panel's corners per frame for the live
# desktop to sit on).
# usage: Blender -b --factory-startup -noaudio -P hero.py -- out=<dir> [cam=<name>] [w=1440 h=900 spp=128]
#          test                 one low still of every candidate camera, asleep and awake
#          at=0,4,6.8           stills at those film times (seconds)
#          frames=0-221         the plate's frames (30 fps from film t=0), PNG, plus corners.json
#          close=binders,wall,drill  close-ups of the room's details, the screen asleep (not in the film)
#          [ev=0 street=1 landing=1 lamp=1 sky=1 spill=1 fill=1 screen=1 blur=1]
import os, sys, math, json
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
sys.path.insert(0, HERE)
import bpy
from mathutils import Vector
from bpy_extras.object_utils import world_to_camera_view
from lib import *

A = args()
KV = dict(a.split("=", 1) for a in A if "=" in a)
num = lambda k, d: float(KV.get(k, d))
OUT = KV.get("out", os.path.join(HERE, "renders"))
W, H = int(num("w", 1440)), int(num("h", 900))
SPP = int(num("spp", 128))
os.makedirs(OUT, exist_ok=True)

sc = reset()
gpu(sc, SPP)
sc.view_settings.exposure = num("ev", 0)
sc.cycles.use_denoising = True
sc.cycles.denoiser = "OPENIMAGEDENOISE"
sc.cycles.max_bounces = 10
sc.cycles.diffuse_bounces = 6
sc.cycles.glossy_bounces = 4
sc.cycles.transparent_max_bounces = 64
sc.cycles.sample_clamp_indirect = 6.0
sc.cycles.light_sampling_threshold = 0.005
sc.render.fps = 30
sc.render.film_transparent = False

import scene as S

S.SLAT = num("slat", S.SLAT)

built = S.build()
M, L, scr = built["M"], built["lights"], built["screen"]
L["street"].data.energy *= num("street", 1)
L["landing"].data.energy *= num("landing", 1)
L["lamp"].data.energy *= num("lamp", 1)
L["sky"].data.energy *= num("sky", 1)
L["spill"].data.energy *= num("spill", 1)
L["fill"].data.energy *= num("fill", 1)
SCREEN = 1.4 * num("screen", 1)

# ── cameras ─────────────────────────────────────────────────────────────
# the opening's candidates (pos, look-at, vfov, f-stop); the film's opening is CAM
CANDS = {
    "chair": ((2.16, 1.70, 1.24), (1.98, 2.93, 0.98), 44, 2.8),
    "door": ((0.85, 0.42, 1.50), (2.30, 2.80, 0.92), 42, 2.0),
    "corner": ((0.45, 0.95, 1.38), (2.30, 2.90, 0.95), 40, 2.8),
    "back": ((1.55, 0.55, 1.42), (2.05, 2.93, 0.98), 36, 2.8),
}
CAM = KV.get("cam", "door")


def end_shot():
    """The camera that sees the panel exactly as the flat desktop is drawn as the film hands over
    (main.js screenShot() at 16:10: the panel fills the frame at the handoff's 30° field of view)."""
    c = scr.matrix_world.translation.copy()
    n = Vector((0, -1, 0))
    d = (S.PH / 2) / math.tan(math.radians(15))
    return c + n * d, -n, 30.0, d


def opening():
    p, t, v, f = CANDS[CAM]
    p, t = Vector(p), Vector(t)
    # focus on the monitor's face
    mid = scr.matrix_world.translation
    d = (t - p).normalized()
    return p, d, v, (mid - p).dot(d), f


PAD = Vector((1.53, 2.66, 0.75))  # the legal pad in the lamp's pool, where the eye starts


def rack(t, at_mon):
    """In the dark the lens rests on the lit pad; as the screen wakes it pulls focus onto the panel."""
    p, t_, *_ = CANDS[CAM]
    p = Vector(p)
    d = (Vector(t_) - p).normalized()
    pad = (PAD - p).dot(d)
    k = smooth((t - 0.5) / 0.9)
    return 1 / ((1 - k) / pad + k / at_mon)


def smooth(x):
    x = min(1, max(0, x))
    return x * x * (3 - 2 * x)


def ease_io(x):
    x = min(1, max(0, x))
    return 4 * x ** 3 if x < 0.5 else 1 - (-2 * x + 2) ** 3 / 2


T0, T1 = 1.2, 6.682 + 0.7  # the move: main.js director, move(1.2, intro, ...)


def pose(t):
    """Film time → (pos, dir, vfov, focus distance, f-stop, glow): main.js camera3() onto "screen"."""
    ap, ad, av, af, fs = opening()
    bp, bd, bv, bf = end_shot()
    glow = smooth((t - 0.42) / 0.53)
    u = ease_io((t - T0) / (T1 - T0)) ** 1.15 if t > T0 else 0.0
    pos = ap.lerp(bp, u)
    w = smooth(u / 0.75)
    look = (ap + ad * af).lerp(bp + bd * bf, w)
    d = (look - pos).normalized()
    vfov = av + (bv - av) * u
    on = 1 / max(0.05, (scr.matrix_world.translation - pos).dot(d))
    k = smooth(u / 0.4)
    focus = 1 / ((1 - k) / rack(t, af) + k * on)
    return pos, d, vfov, focus, fs, glow


cd = D.cameras.new("Plate")
cd.sensor_fit = "VERTICAL"
cd.sensor_height = 24.0
cd.clip_start = 0.02
cd.clip_end = 100
cd.dof.use_dof = True
cam = D.objects.new("Plate", cd)
sc.collection.objects.link(cam)
sc.camera = cam


def place(t, key=None):
    pos, d, vfov, focus, fs, glow = pose(t)
    cam.location = pos
    cam.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    cd.lens = 12.0 / math.tan(math.radians(vfov) / 2)
    cd.dof.focus_distance = focus
    cd.dof.aperture_fstop = fs
    S.set_screen(M, SCREEN * glow)
    bpy.context.view_layer.update()
    print("CAM", t, tuple(round(x, 3) for x in cam.matrix_world.translation), round(cd.lens, 2))
    if key is not None:
        cam.keyframe_insert("location", frame=key)
        cam.keyframe_insert("rotation_euler", frame=key)
        cd.keyframe_insert("lens", frame=key)
    return glow


def corners():
    bpy.context.view_layer.update()
    out = []
    for p in S.panel_corners(scr):
        v = world_to_camera_view(sc, cam, p)
        out.append([round(v.x, 5), round(1 - v.y, 5)])
    return out  # [bl, br, tr, tl], 0..1 from the top-left


def corners_at(t):
    pos, d, *_ = pose(t)
    return [round(x, 5) for x in (*pos, *d)]


def still(path, w, h, spp):
    sc.render.resolution_x, sc.render.resolution_y, sc.render.resolution_percentage = w, h, 100
    sc.cycles.samples = spp
    sc.render.image_settings.file_format = "PNG"
    # 16-bit for the plate: 8-bit bands in the dark gradients before the encoder ever sees them
    sc.render.image_settings.color_depth = KV.get("depth", "8")
    sc.render.image_settings.compression = 15
    sc.render.filepath = path
    bpy.ops.render.render(write_still=True)


for o in D.objects:
    if any(o.name.startswith(p) for p in KV.get("hide", "").split(",") if p):
        o.hide_render = True

if "rays" in A:
    dg = bpy.context.evaluated_depsgraph_get()
    src = L["street"].matrix_world.translation
    print("SPOT", tuple(src), tuple(L["street"].matrix_world.to_3x3() @ Vector((0, 0, -1))), L["street"].data.energy, math.degrees(L["street"].data.spot_size))
    for y in (1.9, 2.1, 2.3, 2.5, 2.7):
        for z in (1.0, 1.4, 1.8):
            p = src
            hits = []
            d = (Vector((S.RW, y, z)) - src).normalized()
            for _ in range(12):
                ok, loc, nrm, idx, ob, mtx = sc.ray_cast(dg, p + d * 1e-4, d)
                if not ok:
                    break
                hits.append(ob.name)
                p = loc
            print("RAY", y, z, hits)
    sys.exit(0)

# the first render after building the scene comes out wrong (a flat colour); a tiny one absorbs it
place(0)
still(os.path.join(OUT, ".warmup.png"), 16, 10, 1)

if "test" in A:
    for name in KV.get("cams", ",".join(CANDS)).split(","):
        CAM = name
        for t, tag in ((0.0, "asleep"), (1.0, "awake")):
            place(t)
            still(os.path.join(OUT, "%s-%s.png" % (name, tag)), W, H, SPP)

if "at" in KV:
    for t in [float(x) for x in KV["at"].split(",")]:
        place(t)
        still(os.path.join(OUT, "%s-t%05.2f.png" % (CAM, t)), W, H, SPP)
        print("CORNERS", t, json.dumps(corners()))

# close-ups of the details (pos, look-at, vfov, f-stop), for checking them; not in the film
CLOSE = {
    "binders": ((0.72, 2.05, 1.32), (0.80, 2.88, 1.18), 38, 4.0),
    "wall": ((2.35, 1.75, 1.45), (3.12, 3.2, 1.45), 46, 5.6),
    "drill": ((2.35, 2.35, 1.05), (2.64, 3.08, 0.92), 30, 4.0),
}
if "close" in KV:
    S.set_screen(M, 0)
    for name in KV["close"].split(","):
        p, t, v, f = CLOSE[name]
        p, t = Vector(p), Vector(t)
        cam.location = p
        cam.rotation_euler = (t - p).to_track_quat("-Z", "Y").to_euler()
        cd.lens = 12.0 / math.tan(math.radians(v) / 2)
        cd.dof.focus_distance = (t - p).length
        cd.dof.aperture_fstop = f
        still(os.path.join(OUT, "close-%s.png" % name), W, H, SPP)

if "frames" in KV:
    a, b = [int(x) for x in KV["frames"].split("-")]
    # the live desktop covers the panel in the plate, so the panel shows a blur of it: its light
    # still falls on the room, and nothing sharp can ghost through the overlay while it fades up
    soft = os.path.join(HERE, "desktop-soft.jpg")
    if os.path.exists(soft):
        M["screen"].node_tree.nodes["desktop"].image = D.images.load(soft, check_existing=True)
    sc.render.use_motion_blur = num("blur", 1) > 0
    sc.render.motion_blur_shutter = 0.5
    # keys every frame, linear between, so the shutter sees the move between frames
    bpy.context.preferences.edit.keyframe_new_interpolation_type = "LINEAR"
    for f in range(max(0, a - 1), b + 2):
        place(f / 30, key=f)
    meta = {}
    prev = None
    import shutil
    for f in range(a, b + 1):
        sc.frame_set(f)
        meta[f] = {"glow": round(place(f / 30), 4), "corners": corners()}
        place(f / 30, key=f)
        path = os.path.join(OUT, "f%04d.png" % f)
        # a held frame (the camera still, the glow unchanged, either side) is the frame before it again
        key = (json.dumps(meta[f]), json.dumps(corners_at((f - 1) / 30)), json.dumps(corners_at((f + 1) / 30)))
        if prev and prev[0] == key and os.path.exists(prev[1]) and not os.path.exists(path):
            shutil.copyfile(prev[1], path)
        elif not os.path.exists(path):
            sc.frame_set(f)
            place(f / 30)
            still(path, W, H, SPP)
        prev = (key, path)
    with open(os.path.join(OUT, "corners-%d-%d.json" % (a, b)), "w") as fp:
        json.dump(meta, fp)
