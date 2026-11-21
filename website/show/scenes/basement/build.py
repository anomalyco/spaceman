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
# newer concrete: lighter, smoother, trowelled in long arcs
patchm = noisy(mat("patch", rough=0.75), (0.36, 0.352, 0.335), (0.43, 0.42, 0.4), scale=2.5, detail=10, bump=0.03, mscale=(1, 3, 1))
seam = mat("seam", (0.085, 0.082, 0.078), 0.9)
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
    m = mat("wrap", (0.38, 0.395, 0.41), 0.3)
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
    for b in (0.71, 0.6, 0.3, -0.1, -0.5, -0.82):
        e = els.new((b - 0.028 + 1) / 2)
        e.color = (1, 1, 1, 1)
        e = els.new((b + 0.028 + 1) / 2)
        e.color = (0, 0, 0, 1)
    link(m, my.outputs["Result"], tr.inputs["Fac"])
    # blood: a noise, weighted toward the head and chest and the underside
    n = node(m, "ShaderNodeTexNoise")
    # through the plastic it is soft-edged and dull, darkest where it pressed against the sheet
    n.inputs["Scale"].default_value = 2.6
    n.inputs["Detail"].default_value = 3.0
    n.inputs["Roughness"].default_value = 0.45
    link(m, v.outputs[0], n.inputs["Vector"])
    up = node(m, "ShaderNodeMapRange")
    up.inputs["From Min"].default_value = -0.05
    up.inputs["From Max"].default_value = 0.65
    up.inputs["To Max"].default_value = 0.24
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
    br.color_ramp.elements[0].position = 0.655
    br.color_ramp.elements[0].color = (0.38, 0.395, 0.41, 1)
    br.color_ramp.elements[1].position = 0.92
    br.color_ramp.elements[1].color = (0.035, 0.004, 0.004, 1)
    e = br.color_ramp.elements.new(0.7)
    e.color = (0.11, 0.03, 0.027, 1)
    e = br.color_ramp.elements.new(0.77)
    e.color = (0.1, 0.018, 0.016, 1)
    link(m, a2.outputs[0], br.inputs["Fac"])
    mx = node(m, "ShaderNodeMix")
    mx.data_type = "RGBA"
    link(m, tr.outputs["Color"], mx.inputs[0])
    link(m, br.outputs["Color"], mx.inputs[6])
    mx.inputs[7].default_value = (0.1, 0.1, 0.105, 1)
    hn = node(m, "ShaderNodeTexNoise")
    # long and soft: the sheen runs along the creases he was wound in, never specks
    hs = node(m, "ShaderNodeVectorMath")
    hs.operation = "MULTIPLY"
    link(m, v.outputs[0], hs.inputs[0])
    hs.inputs[1].default_value = (1.0, 0.18, 1.0)
    hn.inputs["Scale"].default_value = 11.0
    hn.inputs["Detail"].default_value = 1.0
    hn.inputs["Roughness"].default_value = 0.4
    link(m, hs.outputs[0], hn.inputs["Vector"])
    hr = node(m, "ShaderNodeValToRGB")
    hr.color_ramp.elements[0].position = 0.5
    hr.color_ramp.elements[0].color = (0, 0, 0, 1)
    hr.color_ramp.elements[1].position = 0.72
    hr.color_ramp.elements[1].color = (0.2, 0.2, 0.2, 1)
    link(m, hn.outputs["Fac"], hr.inputs["Fac"])
    # only on what faces up toward the bulb
    up2 = node(m, "ShaderNodeMapRange")
    up2.inputs["From Min"].default_value = 0.06
    up2.inputs["From Max"].default_value = 0.14
    link(m, sep.outputs["Z"], up2.inputs["Value"])
    hm = node(m, "ShaderNodeMath")
    hm.operation = "MULTIPLY"
    link(m, hr.outputs["Color"], hm.inputs[0])
    link(m, up2.outputs["Result"], hm.inputs[1])
    sh = node(m, "ShaderNodeMix")
    sh.data_type = "RGBA"
    sh.blend_type = "SCREEN"
    link(m, hm.outputs[0], sh.inputs[0])
    link(m, mx.outputs[2], sh.inputs[6])
    sh.inputs[7].default_value = (0.75, 0.77, 0.8, 1)
    link(m, sh.outputs[2], bsdf(m).inputs["Base Color"])
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
    mx.inputs[7].default_value = (0.04, 0.007, 0.005, 1)
    link(m, mx.outputs[2], p.inputs["Base Color"])
    return m


