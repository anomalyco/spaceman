# The study at No. 12, upstairs at the front, at night; and No. 14 across the street.
# Builds the set, bakes its light into atlases (the screen's light separately), exports
# public/show/3d/study.glb + study.json, and renders emission previews of what the browser draws.
# usage: Blender -b --factory-startup -P build.py -- [quick] [previews] [out=<dir>]
import random
import os, sys, math, json
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import bpy
from mathutils import Vector
from lib import *
import street as street_mod

A = args()
QUICK = "quick" in A
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = next((a[4:] for a in A if a.startswith("out=")), os.path.join(HERE, "..", "..", "..", "public", "show", "3d"))
PREV = os.path.join(HERE, "previews")
os.makedirs(OUT, exist_ok=True)
os.makedirs(PREV, exist_ok=True)

sc = reset()
gpu(sc)
sc.render.resolution_x, sc.render.resolution_y = 1600, 1000

RW, RD, RH, T = 3.6, 3.2, 2.5, 0.12
G = -3.0  # the street, a storey below the study floor
WY0, WY1, WZ0, WZ1 = 1.70, 2.95, 0.90, 2.15  # the window opening in the east wall
pi = math.pi
rad = math.radians

NEAR, ROOM, STREET, LIT = [], [], [], []

# ── materials ──
plaster = noisy(mat("plaster", rough=0.9), (0.47, 0.455, 0.43), (0.53, 0.515, 0.49), scale=3.0, rough=(0.85, 0.95), bump=0.04)
ceiling = noisy(mat("ceiling", rough=0.9), (0.62, 0.615, 0.6), (0.66, 0.655, 0.64), scale=2.0)
paint = noisy(mat("paint", rough=0.35), (0.64, 0.63, 0.6), (0.67, 0.66, 0.63), scale=20)
floorm = boards("floor", base=(0.33, 0.21, 0.12), dark=(0.19, 0.11, 0.06))
oak = wood("oak", base=(0.52, 0.36, 0.21), dark=(0.33, 0.2, 0.1))
steel = mat("steel", (0.05, 0.05, 0.055), 0.45)
alu = noisy(mat("alu", rough=0.3), (0.5, 0.505, 0.52), (0.54, 0.545, 0.56), scale=25, detail=2, rough=(0.25, 0.35))
body = mat("monitor", (0.025, 0.025, 0.028), 0.3)
keycap = mat("keys", (0.04, 0.04, 0.045), 0.5)
fabric = noisy(mat("fabric", rough=1.0), (0.05, 0.052, 0.058), (0.075, 0.078, 0.085), scale=400, detail=2, bump=0.2)
rugm = noisy(mat("rug", rough=1.0), (0.2, 0.2, 0.19), (0.25, 0.245, 0.235), scale=300, detail=3, bump=0.3)
binder = noisy(mat("binder", rough=0.55), (0.12, 0.15, 0.19), (0.135, 0.165, 0.205), scale=60)
label = mat("label", (0.72, 0.72, 0.7), 0.6)
archive = noisy(mat("archive", rough=0.9), (0.42, 0.4, 0.37), (0.46, 0.44, 0.41), scale=40)
notebook = mat("notebook", (0.09, 0.13, 0.11), 0.6)
pen = mat("pen", (0.02, 0.02, 0.02), 0.3)
slat = mat("slat", (0.62, 0.62, 0.6), 0.4)
cord = mat("cord", (0.55, 0.54, 0.5), 0.9)
glassm = mat("Glass", (0.7, 0.78, 0.82), 0.05)
screenm = mat("Screen", (0.004, 0.004, 0.005), 0.1)
# outside: the street (show/scenes/street.py)
SM = street_mod.materials()

