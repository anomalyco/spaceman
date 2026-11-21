# The basement at No. 12 ("six is downstairs"): concrete block, joists, a bare bulb, the stairs down the
# left wall. Three places side by side toward the far wall: two old patches of newer concrete, labelled,
# and the third under builder's plastic, its label already printed. Two states, each baked with the other
# hidden: StateBefore (the plastic down and waiting) and StateAfter (him on it, wrapped, and what came
# down the stairs with him). Exports public/show/3d/basement.glb + basement.json and the Canvas 2D stills.
# usage: Blender -b --factory-startup -P build.py -- [quick] [previews] [out=<dir>]
import os, sys, math, json, random
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import bpy, bmesh
from mathutils import Vector, Quaternion
from lib import *

A = args()
QUICK = "quick" in A
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = next((a[4:] for a in A if a.startswith("out=")), os.path.join(HERE, "..", "..", "..", "public", "show", "3d"))
PREV = os.path.join(HERE, "previews")
os.makedirs(OUT, exist_ok=True)
os.makedirs(PREV, exist_ok=True)
pi = math.pi
rad = math.radians
random.seed(14)

sc = reset()
gpu(sc)
RW, RD, RH = 5.6, 4.4, 2.3
PLACES = [(1.95, "SIXTEEN"), (3.10, "MARK"), (4.25, "FOURTEEN")]  # x centres; each 0.8 x 2.0, y 1.95..3.95
PY0, PY1 = 1.95, 3.95
BX, BY = 4.27, 2.86  # him

ROOM, DETAIL, BEFORE, AFTER = [], [], [], []

# ── materials ──
FLOOR = dict(color_a=(0.19, 0.185, 0.175), color_b=(0.28, 0.27, 0.255), scale=3.0, detail=8.0, rough=(0.7, 0.9), bump=0.1)
floorm = noisy(mat("floorc", rough=0.85), **FLOOR)
patchm = noisy(mat("patch", rough=0.8), (0.33, 0.325, 0.31), (0.38, 0.372, 0.355), scale=6, detail=6, bump=0.05)
seam = mat("seam", (0.03, 0.03, 0.03), 0.9)
joist = wood("joist", base=(0.5, 0.38, 0.24), dark=(0.36, 0.25, 0.14), scale=(1, 1, 14))
tread = wood("tread", base=(0.44, 0.31, 0.18), dark=(0.3, 0.2, 0.1))
sub = boards("subfloor", base=(0.4, 0.3, 0.19), dark=(0.28, 0.2, 0.12), width=0.2, length=2.4)
enamel = mat("enamel", (0.62, 0.62, 0.6), 0.3)
copper = mat("copper", (0.42, 0.22, 0.11), 0.4)
steel = mat("steelshelf", (0.22, 0.225, 0.23), 0.5)
bin_m = mat("bin", (0.58, 0.58, 0.56), 0.4)
lid = mat("lid", (0.06, 0.12, 0.3), 0.4)
white = mat("labeltape", (0.82, 0.82, 0.8), 0.5)
ink = mat("ink", (0.008, 0.008, 0.008), 0.5)
black = mat("black", (0.02, 0.02, 0.02), 0.6)
tape = mat("ducttape", (0.11, 0.11, 0.115), 0.5)
plastic = noisy(mat("plastic", rough=0.3), (0.4, 0.44, 0.48), (0.55, 0.59, 0.63), scale=12, detail=3)
maker = mat("labelmaker", (0.12, 0.12, 0.125), 0.5)
maker2 = mat("labelmaker2", (0.75, 0.56, 0.06), 0.5)
blood = noisy(mat("blood", rough=0.15), (0.05, 0.002, 0.002), (0.11, 0.006, 0.005), scale=9, detail=4)