def applied(o):
    """o with its modifiers baked into its mesh."""
    dg = bpy.context.evaluated_depsgraph_get()
    me = D.meshes.new_from_object(o.evaluated_get(dg))
    o.modifiers.clear()
    o.data = me
    return o


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
# the same bins, bought over years: pushed in to different depths, one lid left off and leaning,
# one shelf a size short, labels stuck on by hand at slightly different heights
brng = random.Random(16)
for s_i, zz in enumerate((0.12, 0.62, 1.12, 1.62)):
    R.append(box("shelf", ((SX0 + SX1) / 2, (SY0 + SY1) / 2, zz), (SX1 - SX0, SY1 - SY0, 0.02), steel, 0.002))
    for k in range(3):
        yy = SY0 + 0.25 + k * 0.45 + brng.uniform(-0.03, 0.03)
        xx = (SX0 + SX1) / 2 + brng.choice((0.0, 0.0, 0.025, 0.045))
        hh = 0.24 if (s_i, k) in ((1, 0), (1, 1)) else 0.3
        R.append(box("bin", (xx, yy, zz + 0.01 + hh / 2), (0.4, 0.4, hh), bin_m, 0.012))
        if (s_i, k) != (2, 2):
            R.append(box("lid", (xx, yy, zz + 0.02 + hh + 0.005), (0.42, 0.42, 0.03), lid, 0.008, rot=(0, 0, rad(brng.uniform(-3, 3)))))
        R.append(box("binlabel", (xx - 0.2 - 0.002, yy + brng.uniform(-0.03, 0.03), zz + 0.02 + hh * 0.62 + brng.uniform(-0.015, 0.015)),
                     (0.003, brng.choice((0.16, 0.12, 0.18)), 0.035), white, 0.0, rot=(rad(brng.uniform(-3, 3)), 0, 0)))
# the third bin's lid, off and leaning against the shelf post
R.append(box("lidOff", (SX0 - 0.03, SY1 - 0.24, 0.35), (0.03, 0.42, 0.42), lid, 0.008, rot=(0, rad(-8), 0)))
# the bulb's cord and socket (the bulb itself is emissive, not baked)
BULB = (3.25, 2.55, 1.92)
R.append(cyl("cord", (BULB[0], BULB[1], (BULB[2] + RH) / 2 + 0.05), 0.004, RH - BULB[2] - 0.1, black, 8))
R.append(cyl("socket", (BULB[0], BULB[1], BULB[2] + 0.06), 0.018, 0.05, black, 16))
# the two old places: newer concrete, saw-cut seams
for (cx, name) in PLACES[:2]:
    R.append(box("patch", (cx, (PY0 + PY1) / 2, 0.002), (0.8, PY1 - PY0, 0.004), patchm, 0.002, 3))
    for (sx, sy, w, h) in [(cx, PY0, 0.81, 0.007), (cx, PY1, 0.81, 0.007), (cx - 0.4, (PY0 + PY1) / 2, 0.007, 2.01), (cx + 0.4, (PY0 + PY1) / 2, 0.007, 2.01)]:
        R.append(box("seam", (sx, sy, 0.0042), (w, h, 0.001), seam, 0))


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
# the floor sheet over the third place, taped down at its edges: flat but for the creases it was folded
# in from the packet (a grid, alternately up and down) and a few broad soft waves
FX0, FX1, FY0, FY1 = 3.6, RW - 0.06, 1.25, RD - 0.2
from mathutils import noise as mnoise
bm = bmesh.new()
NXs, NYs = (120, 180) if QUICK else (240, 360)
bmesh.ops.create_grid(bm, x_segments=NXs, y_segments=NYs, size=0.5)
CX = [FX0 + (FX1 - FX0) * k / 4 for k in range(1, 4)]
CY = [FY0 + (FY1 - FY0) * k / 6 for k in range(1, 6)]