# ── the room ──
def walls():
    R = ROOM
    R.append(box("floor", (RW / 2, RD / 2, -0.05), (RW + 2 * T, RD + 2 * T, 0.1), floorm, 0))
    R.append(box("ceiling", (RW / 2, RD / 2, RH + 0.05), (RW + 2 * T, RD + 2 * T, 0.1), ceiling, 0))
    R.append(box("wallN", (RW / 2, RD + T / 2, RH / 2), (RW + 2 * T, T, RH), plaster, 0))
    R.append(box("wallW", (-T / 2, RD / 2, RH / 2), (T, RD, RH), plaster, 0))
    # south wall with the doorway (x 0.3..1.15)
    R.append(box("wallS1", (-T / 2 + (0.3 + T) / 2, -T / 2, RH / 2), (0.3 + T, T, RH), plaster, 0))
    R.append(box("wallS2", ((1.15 + RW + T) / 2, -T / 2, RH / 2), (RW + T - 1.15, T, RH), plaster, 0))
    R.append(box("wallS3", ((0.3 + 1.15) / 2, -T / 2, (2.1 + RH) / 2), (0.85, T, RH - 2.1), plaster, 0))
    # east wall around the window
    ex = RW + T / 2
    R.append(box("wallE1", (ex, RD / 2, WZ0 / 2), (T, RD + 2 * T, WZ0), plaster, 0))
    R.append(box("wallE2", (ex, RD / 2, (WZ1 + RH) / 2), (T, RD + 2 * T, RH - WZ1), plaster, 0))
    R.append(box("wallE3", (ex, WY0 / 2 - T / 2, (WZ0 + WZ1) / 2), (T, WY0 + T, WZ1 - WZ0), plaster, 0))
    R.append(box("wallE4", (ex, (WY1 + RD + T) / 2, (WZ0 + WZ1) / 2), (T, RD + T - WY1, WZ1 - WZ0), plaster, 0))
    # skirting
    for n, c, s in [("skN", (RW / 2, RD - 0.006, 0.045), (RW, 0.012, 0.09)), ("skW", (0.006, RD / 2, 0.045), (0.012, RD, 0.09)),
                    ("skE", (RW - 0.006, RD / 2, 0.045), (0.012, RD, 0.09)), ("skS", ((1.15 + RW) / 2, 0.006, 0.045), (RW - 1.15, 0.012, 0.09))]:
        R.append(box(n, c, s, paint, 0.002))
    # door frame
    for n, c, s in [("dfL", (0.3 + 0.02, 0.0, 1.05), (0.04, 0.16, 2.1)), ("dfR", (1.15 - 0.02, 0.0, 1.05), (0.04, 0.16, 2.1)), ("dfT", (0.725, 0.0, 2.1 - 0.02), (0.85, 0.16, 0.04))]:
        R.append(box(n, c, s, paint, 0.003))
    # window frame, sill, radiator
    fx = RW + 0.05
    yc, zc = (WY0 + WY1) / 2, (WZ0 + WZ1) / 2
    for n, c, s in [("wfL", (fx, WY0 + 0.025, zc), (0.06, 0.05, WZ1 - WZ0)), ("wfR", (fx, WY1 - 0.025, zc), (0.06, 0.05, WZ1 - WZ0)),
                    ("wfT", (fx, yc, WZ1 - 0.025), (0.06, WY1 - WY0, 0.05)), ("wfB", (fx, yc, WZ0 + 0.025), (0.06, WY1 - WY0, 0.05)),
                    ("wfM", (fx, yc, zc + 0.2), (0.05, WY1 - WY0, 0.035))]:
        R.append(box(n, c, s, paint, 0.004))
    R.append(box("sill", (RW - 0.02, yc, WZ0 - 0.012), (0.2, WY1 - WY0 + 0.12, 0.025), paint, 0.004))
    rad_ = box("radiator", (RW - 0.07, yc, 0.42), (0.05, 1.0, 0.5), paint, 0.006)
    R.append(rad_)
    for i in range(24):
        R.append(box("fin%d" % i, (RW - 0.1, yc - 0.48 + i * 0.0417, 0.42), (0.012, 0.022, 0.48), paint, 0.003))
    # venetian blinds, lowered to just below the transom, slats tilted
    bx = RW - 0.035
    R.append(box("headrail", (bx, yc, WZ1 - 0.03), (0.04, WY1 - WY0 - 0.06, 0.035), slat, 0.004))
    z = WZ1 - 0.07
    i = 0
    while z > 1.24:
        R.append(box("slat%d" % i, (bx, yc, z), (0.025, WY1 - WY0 - 0.08, 0.0012), slat, 0.0, rot=(0, rad(22), 0)))
        z -= 0.021
        i += 1
    R.append(box("bottomrail", (bx, yc, z), (0.03, WY1 - WY0 - 0.08, 0.008), slat, 0.002))
    for k in (-0.4, 0.0, 0.4):
        R.append(box("ladder%d" % (k * 10), (bx + 0.013, yc + k, (z + WZ1 - 0.05) / 2), (0.0015, 0.003, WZ1 - 0.05 - z), cord, 0))
    # ceiling light (off)
    R.append(cyl("ceilingLight", (RW / 2, RD / 2, RH - 0.03), 0.17, 0.06, paint, 48))
    # rug, squared to the desk
    R.append(box("rug", (2.0, RD - 1.05, 0.005), (1.7, 1.25, 0.01), rugm, 0.004))
    # the bookcase, binders by month, archive boxes
    sx, sy = 0.66, RD - 0.17
    for n, c, s in [("bcL", (sx - 0.4, sy, 0.95), (0.022, 0.32, 1.9)), ("bcR", (sx + 0.4, sy, 0.95), (0.022, 0.32, 1.9)),
                    ("bcBack", (sx, sy + 0.155, 0.95), (0.8, 0.01, 1.9))]:
        R.append(box(n, c, s, paint, 0.003))
    for k, hz in enumerate([0.08, 0.45, 0.82, 1.19, 1.56, 1.89]):
        R.append(box("bcS%d" % k, (sx, sy, hz), (0.78, 0.3, 0.022), paint, 0.003))
    # orderly, but used: the older row's spines a little faded and their labels yellowed, a few pulled
    # proud of the rest, some fatter than others, each label stuck on by hand
    rng = random.Random(14)
    faded = noisy(mat("binderOld", rough=0.6), (0.14, 0.165, 0.19), (0.155, 0.18, 0.205), scale=60)
    old_label = mat("labelOld", (0.7, 0.66, 0.55), 0.6)
    for row, hz in enumerate([0.831, 1.201]):
        x = sx - 0.36 + 0.004
        for i in range(11):
            wdt = rng.choice((0.058, 0.058, 0.052, 0.066))
            if x + wdt > sx + 0.37:
                break
            xc = x + wdt / 2
            dy = rng.choice((0.0, 0.0, 0.0, -0.012, -0.022, 0.006))
            R.append(box("binder%d_%d" % (row, i), (xc, sy + 0.005 + dy, hz + 0.011 + 0.155), (wdt - 0.003, 0.28, 0.31), faded if row == 0 else binder, 0.003))
            R.append(box("label%d_%d" % (row, i), (xc + rng.uniform(-0.002, 0.002), sy - 0.136 + dy, hz + 0.011 + 0.2 + rng.uniform(-0.006, 0.006)),
                         (0.034, 0.002, 0.07), old_label if row == 0 else label, 0.0, rot=(0, rad(rng.uniform(-2.5, 2.5)), 0)))
            x += wdt + 0.008
    for i in range(3):
        R.append(box("arch%d" % i, (sx - 0.25 + i * 0.25, sy, 1.56 + 0.011 + 0.13), (0.24, 0.28, 0.26), archive, 0.004))
    for i in range(2):
        R.append(box("arch_low%d" % i, (sx - 0.19 + i * 0.38, sy, 0.08 + 0.011 + 0.16), (0.36, 0.28, 0.32), archive, 0.004))


