# The study at No. 12, upstairs at the front, at night; and No. 14 across the street.
# Builds the set, bakes its light into atlases (the screen's light separately), exports
# public/show/3d/study.glb + study.json, and renders emission previews of what the browser draws.
# usage: Blender -b --factory-startup -P build.py -- [quick] [previews] [out=<dir>]
import os, sys, math, json
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import bpy
from mathutils import Vector
from lib import *

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

NEAR, ROOM, STREET = [], [], []

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
# outside
lawn = noisy(mat("lawn", rough=1.0), (0.03, 0.05, 0.02), (0.05, 0.07, 0.03), scale=2, detail=8)
asphalt = noisy(mat("asphalt", rough=0.8), (0.05, 0.05, 0.052), (0.07, 0.07, 0.072), scale=6, detail=8)
concrete = noisy(mat("concrete", rough=0.85), (0.25, 0.245, 0.235), (0.3, 0.295, 0.285), scale=4, detail=8)
hedgem = noisy(mat("hedge", rough=0.9), (0.02, 0.045, 0.02), (0.05, 0.09, 0.035), scale=60, detail=6, bump=0.6)
siding = mat("siding", (0.5, 0.5, 0.47), 0.7)
trim = mat("trim", (0.62, 0.62, 0.6), 0.5)
roof = noisy(mat("roof", rough=0.9), (0.07, 0.07, 0.075), (0.1, 0.1, 0.105), scale=30)
darkglass = mat("darkglass", (0.01, 0.012, 0.015), 0.1)
lamp_m = mat("lamppost", (0.08, 0.08, 0.085), 0.5)
lit = mat("No14Window", (0.0, 0.0, 0.0), 1.0, emit=(1.0, 0.68, 0.38), strength=3.0)

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
    for row, hz in enumerate([0.831, 1.201]):
        for i in range(11):
            x = sx - 0.36 + 0.034 + i * 0.066
            R.append(box("binder%d_%d" % (row, i), (x, sy + 0.005, hz + 0.011 + 0.155), (0.058, 0.28, 0.31), binder, 0.003))
            R.append(box("label%d_%d" % (row, i), (x, sy - 0.136, hz + 0.011 + 0.2), (0.034, 0.002, 0.07), label, 0.0))
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
    S = STREET
    S.append(box("yard", ((RW + T + 9.5) / 2, 0, G - 0.05), (9.5 - RW - T, 80, 0.1), lawn, 0))
    S.append(box("walk1", (10.25, 0, G - 0.03), (1.5, 80, 0.1), concrete, 0.01))
    S.append(box("road", (14.5, 0, G - 0.1), (7.0, 80, 0.1), asphalt, 0))
    S.append(box("kerb1", (11.0, 0, G - 0.06), (0.15, 80, 0.12), concrete, 0.01))
    S.append(box("kerb2", (18.0, 0, G - 0.06), (0.15, 80, 0.12), concrete, 0.01))
    S.append(box("walk2", (18.75, 0, G - 0.03), (1.5, 80, 0.1), concrete, 0.01))
    S.append(box("yard14", (32, 0, G - 0.05), (23, 80, 0.1), lawn, 0))
    S.append(box("path14", (22.8, 1.0, G - 0.02), (6.4, 1.2, 0.1), concrete, 0.01))
    # the hedge, clipped, a little over six foot
    for i, (y0, y1) in enumerate([(-9.0, 0.35), (1.65, 9.0)]):
        h = box("hedge%d" % i, (20.3, (y0 + y1) / 2, G + 0.96), (0.95, y1 - y0, 1.92), hedgem, 0.12, 3)
        sub = h.modifiers.new("sub", "SUBSURF")
        sub.levels = sub.render_levels = 2
        tex = bpy.data.textures.new("hedgeNoise", "CLOUDS")
        tex.noise_scale = 0.25
        dp = h.modifiers.new("disp", "DISPLACE")
        dp.texture = tex
        dp.strength = 0.06
        S.append(h)
    # No. 14
    hx0, hx1, hy0, hy1, hh = 26.0, 33.5, -4.0, 8.0, 5.6
    S.append(box("house14", ((hx0 + hx1) / 2, (hy0 + hy1) / 2, G + hh / 2), (hx1 - hx0, hy1 - hy0, hh), siding, 0.02))
    bm = bmesh.new()
    rw2, ridge = (hx1 - hx0) / 2 + 0.4, 2.6
    vs = [bm.verts.new(v) for v in [(-rw2, hy0 - 0.4, 0), (rw2, hy0 - 0.4, 0), (0, hy0 - 0.4, ridge), (-rw2, hy1 + 0.4, 0), (rw2, hy1 + 0.4, 0), (0, hy1 + 0.4, ridge)]]
    for f in [(0, 1, 2), (3, 5, 4), (0, 2, 5, 3), (1, 4, 5, 2), (0, 3, 4, 1)]:
        bm.faces.new([vs[i] for i in f])
    r = obj_from_bm("roof14", bm, roof)
    r.location = ((hx0 + hx1) / 2, 0, G + hh)
    S.append(r)
    # windows on the front: ground floor two and the door; upstairs three, the middle one lit
    fx = hx0 - 0.03
    for (wy, wz, ww, wh) in [(-1.6, 1.2, 1.4, 1.3), (5.2, 1.2, 1.4, 1.3), (-1.6, 3.9, 1.2, 1.2), (5.2, 3.9, 1.2, 1.2)]:
        S.append(box("win", (fx, wy, G + wz), (0.06, ww, wh), darkglass, 0.005))
        S.append(box("winTrim", (fx + 0.01, wy, G + wz), (0.05, ww + 0.14, wh + 0.14), trim, 0.008))
    S.append(box("door14", (fx, 1.0, G + 1.05), (0.06, 1.0, 2.1), roof, 0.01))
    S.append(box("doorTrim", (fx + 0.01, 1.0, G + 1.1), (0.05, 1.16, 2.25), trim, 0.01))
    S.append(box("litTrim", (fx + 0.01, 1.8, G + 3.9), (0.05, 1.34, 1.34), trim, 0.008))
    # neighbours, either side
    for k, yy in enumerate([-16.0, 18.0]):
        S.append(box("house%d" % k, (29.5, yy, G + 2.8), (7.0, 10.0, 5.6), siding, 0.02))
        S.append(box("nwin%d" % k, (25.97, yy, G + 3.9), (0.06, 1.2, 1.2), darkglass, 0.005))
    # streetlights
    for k, (px, py) in enumerate([(18.4, -6.0), (10.6, 8.5)]):
        S.append(cyl("pole%d" % k, (px, py, G + 3.25), 0.07, 6.5, lamp_m, 16, r2=0.05))
        S.append(box("arm%d" % k, (px + (-0.6 if px > 14 else 0.6), py, G + 6.45), (1.2, 0.08, 0.06), lamp_m, 0.01))
        S.append(box("head%d" % k, (px + (-1.15 if px > 14 else 1.15), py, G + 6.4), (0.5, 0.22, 0.07), lamp_m, 0.01))