def block(name, axis):
    """Painted concrete block, 0.4 x 0.2 courses; axis is the wall's horizontal axis ("X" or "Y")."""
    m = mat(name, (0.5, 0.49, 0.46), 0.9)
    sep = node(m, "ShaderNodeSeparateXYZ")
    link(m, texcoord(m, "Object"), sep.inputs[0])
    cb = node(m, "ShaderNodeCombineXYZ")
    link(m, sep.outputs[axis], cb.inputs["X"])
    link(m, sep.outputs["Z"], cb.inputs["Y"])
    b = node(m, "ShaderNodeTexBrick")
    b.offset = 0.5
    b.inputs["Scale"].default_value = 1.0
    b.inputs["Mortar Size"].default_value = 0.008
    b.inputs["Brick Width"].default_value = 0.4
    b.inputs["Row Height"].default_value = 0.2
    b.inputs["Color1"].default_value = (0.47, 0.46, 0.43, 1)
    b.inputs["Color2"].default_value = (0.52, 0.51, 0.48, 1)
    b.inputs["Mortar"].default_value = (0.38, 0.37, 0.35, 1)
    link(m, cb.outputs[0], b.inputs["Vector"])
    # damp creeping up from the floor
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 4.0
    n.inputs["Detail"].default_value = 8.0
    link(m, texcoord(m, "Object"), n.inputs["Vector"])
    h = node(m, "ShaderNodeMath")
    h.operation = "MULTIPLY_ADD"
    link(m, sep.outputs["Z"], h.inputs[0])
    h.inputs[1].default_value = 2.2
    link(m, n.outputs["Fac"], h.inputs[2])
    r = node(m, "ShaderNodeValToRGB")
    r.color_ramp.elements[0].position = 0.55
    r.color_ramp.elements[0].color = (0.62, 0.6, 0.55, 1)
    r.color_ramp.elements[1].position = 0.9
    r.color_ramp.elements[1].color = (1, 1, 1, 1)
    link(m, h.outputs[0], r.inputs["Fac"])
    mul = node(m, "ShaderNodeMix")
    mul.data_type = "RGBA"
    mul.blend_type = "MULTIPLY"
    mul.inputs[0].default_value = 1.0
    link(m, b.outputs["Color"], mul.inputs[6])
    link(m, r.outputs["Color"], mul.inputs[7])
    link(m, mul.outputs[2], bsdf(m).inputs["Base Color"])
    bp = node(m, "ShaderNodeBump")
    bp.inputs["Strength"].default_value = 0.4
    bp.inputs["Distance"].default_value = 0.004
    link(m, b.outputs["Fac"], bp.inputs["Height"])
    link(m, bp.outputs["Normal"], bsdf(m).inputs["Normal"])
    return m


blockX, blockY = block("blockX", "X"), block("blockY", "Y")


def wrap_material(bx, by):
    """Builder's plastic round him, taped at five places; blood through it at the head and chest, and
    pooled along the underside."""
    m = mat("wrap", (0.56, 0.57, 0.58), 0.3)
    v = node(m, "ShaderNodeVectorMath")
    v.operation = "SUBTRACT"
    link(m, texcoord(m, "Object"), v.inputs[0])
    v.inputs[1].default_value = (bx, by, 0)
    sep = node(m, "ShaderNodeSeparateXYZ")
    link(m, v.outputs[0], sep.inputs[0])
    # tape bands along the body (local y from -1 to 1 mapped to 0..1)
    my = node(m, "ShaderNodeMapRange")
    my.inputs["From Min"].default_value = -1.0
    my.inputs["From Max"].default_value = 1.0
    link(m, sep.outputs["Y"], my.inputs["Value"])
    tr = node(m, "ShaderNodeValToRGB")
    tr.color_ramp.interpolation = "CONSTANT"
    els = tr.color_ramp.elements
    els[0].position = 0.0
    els[0].color = (0, 0, 0, 1)
    els[1].position = 0.999
    els[1].color = (0, 0, 0, 1)
    for b in (0.6, 0.3, -0.1, -0.5, -0.82):
        e = els.new((b - 0.028 + 1) / 2)
        e.color = (1, 1, 1, 1)
        e = els.new((b + 0.028 + 1) / 2)
        e.color = (0, 0, 0, 1)
    link(m, my.outputs["Result"], tr.inputs["Fac"])
    # blood: a noise, weighted toward the head and chest and the underside
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 3.5
    n.inputs["Detail"].default_value = 10.0
    n.inputs["Roughness"].default_value = 0.62
    link(m, v.outputs[0], n.inputs["Vector"])
    up = node(m, "ShaderNodeMapRange")
    up.inputs["From Min"].default_value = -0.05
    up.inputs["From Max"].default_value = 0.65
    up.inputs["To Max"].default_value = 0.32
    link(m, sep.outputs["Y"], up.inputs["Value"])
    low = node(m, "ShaderNodeMapRange")
    low.inputs["From Min"].default_value = 0.12
    low.inputs["From Max"].default_value = 0.03
    low.inputs["To Max"].default_value = 0.3
    link(m, sep.outputs["Z"], low.inputs["Value"])
    a1 = node(m, "ShaderNodeMath")
    a1.operation = "ADD"
    link(m, n.outputs["Fac"], a1.inputs[0])
    link(m, up.outputs["Result"], a1.inputs[1])
    a2 = node(m, "ShaderNodeMath")
    a2.operation = "ADD"
    link(m, a1.outputs[0], a2.inputs[0])
    link(m, low.outputs["Result"], a2.inputs[1])
    br = node(m, "ShaderNodeValToRGB")
    br.color_ramp.elements[0].position = 0.68
    br.color_ramp.elements[0].color = (0.56, 0.57, 0.58, 1)
    br.color_ramp.elements[1].position = 0.86
    br.color_ramp.elements[1].color = (0.05, 0.002, 0.002, 1)
    e = br.color_ramp.elements.new(0.74)
    e.color = (0.24, 0.025, 0.02, 1)
    link(m, a2.outputs[0], br.inputs["Fac"])
    mx = node(m, "ShaderNodeMix")
    mx.data_type = "RGBA"
    link(m, tr.outputs["Color"], mx.inputs[0])
    link(m, br.outputs["Color"], mx.inputs[6])
    mx.inputs[7].default_value = (0.1, 0.1, 0.105, 1)
    link(m, mx.outputs[2], bsdf(m).inputs["Base Color"])
    return m