def desk():
    N = NEAR
    dx, dy, top = 2.0, RD - 0.4, 0.74
    N.append(box("desktop", (dx, dy, top - 0.015), (1.6, 0.75, 0.03), oak, 0.004))
    for lx in (dx - 0.76, dx + 0.76):
        for ly in (dy - 0.33, dy + 0.33):
            N.append(box("leg", (lx, ly, (top - 0.03) / 2), (0.03, 0.03, top - 0.03), steel, 0.003))
        N.append(box("rail", (lx, dy, top - 0.06), (0.03, 0.66, 0.03), steel, 0.003))
    N.append(box("railB", (dx, dy + 0.33, top - 0.06), (1.52, 0.03, 0.03), steel, 0.003))
    # monitor: body, panel, stand
    mx, my, mz = dx, RD - 0.27, 1.07
    N.append(box("monBody", (mx, my, mz), (0.646, 0.022, 0.413), body, 0.005, 3))
    N.append(box("monBack", (mx, my + 0.02, mz - 0.02), (0.3, 0.025, 0.2), alu, 0.01, 3))
    N.append(box("neck", (mx, my + 0.05, top + 0.17), (0.07, 0.016, 0.34), alu, 0.004))
    N.append(box("standFoot", (mx, my + 0.0, top + 0.005), (0.24, 0.2, 0.01), alu, 0.004))
    N.append(box("neckBend", (mx, my + 0.05, top + 0.012), (0.07, 0.03, 0.02), alu, 0.004))
    # keyboard, trackpad
    kx, ky = dx - 0.04, RD - 0.62
    N.append(box("kb", (kx, ky, top + 0.004), (0.28, 0.115, 0.008), alu, 0.002))
    for r in range(5):
        for c in range(13):
            N.append(box("key", (kx - 0.12 + c * 0.0198, ky - 0.04 + r * 0.02, top + 0.009), (0.016, 0.016, 0.003), keycap, 0.001, 1))
    N.append(box("trackpad", (dx + 0.24, ky, top + 0.003), (0.13, 0.11, 0.006), alu, 0.002))
    # three pens, parallel; a notebook squared to the edge; the label maker
    for i in range(3):
        N.append(cyl("pen%d" % i, (dx + 0.52 + i * 0.022, RD - 0.58, top + 0.005), 0.0045, 0.14, pen, 16, rot=(pi / 2, 0, 0)))
    N.append(box("notebook", (dx + 0.6, RD - 0.3, top + 0.007), (0.15, 0.21, 0.014), notebook, 0.002))
    N.append(box("labelmaker", (dx - 0.52, RD - 0.58, top + 0.018), (0.065, 0.13, 0.036), steel, 0.006, 3))
    # desk lamp, off
    lx, ly = dx - 0.62, RD - 0.16
    N.append(cyl("lampBase", (lx, ly, top + 0.008), 0.07, 0.016, steel, 48, bevel=0.003))
    N.append(cyl("lampArm1", (lx, ly + 0.03, top + 0.22), 0.006, 0.42, steel, 12, rot=(rad(-12), 0, 0)))
    N.append(cyl("lampArm2", (lx, ly - 0.07, top + 0.47), 0.006, 0.3, steel, 12, rot=(rad(62), 0, 0)))
    N.append(cyl("lampHead", (lx, ly - 0.2, top + 0.5), 0.05, 0.12, steel, 32, rot=(rad(30), 0, 0), r2=0.022))
    # the chair, squared to the desk
    cx, cy = dx, RD - 1.08
    N.append(box("seat", (cx, cy, 0.47), (0.5, 0.47, 0.08), fabric, 0.03, 3))
    N.append(box("back", (cx, cy - 0.26, 0.86), (0.46, 0.06, 0.52), fabric, 0.03, 3, rot=(rad(-8), 0, 0)))
    N.append(box("backFrame", (cx, cy - 0.29, 0.66), (0.06, 0.03, 0.4), steel, 0.01, 2, rot=(rad(-10), 0, 0)))
    for s in (-1, 1):
        N.append(box("armPost", (cx + s * 0.27, cy - 0.02, 0.56), (0.03, 0.04, 0.2), steel, 0.008))
        N.append(box("armPad", (cx + s * 0.27, cy + 0.0, 0.665), (0.06, 0.24, 0.025), fabric, 0.01))
    N.append(box("seatPan", (cx, cy, 0.42), (0.22, 0.22, 0.04), steel, 0.008))
    N.append(cyl("gas", (cx, cy, 0.26), 0.024, 0.3, alu, 24))
    for k in range(5):
        a = rad(90 + k * 72)
        N.append(box("leg%d" % k, (cx + math.cos(a) * 0.16, cy + math.sin(a) * 0.16, 0.085), (0.31, 0.04, 0.03), steel, 0.01, 2, rot=(0, rad(-4), a)))
        N.append(cyl("caster%d" % k, (cx + math.cos(a) * 0.31, cy + math.sin(a) * 0.31, 0.03), 0.028, 0.025, steel, 20, rot=(pi / 2, 0, a)))