def sheet_z(x, y):
    z = 0.006
    for k, c in enumerate(CX):
        z += (1 if k % 2 else -0.6) * 0.0026 * max(0.0, 1 - abs(x - c) / 0.012) ** 1.5
    for k, c in enumerate(CY):
        z += (1 if k % 2 == 0 else -0.6) * 0.0022 * max(0.0, 1 - abs(y - c) / 0.012) ** 1.5
    z += 0.006 * max(0.0, mnoise.noise(Vector((x * 1.3, y * 0.9, 0.5))))
    z += 0.0015 * mnoise.noise(Vector((x * 7.0, y * 5.0, 2.0)))
    # held flat under the tape
    e = min(x - FX0, y - FY0)
    z = 0.004 + (z - 0.004) * min(1.0, max(0.0, (e - 0.03) / 0.12))
    wob = 1 + 0.5 * mnoise.noise(Vector((x * 2.0, y * 2.0, 4.0)))
    for d in (FX1 - x, FY1 - y):
        z += 0.022 * wob * max(0.0, 1 - d / 0.07) ** 2.2
    return z


for v in bm.verts:
    x, y = FX0 + (v.co.x + 0.5) * (FX1 - FX0), FY0 + (v.co.y + 0.5) * (FY1 - FY0)
    v.co = Vector((x, y, sheet_z(x, y)))
sheet = obj_from_bm("floorSheet", bm, plastic)
# the creases need the fine grid to be shaped, not to be drawn: most of it is flat
dm = sheet.modifiers.new("lighter", "DECIMATE")
dm.ratio = 0.12
applied(sheet)
for p in sheet.data.polygons:
    p.use_smooth = True