def smear_material():
    """The floor's own concrete (same noise, same place) with blood laid over it where the vertex colour says."""
    m = noisy(mat("smear", rough=0.85), **FLOOR)
    p = bsdf(m)
    src = [l for l in m.node_tree.links if l.to_socket == p.inputs["Base Color"]][0].from_socket
    vc = node(m, "ShaderNodeVertexColor")
    vc.layer_name = "smear"
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 40.0
    n.inputs["Detail"].default_value = 6.0
    link(m, texcoord(m, "Object"), n.inputs["Vector"])
    a = node(m, "ShaderNodeMath")
    a.operation = "MULTIPLY_ADD"
    link(m, n.outputs["Fac"], a.inputs[0])
    a.inputs[1].default_value = 0.6
    link(m, vc.outputs["Color"], a.inputs[2])
    r = node(m, "ShaderNodeValToRGB")
    r.color_ramp.elements[0].position = 0.62
    r.color_ramp.elements[1].position = 0.72
    link(m, a.outputs[0], r.inputs["Fac"])
    mx = node(m, "ShaderNodeMix")
    mx.data_type = "RGBA"
    link(m, r.outputs["Color"], mx.inputs[0])
    link(m, src, mx.inputs[6])
    mx.inputs[7].default_value = (0.07, 0.004, 0.003, 1)
    link(m, mx.outputs[2], p.inputs["Base Color"])
    return m


def text(name, s, center, size, rot=(0, 0, 0), m=ink):
    cu = D.curves.new(name, "FONT")
    cu.body = s
    cu.size = size
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    o = D.objects.new(name, cu)
    bpy.context.scene.collection.objects.link(o)
    o.data.materials.append(m)
    bpy.ops.object.select_all(action="DESELECT")
    o.select_set(True)
    bpy.context.view_layer.objects.active = o
    bpy.ops.object.convert(target="MESH")
    o = bpy.context.view_layer.objects.active
    w = o.dimensions.x
    o.location = center
    o.rotation_euler = rot
    return o, w


def label(name, s, center, size=0.07):
    """A label-maker label lying flat: white tape, black capitals."""
    x, y, z = center
    t, w = text(name + "Text", s, (x, y, z + 0.0026), size)
    return [box(name, (x, y, z + 0.0012), (w + 0.06, size * 1.15, 0.0024), white, 0.0), t]