walls()
desk()
street()

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
litwin = box("No14Window", (26.0 - 0.06, 1.8, G + 3.9), (0.02, 1.2, 1.2), lit, 0)

# ── light ──
bg = world((0.0035, 0.005, 0.011), 1.0)
cool = (0.72, 0.82, 1.0)
lights_base = [
    light("streetInto", "SPOT", (10.6 + 1.15, 8.5, G + 6.35), 5000, cool, 0.05, target=(RW, (WY0 + WY1) / 2, 1.2), spot=16, blend=0.5),
    light("street14", "SPOT", (18.4 - 1.15, -6.0, G + 6.35), 9000, cool, 0.1, target=(17.0, -6.0, G), spot=130, blend=0.6),
    light("streetOurs", "SPOT", (10.6 + 1.15, 8.5, G + 6.35), 9000, cool, 0.1, target=(11.7, 8.5, G), spot=130, blend=0.6),
    light("moon", "SUN", (0, 0, 20), 0.05, (0.55, 0.65, 1.0), 0.02, target=(8, 12, 0)),
    light("hall", "AREA", (0.72, -0.7, 2.2), 30, (1.0, 0.72, 0.45), (0.5, 0.5), target=(0.72, -0.7, 0)),
    light("lit14", "AREA", (25.5, 1.8, G + 3.9), 60, (1.0, 0.68, 0.38), (1.0, 1.0), target=(10, 1.8, G + 2.5)),
]
screen_light = light("screenLight", "AREA", (mx, my - 0.02, mz + 0.004), 22, (0.93, 0.91, 0.87), (PW, PH), target=(mx, my - 5, mz))
screen_light.data.spread = rad(160)