R.append(sheet)
for (sx, sy, w, h) in [((FX0 + FX1) / 2, FY0, FX1 - FX0, 0.05), (FX0, (FY0 + FY1) / 2, 0.05, FY1 - FY0)]:
    R.append(box("floorTape", (sx, sy, 0.006), (w, h, 0.002), tape, 0))

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
# him, lying on his back, head turned away, one arm a little out, feet fallen apart, wound in the sheet:
# ellipsoids in one mesh (with what the plastic bridges), voxel-remeshed and smoothed into one form, then
# creased, pleated and pulled in at the tape.
PARTS = [  # centre (x across, y along, z), radii, rotation (degrees)
    ((0.0, 0.83, 0.115), (0.082, 0.1, 0.09), (0, 20, 0)), ((0.025, 0.755, 0.1), (0.06, 0.055, 0.055), (0, 20, 0)),
    ((0.065, 0.83, 0.18), (0.016, 0.028, 0.02), (0, 40, 0)), ((0.01, 0.69, 0.08), (0.05, 0.07, 0.05), (0, 0, 0)),
    ((0.0, 0.47, 0.115), (0.175, 0.16, 0.1), (0, 0, 0)), ((0.0, 0.53, 0.115), (0.2, 0.08, 0.09), (0, 0, 0)),
    ((0.2, 0.56, 0.085), (0.07, 0.075, 0.06), (0, 0, 0)), ((-0.2, 0.56, 0.085), (0.07, 0.075, 0.06), (0, 0, 0)),
    ((0.0, 0.24, 0.105), (0.155, 0.17, 0.095), (0, 0, 0)),
    ((0.0, 0.03, 0.095), (0.17, 0.12, 0.085), (0, 0, 0)),
    ((0.255, 0.4, 0.065), (0.052, 0.15, 0.048), (0, 0, -4)), ((-0.265, 0.4, 0.065), (0.052, 0.15, 0.048), (0, 0, 8)),
    ((0.275, 0.12, 0.055), (0.042, 0.135, 0.038), (0, 0, -3)), ((-0.31, 0.13, 0.058), (0.042, 0.135, 0.038), (0, 0, 14)),
    ((0.28, -0.07, 0.045), (0.035, 0.065, 0.022), (0, 0, 0)), ((-0.345, -0.05, 0.048), (0.035, 0.065, 0.022), (0, 0, 18)),
    ((0.095, -0.25, 0.085), (0.085, 0.22, 0.075), (0, 0, -3)), ((-0.105, -0.25, 0.085), (0.085, 0.22, 0.075), (0, 0, 5)),
    ((0.105, -0.45, 0.075), (0.055, 0.05, 0.055), (0, 0, 0)), ((-0.125, -0.45, 0.078), (0.055, 0.05, 0.055), (0, 0, 0)),
    ((0.115, -0.64, 0.06), (0.052, 0.19, 0.05), (0, 0, -3)), ((-0.15, -0.64, 0.062), (0.052, 0.19, 0.05), (0, 0, 6)),
    ((0.14, -0.86, 0.09), (0.042, 0.055, 0.085), (0, 30, 0)), ((-0.19, -0.86, 0.088), (0.042, 0.055, 0.085), (0, -45, 0)),
]
from mathutils import Euler
# what the plastic bridges: between his arms and his sides, between his legs (bound together)
PARTS += [((0.2, 0.3, 0.075), (0.07, 0.26, 0.055), (0, 0, 0)), ((-0.21, 0.3, 0.075), (0.075, 0.26, 0.055), (0, 0, 4)),
          ((0.0, -0.42, 0.075), (0.11, 0.42, 0.06), (0, 0, 0)), ((0.0, -0.84, 0.065), (0.15, 0.07, 0.05), (0, 0, 0)),
          ((0.0, 0.05, 0.12), (0.2, 0.2, 0.07), (0, 0, 0))]
bm = bmesh.new()
for (c, r, rot) in PARTS:
    made = bmesh.ops.create_uvsphere(bm, u_segments=24, v_segments=16, radius=1.0)
    Mr = Euler([rad(a) for a in rot]).to_matrix()
    for v in made["verts"]:
        v.co = Mr @ Vector((v.co.x * r[0], v.co.y * r[1], v.co.z * r[2])) + Vector((BX + c[0], BY + c[1], c[2] + 0.004))
# the sheet's ends gathered past his head and feet, twisted and taped
for (y0, y1, rr) in ((0.93, 1.04, 0.05), (-0.92, -0.99, 0.045)):
    for k in range(6):
        t = k / 5
        made = bmesh.ops.create_uvsphere(bm, u_segments=16, v_segments=10, radius=1.0)
        q = rr * (1 - 0.6 * t)
        for v in made["verts"]:
            v.co = Vector((v.co.x * q * 1.2, v.co.y * q, v.co.z * q * 0.8)) + Vector((BX + (0.01 if y0 > 0 else -0.03), BY + y0 + (y1 - y0) * t, 0.04 + 0.01 * t))