def blob(name, cx, cy, r, z, m, seed, squash=(1.0, 1.0), segs=72):
    rnd = random.Random(seed)
    ph = [rnd.uniform(0, 2 * pi) for _ in range(5)]
    amp = [0.3, 0.18, 0.12, 0.08, 0.06]
    bm = bmesh.new()
    c = bm.verts.new((cx, cy, z))
    ring = []
    for i in range(segs):
        a = 2 * pi * i / segs
        k = 1 + sum(amp[j] * math.sin((j + 2) * a + ph[j]) for j in range(5))
        ring.append(bm.verts.new((cx + math.cos(a) * r * k * squash[0], cy + math.sin(a) * r * k * squash[1], z)))
    for i in range(segs):
        bm.faces.new([c, ring[i], ring[(i + 1) % segs]])
    return obj_from_bm(name, bm, m)


# ── the room ──
R = ROOM
R.append(box("floor", (RW / 2, RD / 2, -0.05), (RW + 0.4, RD + 0.4, 0.1), floorm, 0))
R.append(box("wallL", (-0.1, RD / 2, 1.3), (0.2, RD + 0.4, 2.8), blockY, 0))
R.append(box("wallR", (RW + 0.1, RD / 2, 1.3), (0.2, RD + 0.4, 2.8), blockY, 0))
R.append(box("wallN", (RW / 2, -0.1, 1.3), (RW + 0.4, 0.2, 2.8), blockX, 0))
R.append(box("wallF", (RW / 2, RD + 0.1, 1.3), (RW + 0.4, 0.2, 2.8), blockX, 0))
# the ceiling: joists on 0.4 m centres under the boards, open over the stairs
R.append(box("subfloor", (RW / 2 + 0.5, RD / 2, RH + 0.215), (RW - 1.0, RD, 0.03), sub, 0))
R.append(box("subfloor2", (0.5, 3.0 + (RD - 3.0) / 2, RH + 0.215), (1.0, RD - 3.0, 0.03), sub, 0))
x = 0.2
while x < RW:
    y0 = 3.05 if x < 1.0 else 0.0
    R.append(box("joist", (x, (y0 + RD) / 2, RH + 0.1), (0.045, RD - y0, 0.2), joist, 0.003))
    x += 0.4
R.append(box("header", (0.5, 3.0, RH + 0.1), (1.0, 0.09, 0.2), joist, 0.003))
R.append(box("trimmer", (1.0, RD / 2, RH + 0.1), (0.09, RD, 0.2), joist, 0.003))
# stairs down the left wall, from the top at the near wall to the floor at y = 3
n = 11
rise, run = RH / (n + 1), 0.27
for i in range(n):
    z = RH - (i + 1) * rise
    R.append(box("tread", (0.47, i * run + run / 2 + 0.02, z - 0.018), (0.86, run + 0.03, 0.036), tread, 0.004))
L = math.hypot(n * run, RH)
ang = math.atan2(RH, n * run)
for sx in (0.05, 0.92):
    R.append(box("stringer", (sx, n * run / 2, RH / 2 - 0.1), (0.04, L, 0.25), joist, 0.003, rot=(-ang, 0, 0)))
# a handrail on the wall
R.append(cyl("rail", (0.06, n * run / 2, RH / 2 + 0.78), 0.022, L, tread, 16, rot=(pi / 2 - ang, 0, 0)))
# a floor drain, the boiler in the near corner, steel shelving on the right wall with his bins
R.append(cyl("drain", (2.95, 1.15, 0.001), 0.075, 0.004, black, 32))
R.append(cyl("boiler", (5.2, 0.42, 0.8), 0.3, 1.6, enamel, 48))
for k, (px, py) in enumerate([(5.12, 0.3), (5.28, 0.3)]):
    R.append(cyl("pipe", (px, py, 1.6 + (RH + 0.2 - 1.6) / 2), 0.016, RH + 0.2 - 1.6, copper, 12))
SX0, SX1, SY0, SY1 = RW - 0.48, RW - 0.03, 1.0, 2.4
for (px, py) in [(SX0, SY0), (SX1, SY0), (SX0, SY1), (SX1, SY1)]:
    R.append(box("post", (px, py, 0.92), (0.03, 0.03, 1.84), steel, 0.002))