def street():
    parts, lamp = street_mod.build(G, SM)
    STREET.extend(parts["street"])
    LIT.extend(parts["lit"])
    return lamp


walls()
desk()
lit14 = street()
# No. 14's front (the house, its hedge, its door): what CamWindow looks at, baked on its own atlas
FRONT = [o for o in STREET if 19.0 < o.location.x < 34.0 and -5.0 < o.location.y < 9.5]
STREET[:] = [o for o in STREET if o not in FRONT]

# not baked: the screen, the glass, No. 14's lit window
mx, my, mz = 2.0, RD - 0.27, 1.07
PW, PH = 0.62, 0.3875
bm = bmesh.new()
bm.loops.layers.uv.new("UVMap")
bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=0.5, calc_uvs=True)
for v in bm.verts:
    v.co = Vector((v.co.x * PW, v.co.y * PH, 0))
screen = obj_from_bm("Screen", bm, screenm)
screen.location = (mx, my - 0.0115, mz + 0.004)
screen.rotation_euler = (pi / 2, 0, 0)
glass = box("Glass", (RW + 0.08, (WY0 + WY1) / 2, (WZ0 + WZ1) / 2), (0.006, WY1 - WY0, WZ1 - WZ0), glassm, 0)

# ── light ──
bg, street_lights = street_mod.night(G)
lights_base = street_lights + [
    light("hall", "AREA", (0.72, -0.7, 2.2), 30, (1.0, 0.72, 0.45), (0.5, 0.5), target=(0.72, -0.7, 0)),
    lit14,
    # the streetlight by No. 14 catches its clapboard from the side
    light("facade14", "SPOT", (18.4 - 1.15, -6.0, G + 6.35), 3500, (0.72, 0.82, 1.0), 0.1, target=(26.0, 1.8, G + 2.6), spot=50, blend=0.7),
]
screen_light = light("screenLight", "AREA", (mx, my - 0.02, mz + 0.004), 22, (0.93, 0.91, 0.87), (PW, PH), target=(mx, my - 5, mz))
# the panel lights the desk the way a panel does: everywhere in front of it, falling off, with no edge to the pool
screen_light.data.spread = rad(180)