# ── bake ──
res = {"Near": 1024 if QUICK else 2048, "Room": 2048 if QUICK else 4096, "Street": 1024 if QUICK else 2048}
eres = {k: v // 2 for k, v in res.items()}
samples = 48 if QUICK else 400
groups = {"Near": join("Near", NEAR), "Room": join("Room", ROOM), "Street": join("Street", STREET)}
for o in groups.values():
    uv_atlas(o)

glass.hide_render = True
meta = {"scale": {}, "emitScale": {}}
baked = {}
# 1: everything except the screen
screen_light.hide_render = True
for name, o in groups.items():
    img = bake_into(o, name + "_base", res[name], samples)
    s = auto_scale(img)
    meta["scale"][name] = s
    baked[name] = [save_jpeg(img, os.path.join(PREV, "..", "bake", name + "-base.jpg"), s), None]
# 2: the screen's light alone
for l in lights_base:
    l.hide_render = True
screen_light.hide_render = False
bg.inputs["Strength"].default_value = 0
bsdf(lit).inputs["Emission Strength"].default_value = 0
for name in ("Near", "Room"):
    o = groups[name]
    img = bake_into(o, name + "_emit", eres[name], samples // 2)
    s = auto_scale(img, 99.9)
    meta["emitScale"][name] = s
    baked[name][1] = save_jpeg(img, os.path.join(PREV, "..", "bake", name + "-emit.jpg"), s, k=2)
bsdf(lit).inputs["Emission Strength"].default_value = 3.0

# ── export ──
for name, o in groups.items():
    only_uv(o)
    set_material(o, baked_material(name, *baked[name]))
cams = [
    camera("CamScreen", (mx, my - 0.0115 - 0.19375 / math.tan(rad(14)), mz + 0.004), (mx, my, mz + 0.004), 28),
    camera("CamChair", (mx + 0.16, RD - 1.5, 1.24), (mx - 0.02, my, 0.98), 44, dof=2.8),
    camera("CamDesk", (mx + 0.62, RD - 0.98, 0.82), (mx, my, 0.96), 40, dof=2.0),
    camera("CamRoom", (0.72, 0.25, 1.62), (2.35, 2.65, 1.0), 52, dof=4),
    camera("CamWindow", (RW - 0.42, 2.25, 1.06), (26.0, 1.8, G + 3.85), 24, dof=4),
]
export_glb(os.path.join(OUT, "study.glb"), list(groups.values()) + [screen, glass, litwin] + cams)


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
        render_still(os.path.join(PREV, c.name + ".jpg"), c, 960, 600, 32)
    # asleep: the screen dark, so the room has only the street's light (the page's first image)
    for name in ("Near", "Room"):
        emission_preview(groups[name], baked[name][0], meta["scale"][name], baked[name][1], meta["emitScale"].get(name, 1), 0.0)
    bsdf(sm).inputs["Emission Strength"].default_value = 0.0
    render_still(os.path.join(PREV, "CamChair-asleep.jpg"), cams[1], 960, 600, 32)
    render_still(os.path.join(PREV, "CamRoom-asleep.jpg"), cams[3], 960, 600, 32)