for zz in (0.12, 0.62, 1.12, 1.62):
    R.append(box("shelf", ((SX0 + SX1) / 2, (SY0 + SY1) / 2, zz), (SX1 - SX0, SY1 - SY0, 0.02), steel, 0.002))
    for k in range(3):
        yy = SY0 + 0.25 + k * 0.45
        R.append(box("bin", ((SX0 + SX1) / 2, yy, zz + 0.16), (0.4, 0.4, 0.3), bin_m, 0.012))
        R.append(box("lid", ((SX0 + SX1) / 2, yy, zz + 0.32), (0.42, 0.42, 0.03), lid, 0.008))
        R.append(box("binlabel", (SX0 + 0.019, yy, zz + 0.2), (0.003, 0.16, 0.035), white, 0.0))
# the bulb's cord and socket (the bulb itself is emissive, not baked)
BULB = (3.25, 2.55, 1.92)
R.append(cyl("cord", (BULB[0], BULB[1], (BULB[2] + RH) / 2 + 0.05), 0.004, RH - BULB[2] - 0.1, black, 8))
R.append(cyl("socket", (BULB[0], BULB[1], BULB[2] + 0.06), 0.018, 0.05, black, 16))
# the two old places: newer concrete, saw-cut seams
for (cx, name) in PLACES[:2]:
    R.append(box("patch", (cx, (PY0 + PY1) / 2, 0.0005), (0.8, PY1 - PY0, 0.002), patchm, 0))
    for (sx, sy, w, h) in [(cx, PY0, 0.82, 0.012), (cx, PY1, 0.82, 0.012), (cx - 0.4, (PY0 + PY1) / 2, 0.012, 2.02), (cx + 0.4, (PY0 + PY1) / 2, 0.012, 2.02)]:
        R.append(box("seam", (sx, sy, 0.0008), (w, h, 0.002), seam, 0))


def hung_plastic(name, axis, c0, c1, fixed, inward):
    """A sheet taped up a wall, hanging in loose vertical folds and spreading a little onto the floor."""
    nu, nv = (90, 40) if not QUICK else (45, 20)
    bm = bmesh.new()
    vs = []
    rnd = random.Random(name)
    ph = [rnd.uniform(0, 6.28) for _ in range(3)]
    for j in range(nv + 1):
        z = RH - 0.04 - (RH - 0.04 + 0.08) * j / nv
        row = []
        for i in range(nu + 1):
            u = c0 + (c1 - c0) * i / nu
            fold = 0.022 * math.sin(u * 17 + ph[0]) + 0.012 * math.sin(u * 41 + ph[1]) + 0.008 * math.sin(u * 7 + ph[2] + z * 2)
            fold *= 0.35 + 0.65 * (1 - z / RH)
            d = 0.015 + abs(fold)
            zz = z
            if z < 0.004:
                d += -z  # the part past the floor lies flat on it, pulled into the room
                zz = 0.004
            p = (u, fixed - inward * d, zz) if axis == "X" else (fixed - inward * d, u, zz)
            row.append(bm.verts.new(p))
        vs.append(row)
    for j in range(nv):
        for i in range(nu):
            bm.faces.new([vs[j][i], vs[j][i + 1], vs[j + 1][i + 1], vs[j + 1][i]])
    # face into the room
    if axis == "X":
        bmesh.ops.reverse_faces(bm, faces=bm.faces)
    o = obj_from_bm(name, bm, plastic)
    for p in o.data.polygons:
        p.use_smooth = True
    return o


R.append(hung_plastic("sheetF", "X", 1.3, RW - 0.05, RD, 1))
R.append(hung_plastic("sheetR", "Y", SY1 + 0.15, RD - 0.03, RW, 1))
R.append(box("tapeF", ((1.3 + RW) / 2, RD - 0.012, RH - 0.05), (RW - 1.35, 0.004, 0.05), tape, 0))
R.append(box("tapeR", (RW - 0.012, (SY1 + 0.15 + RD) / 2, RH - 0.05), (0.004, RD - SY1 - 0.18, 0.05), tape, 0))
# the floor sheet over the third place, taped down at its edges
FX0, FX1, FY0, FY1 = 3.6, RW - 0.06, 1.25, RD - 0.2
bm = bmesh.new()
bmesh.ops.create_grid(bm, x_segments=60, y_segments=80, size=0.5)
for v in bm.verts:
    v.co = Vector((FX0 + (v.co.x + 0.5) * (FX1 - FX0), FY0 + (v.co.y + 0.5) * (FY1 - FY0), 0.011 + 0.005 * math.sin(v.co.x * 23 + v.co.y * 7) * math.sin(v.co.y * 31) + 0.003 * math.sin(v.co.x * 57 - v.co.y * 13)))