# ── bake ──
res = {"Near": 1024 if QUICK else 2048, "Room": 2048 if QUICK else 4096, "Street": 1024 if QUICK else 2048, "Front14": 512 if QUICK else 2048, "Lit14": 256 if QUICK else 512}
eres = {k: v // 2 for k, v in res.items()}
samples = 48 if QUICK else 400
groups = {"Near": join("Near", NEAR), "Room": join("Room", ROOM), "Street": join("Street", STREET), "Front14": join("Front14", FRONT), "Lit14": join("Lit14", LIT)}
# the dim, far surfaces outside need more samples than the room to bake clean
gsamples = {"Street": samples * 2, "Front14": samples * 3}
for o in groups.values():
    uv_atlas(o)

glass.hide_render = True
meta = {"scale": {}, "emitScale": {}}
baked = {}
# 1: everything except the screen
screen_light.hide_render = True
for name, o in groups.items():
    img = bake_into(o, name + "_base", res[name], gsamples.get(name, samples))
    s = auto_scale(img)
    meta["scale"][name] = s
    baked[name] = [save_jpeg(img, os.path.join(PREV, "..", "bake", name + "-base.jpg"), s), None]
# 2: the screen's light alone
for l in lights_base:
    l.hide_render = True
screen_light.hide_render = False
bg.inputs["Strength"].default_value = 0
for name in ("Near", "Room"):
    o = groups[name]
    img = bake_into(o, name + "_emit", eres[name], samples // 2)
    s = auto_scale(img, 99.9)
    meta["emitScale"][name] = s
    baked[name][1] = save_jpeg(img, os.path.join(PREV, "..", "bake", name + "-emit.jpg"), s, k=2)

# ── export ──
# the night sky behind the houses: not baked, drawn as its own faint emission
skym = mat("Sky", (0.0, 0.0, 0.0), 1.0, emit=(0.030, 0.040, 0.068), strength=1.0)
skyplane = box("SkyPlane", (75.0, 0.0, G + 25.0), (0.1, 260.0, 70.0), skym, 0)
meta["emitStrength"] = {"Sky": 1.0}
for name, o in groups.items():
    only_uv(o)
    set_material(o, baked_material(name, *baked[name]))
cams = [
    camera("CamScreen", (mx, my - 0.0115 - 0.19375 / math.tan(rad(14)), mz + 0.004), (mx, my, mz + 0.004), 28),
    camera("CamChair", (mx + 0.16, RD - 1.5, 1.24), (mx - 0.02, my, 0.98), 44, dof=2.8),
    camera("CamDesk", (mx + 0.62, RD - 0.98, 0.82), (mx, my, 0.96), 40, dof=2.0),
    camera("CamRoom", (0.72, 0.25, 1.62), (2.35, 2.65, 1.0), 52, dof=4),
    # a step back from the sill and a little below the blinds: the bottom rail a thin soft line at the top edge
    # (the live depth of field can't melt a rail held a hand's width from the lens)
    camera("CamWindow", (RW - 0.5, 2.25, 1.1), (26.0, 1.8, G + 3.85), 24, dof=4),
]
export_glb(os.path.join(OUT, "study.glb"), list(groups.values()) + [screen, glass, skyplane] + cams)


def gl(v):  # Blender Z-up → glTF Y-up
    return [round(v[0], 4), round(v[2], 4), round(-v[1], 4)]


corners = [screen.matrix_world @ Vector(c) for c in [(-PW / 2, -PH / 2, 0), (PW / 2, -PH / 2, 0), (PW / 2, PH / 2, 0), (-PW / 2, PH / 2, 0)]]
meta["screen"] = [gl(c) for c in corners]
meta["cameras"] = {}
for c in cams:
    fwd = c.matrix_world.to_3x3() @ Vector((0, 0, -1))
    meta["cameras"][c.name] = {"pos": gl(c.location), "dir": gl(fwd), "vfov": round(math.degrees(c.data.angle_y), 3), "focus": round(c.data.dof.focus_distance, 3) if c.data.dof.use_dof else None}
with open(os.path.join(OUT, "study.json"), "w") as f:
    json.dump(meta, f, indent=1)
print("META", json.dumps(meta))

# ── previews: exactly what the browser draws ──
if "previews" in A:
    for name, o in groups.items():
        emission_preview(o, baked[name][0], meta["scale"][name], baked[name][1], meta["emitScale"].get(name, 1), 1.0)
    for l in lights_base + [screen_light]:
        l.hide_render = True
    bg.inputs["Strength"].default_value = 0
    sm = screen.data.materials[0]
    bsdf(sm).inputs["Emission Color"].default_value = (0.78, 0.77, 0.74, 1)
    bsdf(sm).inputs["Emission Strength"].default_value = 1.0
    for c in cams:
        render_still(os.path.join(PREV, c.name + ".jpg"), c, 1600, 1000, 64)
    # asleep: the screen dark, so the room has only the street's light (the page's first image)
    for name in ("Near", "Room"):
        emission_preview(groups[name], baked[name][0], meta["scale"][name], baked[name][1], meta["emitScale"].get(name, 1), 0.0)
    bsdf(sm).inputs["Emission Strength"].default_value = 0.0
    render_still(os.path.join(PREV, "CamChair-asleep.jpg"), cams[1], 1600, 1000, 64)
    render_still(os.path.join(PREV, "CamRoom-asleep.jpg"), cams[3], 1600, 1000, 64)