body = obj_from_bm("body", bm, wrap_material(BX, BY))
rm = body.modifiers.new("remesh", "REMESH")
rm.mode = "VOXEL"
rm.voxel_size = 0.014 if QUICK else 0.01
print("remesh", rm.voxel_size, flush=True)
ls = body.modifiers.new("smooth", "LAPLACIANSMOOTH")
ls.lambda_factor = 1.0
ls.iterations = 40
applied(body)
# the plastic: pulled in at the tape, pleated where it's pulled, creased round him where it was wound
BANDS = (0.71, 0.6, 0.3, -0.1, -0.5, -0.82)
for v in body.data.vertices:
    x, y, z = v.co.x - BX, v.co.y - BY, v.co.z
    n = v.normal
    b = max(math.exp(-(((y - c) / 0.022) ** 2)) for c in BANDS)
    near = max(math.exp(-(((y - c) / 0.09) ** 2)) for c in BANDS)
    up = max(0.0, min(1.0, (z - 0.02) / 0.04))
    # wound round him on a slant, so the creases run across him at an angle, never as ribs
    wound = 1 - abs(mnoise.noise(Vector((x * 3.0, (y + 0.7 * x) * 10.0, z * 3.0))))
    pleat = 1 - abs(mnoise.noise(Vector((x * 30.0, y * 3.0, z * 30.0))))
    soft = mnoise.noise(Vector((x * 9.0, y * 9.0, z * 9.0)))
    d = -0.01 * b + up * (0.006 * wound ** 4 + 0.008 * near * pleat ** 3 + 0.002 * soft)
    v.co = v.co + n * d
for p in body.data.polygons:
    p.use_smooth = True
tex = D.textures.new("crinkle", "CLOUDS")
tex.noise_scale = 0.02
dp = body.modifiers.new("crinkle", "DISPLACE")
dp.texture = tex
dp.texture_coords = "GLOBAL"
dp.strength = 0.002
AFTER.append(body)
# the overlap of the sheet, loose on the floor along his right side
bm = bmesh.new()
rows = []
for i in range(81):
    y = -0.75 + 1.5 * i / 80
    w, _ = (0.0, 0.0)
    for (c, r, rot) in PARTS:
        d = (y - c[1]) / r[1]
        if abs(d) < 1 and c[0] > 0:
            w = max(w, c[0] + r[0] * math.sqrt(1 - d * d))
    reach = 0.07 + 0.05 * mnoise.noise(Vector((y * 3.0, 0.3, 0.7))) + 0.03 * math.sin(y * 9)
    row = []
    for j in range(7):
        t = j / 6
        row.append(bm.verts.new((BX + w - 0.02 + (reach + 0.02) * t, BY + y, 0.012 + 0.025 * (1 - t) ** 2 + 0.002 * math.sin(y * 40 + t * 5))))
    rows.append(row)
for i in range(80):
    for j in range(6):
        bm.faces.new([rows[i][j], rows[i][j + 1], rows[i + 1][j + 1], rows[i + 1][j]])
bm.normal_update()
if sum(f.normal.z for f in bm.faces) < 0:
    bmesh.ops.reverse_faces(bm, faces=bm.faces)
flap = obj_from_bm("flap", bm, wrap_material(BX, BY))
for p in flap.data.polygons:
    p.use_smooth = True
AFTER.append(flap)
print("body done", len(body.data.vertices), flush=True)


def pool_material():
    """Blood by its depth: near black where it's deep, red only in the thin rim, a little uneven."""
    m = mat("pool", rough=0.12)
    vc = node(m, "ShaderNodeVertexColor")
    vc.layer_name = "rad"
    sp = node(m, "ShaderNodeSeparateColor")
    link(m, vc.outputs["Color"], sp.inputs[0])
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 30.0
    n.inputs["Detail"].default_value = 6.0
    link(m, texcoord(m, "Object"), n.inputs["Vector"])
    a = node(m, "ShaderNodeMath")
    a.operation = "MULTIPLY_ADD"
    link(m, n.outputs["Fac"], a.inputs[0])
    a.inputs[1].default_value = 0.12
    link(m, sp.outputs[0], a.inputs[2])
    r = node(m, "ShaderNodeValToRGB")
    els = r.color_ramp.elements
    els[0].position, els[0].color = 0.05, (0.022, 0.001, 0.001, 1)
    els[1].position, els[1].color = 1.1, (0.07, 0.004, 0.003, 1)
    e = els.new(0.9)
    e.color = (0.035, 0.0015, 0.0012, 1)
    e = els.new(1.04)
    e.color = (0.16, 0.012, 0.009, 1)
    link(m, a.outputs[0], r.inputs["Fac"])
    link(m, r.outputs["Color"], bsdf(m).inputs["Base Color"])
    return m