R.append(obj_from_bm("floorSheet", bm, plastic))
for (sx, sy, w, h) in [((FX0 + FX1) / 2, FY0, FX1 - FX0, 0.05), (FX0, (FY0 + FY1) / 2, 0.05, FY1 - FY0)]:
    R.append(box("floorTape", (sx, sy, 0.02), (w, h, 0.002), tape, 0))

# ── the labels (the third already printed) ──
for k, (cx, name) in enumerate(PLACES):
    DETAIL.extend(label("label%d" % k, name, (cx, PY1 + 0.15, 0.0 if k < 2 else 0.02)))

# ── before: the label maker squared to the sheet's edge, the tape ──
BEFORE.append(box("maker", (3.42, 1.55, 0.03), (0.11, 0.2, 0.05), maker, 0.01))
BEFORE.append(box("makerKeys", (3.42, 1.51, 0.056), (0.085, 0.09, 0.004), maker2, 0.002))
BEFORE.append(box("makerScreen", (3.42, 1.6, 0.056), (0.07, 0.04, 0.004), black, 0.001))
BEFORE.append(cyl("tapeRoll", (3.4, 1.9, 0.025), 0.055, 0.05, tape, 32))
BEFORE.append(cyl("tapeCore", (3.4, 1.9, 0.0505), 0.036, 0.002, mat("core", (0.42, 0.33, 0.2), 0.9), 32))

# ── after: him, wrapped; the pool; the trail down the stairs and across the floor ──
# the body: a cocoon lofted through cross-sections from the feet (y -0.93) to the head (y 0.97)
PROF = [(-0.93, 0.07, 0.09), (-0.88, 0.13, 0.15), (-0.78, 0.12, 0.11), (-0.6, 0.15, 0.115), (-0.4, 0.17, 0.125),
        (-0.15, 0.2, 0.14), (0.05, 0.225, 0.165), (0.2, 0.215, 0.16), (0.38, 0.27, 0.2), (0.53, 0.265, 0.19),
        (0.61, 0.13, 0.12), (0.68, 0.12, 0.14), (0.79, 0.13, 0.18), (0.9, 0.11, 0.15), (0.97, 0.05, 0.08)]


def prof(y):
    if y <= PROF[0][0]:
        return PROF[0][1:]
    for a, b in zip(PROF, PROF[1:]):
        if y <= b[0]:
            t = (y - a[0]) / (b[0] - a[0])
            t = t * t * (3 - 2 * t)
            return (a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t)
    return PROF[-1][1:]


def sprof(y):
    ws = [prof(y + d) for d in (-0.03, -0.015, 0, 0.015, 0.03)]
    return (sum(w for w, _ in ws) / 5, sum(h for _, h in ws) / 5)


bm = bmesh.new()
NR, NM = 150 if not QUICK else 70, 48
rings = []
for r in range(NR + 1):
    y = -0.93 + 1.9 * r / NR
    w, h = sprof(y)
    zc = h * 0.45
    ring = []
    for k in range(NM):
        th = 2 * pi * k / NM
        c, s_ = math.cos(th), math.sin(th)
        x = w * math.copysign(abs(c) ** 0.8, c)
        if s_ >= 0:
            z = zc + (h - zc) * s_
            if y < -0.12:
                z -= 0.035 * math.exp(-((x / 0.035) ** 2)) * min(1.0, (-0.12 - y) / 0.15) * s_
        else:
            z = zc - (zc - 0.012) * abs(s_) ** 0.5
        ring.append(bm.verts.new((BX + x, BY + y, z)))
    rings.append(ring)
for r in range(NR):
    for k in range(NM):
        bm.faces.new([rings[r][k], rings[r][(k + 1) % NM], rings[r + 1][(k + 1) % NM], rings[r + 1][k]])
for ring, y in ((rings[0], -0.95), (rings[-1], 0.99)):
    c = bm.verts.new((BX, BY + y, prof(y)[1] * 0.45))
    for k in range(NM):
        bm.faces.new([ring[k], ring[(k + 1) % NM], c])
bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
body = obj_from_bm("body", bm, wrap_material(BX, BY))
for p in body.data.polygons:
    p.use_smooth = True
tex = D.textures.new("crinkle", "CLOUDS")
tex.noise_scale = 0.045
dp = body.modifiers.new("crinkle", "DISPLACE")
dp.texture = tex
dp.texture_coords = "GLOBAL"
dp.strength = 0.008
tex2 = D.textures.new("folds", "CLOUDS")
tex2.noise_scale = 0.16
dp2 = body.modifiers.new("folds", "DISPLACE")
dp2.texture = tex2
dp2.texture_coords = "GLOBAL"
dp2.strength = 0.018
AFTER.append(body)
for (cx, cy, r, sq, sd) in [(BX - 0.32, BY + 0.62, 0.34, (1.0, 1.3), 1), (BX + 0.28, BY + 0.3, 0.2, (1.0, 1.6), 2), (BX - 0.2, BY - 0.25, 0.12, (1.2, 1.0), 3)]:
    AFTER.append(blob("pool", cx, cy, r, 0.021, blood, sd, sq))
# the trail: from the foot of the stairs, round the old places, to the sheet
path = [(0.55, 3.15), (0.85, 2.55), (1.15, 1.85), (1.7, 1.62), (2.6, 1.55), (3.3, 1.7), (3.62, 1.95)]


def along(t):
    """A point and a tangent on the path at t in 0..1 (piecewise linear, smoothed)."""
    segs = [(Vector(path[i]), Vector(path[i + 1])) for i in range(len(path) - 1)]
    lens = [(b - a).length for a, b in segs]
    tot = sum(lens)
    d = t * tot
    for (a, b), l in zip(segs, lens):
        if d <= l:
            return a + (b - a) * (d / l), (b - a).normalized(), tot
        d -= l
    a, b = segs[-1]
    return b, (b - a).normalized(), tot


bm = bmesh.new()
NU, NV = 260, 8
rows = []
shade = []
for i in range(NU + 1):
    t = i / NU
    p, tg, tot = along(t)
    nrm = Vector((-tg.y, tg.x))
    w = 0.2 + 0.08 * math.sin(t * 19) + 0.04 * math.sin(t * 53)
    row = []
    for j in range(NV + 1):
        s = (j / NV - 0.5) * 2
        q = p + nrm * s * w / 2
        row.append(bm.verts.new((q.x, q.y, 0.0035)))
        fade = min(1.0, t * 6) * (0.55 + 0.45 * math.sin(t * 31 + 1.3) ** 2) * (1 - 0.6 * t)
        shade.append(max(0.0, (1 - s * s * 1.1)) * fade)
    rows.append(row)
for i in range(NU):
    for j in range(NV):
        bm.faces.new([rows[i][j], rows[i][j + 1], rows[i + 1][j + 1], rows[i + 1][j]])
bm.normal_update()
if sum(f.normal.z for f in bm.faces) < 0:
    bmesh.ops.reverse_faces(bm, faces=bm.faces)
trail = obj_from_bm("trail", bm, smear_material())
ca = trail.data.color_attributes.new("smear", "FLOAT_COLOR", "POINT")
for k, v in enumerate(trail.data.vertices):
    a = shade[k]
    ca.data[k].color = (a, a, a, 1)
AFTER.append(trail)
# and the label maker, put down beside him when the label went on the floor
AFTER.append(box("maker", (BX + 0.62, BY + 1.0, 0.045), (0.11, 0.2, 0.05), maker, 0.01, rot=(0, 0, rad(4))))
AFTER.append(box("makerKeys", (BX + 0.62, BY + 0.96, 0.071), (0.085, 0.09, 0.004), maker2, 0.002, rot=(0, 0, rad(4))))