poolm = pool_material()


def pool(name, cx, cy, r, z, seed, squash=(1.0, 1.0), segs=200, nring=7):
    """A pool with an irregular, fractal edge (a few runs where it found a fold), domed slightly at its rim."""
    rnd = random.Random(seed)
    harm = [(j, rnd.uniform(0, 2 * pi), 0.3 / j ** 1.15 * rnd.uniform(0.4, 1.3)) for j in range(2, 48)]
    runs = [(rnd.uniform(0, 2 * pi), rnd.uniform(0.08, 0.2), rnd.uniform(0.2, 0.6)) for _ in range(rnd.randint(1, 3))]

    def edge(a):
        k = 1 + sum(A * math.sin(j * a + p) for j, p, A in harm)
        for (la, lw, lh) in runs:
            d = math.atan2(math.sin(a - la), math.cos(a - la))
            k += lh * math.exp(-((d / lw) ** 2))
        return max(0.3, k)

    bm = bmesh.new()
    lay = bm.verts.layers.float_color.new("rad")
    cv = bm.verts.new((cx, cy, z + 0.0022))
    cv[lay] = (0, 0, 0, 1)
    prev = None
    for i in range(1, nring + 1):
        t = i / nring
        t2 = 1 - (1 - t) ** 1.6
        ring = []
        for k in range(segs):
            a = 2 * pi * k / segs
            e = edge(a) * r * t2
            v = bm.verts.new((cx + math.cos(a) * e * squash[0], cy + math.sin(a) * e * squash[1], z + 0.0022 * (1 - t ** 6)))
            v[lay] = (t, t, t, 1)
            ring.append(v)
        for k in range(segs):
            if prev is None:
                bm.faces.new([cv, ring[k], ring[(k + 1) % segs]])
            else:
                bm.faces.new([prev[k], ring[k], ring[(k + 1) % segs], prev[(k + 1) % segs]])
        prev = ring
    o = obj_from_bm(name, bm, poolm)
    # laid over the sheet's creases, as a film
    sw = o.modifiers.new("onto", "SHRINKWRAP")
    sw.target = sheet
    sw.wrap_method = "PROJECT"
    sw.use_project_z = True
    sw.use_negative_direction = True
    sw.use_positive_direction = True
    sw.offset = 0.0012
    return applied(o)


# under his head, run out toward the old places; through the plastic at his side; drops
for (cx, cy, r, sq, sd) in [(BX - 0.2, BY + 0.78, 0.3, (1.15, 1.0), 11), (BX + 0.3, BY + 0.42, 0.13, (0.9, 1.5), 12), (BX - 0.26, BY + 0.1, 0.07, (1.2, 1.0), 13)]:
    AFTER.append(pool("pool", cx, cy, r, 0.03, sd, sq))
rd = random.Random(21)
for k in range(14):
    a = rd.uniform(pi * 0.5, pi * 1.4)
    d = rd.uniform(0.45, 0.85)
    AFTER.append(pool("drop", BX - 0.2 + math.cos(a) * d, BY + 0.7 + math.sin(a) * d * 0.8, rd.uniform(0.006, 0.02), 0.03, 100 + k, segs=40, nring=3))
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
    camera("After", (2.75, 1.05, 0.82), (4.05, 3.05, 0.12), 40, dof=2.5),
    # a phone held upright: from past his feet, along him to the pool and FOURTEEN
    camera("After-tall", (4.2, 1.2, 1.05), (4.15, 3.3, 0.1), 60, dof=2.0),
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
    groups["StateAfter"].hide_render = False
    groups["StateBefore"].hide_render = True
    render_still(os.path.join(PREV, "After-tall.jpg"), cams[2], 600, 1300, 64)