# ── the bulb (emissive, drawn live) and the light ──
bulbm = mat("Bulb", (1.0, 0.8, 0.55), 0.5, emit=(1.0, 0.72, 0.42), strength=1.0)
bulb = D.objects.new("Bulb", D.meshes.new("Bulb"))
bpy.context.scene.collection.objects.link(bulb)
bm = bmesh.new()
bmesh.ops.create_uvsphere(bm, u_segments=24, v_segments=16, radius=0.032)
bm.to_mesh(bulb.data)
bulb.data.materials.append(bulbm)
bulb.location = BULB
bg = world((0.0, 0.0, 0.0), 0.0)
lights = [
    light("bulb", "POINT", BULB, 55, (1.0, 0.72, 0.42), 0.03),
    light("upstairs", "AREA", (0.47, 0.6, RH + 0.6), 6, (0.7, 0.78, 1.0), (0.8, 1.0), target=(0.47, 0.6, 0)),
]
bulb.hide_render = True

# ── bake ──
res = {"Room": 2048 if QUICK else 4096, "Detail": 512 if QUICK else 1024, "StateBefore": 512 if QUICK else 1024, "StateAfter": 1024 if QUICK else 2048}
samples = 48 if QUICK else 400
groups = {"Room": join("Room", ROOM), "Detail": join("Detail", DETAIL), "StateBefore": join("StateBefore", BEFORE), "StateAfter": join("StateAfter", AFTER)}
for o in groups.values():
    uv_atlas(o)
meta = {"scale": {}, "emitScale": {}, "emitStrength": {"Bulb": 40.0}}
baked = {}
for name, o in groups.items():
    # the room and its labels are baked empty, so neither state leaves a shadow in the other
    groups["StateAfter"].hide_render = name != "StateAfter"
    groups["StateBefore"].hide_render = name != "StateBefore"
    img = bake_into(o, name + "_base", res[name], samples)
    s = auto_scale(img)
    meta["scale"][name] = s
    baked[name] = save_jpeg(img, os.path.join(HERE, "bake", name + "-base.jpg"), s)
for name, o in groups.items():
    o.hide_render = False
    only_uv(o)
    set_material(o, baked_material(name, baked[name]))

cams = [
    camera("Before", (0.5, 1.45, 2.1), (3.75, 3.0, 0.1), 56, dof=4.0),
    camera("After", (3.05, 0.42, 1.62), (3.8, 3.1, 0.12), 50, dof=2.8),
]
bulb.hide_render = False
export_glb(os.path.join(OUT, "basement.glb"), list(groups.values()) + [bulb] + cams)


def gl(v):
    return [round(v[0], 4), round(v[2], 4), round(-v[1], 4)]


meta["cameras"] = {}
for c in cams:
    fwd = c.matrix_world.to_3x3() @ Vector((0, 0, -1))
    meta["cameras"][c.name] = {"pos": gl(c.location), "dir": gl(fwd), "vfov": round(math.degrees(c.data.angle_y), 3), "focus": round(c.data.dof.focus_distance, 3)}
with open(os.path.join(OUT, "basement.json"), "w") as f:
    json.dump(meta, f, indent=1)
print("META", json.dumps(meta))

# ── previews: exactly what the browser draws (and the Canvas 2D stills) ──
if "previews" in A:
    for name, o in groups.items():
        emission_preview(o, baked[name], meta["scale"][name])
    for l in lights:
        l.hide_render = True
    bsdf(bulbm).inputs["Emission Strength"].default_value = meta["emitStrength"]["Bulb"]
    for c, state in ((cams[0], "StateBefore"), (cams[1], "StateAfter")):
        groups["StateAfter"].hide_render = state != "StateAfter"
        groups["StateBefore"].hide_render = state == "StateAfter"
        render_still(os.path.join(PREV, c.name + ".jpg"), c, 1600, 1000, 64)
    # and the other state from each camera, to check
    for c, state in ((cams[0], "StateAfter"), (cams[1], "StateBefore")):
        groups["StateAfter"].hide_render = state != "StateAfter"
        groups["StateBefore"].hide_render = state == "StateAfter"
        render_still(os.path.join(PREV, c.name + "-" + state + ".jpg"), c, 1600, 1000, 64)
