# Pinecrest: the street between No. 12 (his house, the study upstairs at the front) and No. 14 across it.
# Shared by the study (baked into study.glb, seen from his window) and the offline renders (his photographs
# of the hedge, the doorbell camera, the photograph No. 14 took of his window).
# Coordinates: Blender Z-up, metres. x runs east across the street; No. 12's front wall (the study's east
# wall) is at x = 3.6..3.72; No. 14's front is at x = 26. G is the street's height (the study floor is 0).
import math, random
import bpy, bmesh
from mathutils import Vector
from lib import *

pi = math.pi
rad = math.radians
HEDGE_H = 1.88  # six foot two


def materials(season=0.0, wet=0.0):
    """season 0 summer .. 1 November; wet darkens and glosses the hard ground."""
    lerp = lambda a, b: tuple(a[i] + (b[i] - a[i]) * season for i in range(3))
    dark = lambda c, k: tuple(v * k for v in c)
    wk = 1.0 - 0.45 * wet
    M = {}
    M["lawn"] = noisy(mat("lawn", rough=1.0), lerp((0.03, 0.05, 0.02), (0.06, 0.055, 0.025)), lerp((0.05, 0.075, 0.03), (0.09, 0.08, 0.04)), scale=2, detail=8, bump=0.3)
    M["asphalt"] = noisy(mat("asphalt", rough=0.8), dark((0.05, 0.05, 0.052), wk), dark((0.075, 0.075, 0.078), wk), scale=6, detail=8, rough=(0.8 - 0.65 * wet, 0.9 - 0.6 * wet), bump=0.15)
    M["concrete"] = noisy(mat("concrete", rough=0.85), dark((0.25, 0.245, 0.235), wk), dark((0.31, 0.305, 0.295), wk), scale=4, detail=8, rough=(0.8 - 0.6 * wet, 0.9 - 0.55 * wet), bump=0.1)
    M["hedge"] = hedge_material(lerp((0.02, 0.045, 0.02), (0.05, 0.035, 0.015)), lerp((0.05, 0.09, 0.035), (0.11, 0.075, 0.03)))
    M["siding"] = clapboard("siding", (0.52, 0.53, 0.5))
    M["siding12"] = clapboard("siding12", (0.44, 0.41, 0.36))
    M["trim"] = mat("trim", (0.66, 0.66, 0.64), 0.5)
    M["roof"] = noisy(mat("roof", rough=0.9), (0.1, 0.098, 0.1), (0.17, 0.165, 0.165), scale=30, bump=0.3, mscale=(1, 1, 9))
    M["gutter"] = mat("gutter", (0.55, 0.55, 0.53), 0.4)
    M["blind"] = noisy(mat("blind", rough=1.0), (0.28, 0.27, 0.25), (0.34, 0.33, 0.3), scale=60, detail=2)
    M["door"] = mat("door14", (0.09, 0.14, 0.12), 0.45)
    M["darkglass"] = mat("darkglass", (0.012, 0.014, 0.018), 0.08)
    M["lamppost"] = mat("lamppost", (0.08, 0.08, 0.085), 0.5)
    M["plaster"] = mat("inner", (0.5, 0.47, 0.42), 0.9)
    M["curtain"] = noisy(mat("curtain", rough=1.0), (0.42, 0.24, 0.12), (0.5, 0.3, 0.16), scale=40, detail=2)
    M["shade"] = mat("shade", (0.85, 0.75, 0.6), 0.9)
    M["mailbox"] = mat("mailbox", (0.05, 0.05, 0.055), 0.4)
    M["pole_r"] = mat("pole_r", (0.55, 0.04, 0.03), 0.5)
    M["pole_w"] = mat("pole_w", (0.8, 0.8, 0.78), 0.5)
    return M


def hedge_material(a, b):
    """Privet, clipped: small leaves (a voronoi), lighter tips, deep dark gaps."""
    m = mat("hedge", rough=0.75)
    tc = texcoord(m, "Object")
    v = node(m, "ShaderNodeTexVoronoi")
    v.inputs["Scale"].default_value = 70.0
    link(m, tc, v.inputs["Vector"])
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 9.0
    n.inputs["Detail"].default_value = 6.0
    link(m, tc, n.inputs["Vector"])
    mx = node(m, "ShaderNodeMix")
    mx.data_type = "FLOAT"
    mx.inputs[0].default_value = 0.5
    link(m, v.outputs["Distance"], mx.inputs[2])
    link(m, n.outputs["Fac"], mx.inputs[3])
    r = node(m, "ShaderNodeValToRGB")
    r.color_ramp.elements[0].position = 0.15
    r.color_ramp.elements[0].color = (a[0] * 0.4, a[1] * 0.4, a[2] * 0.4, 1)
    r.color_ramp.elements[1].position = 0.7
    r.color_ramp.elements[1].color = (*b, 1)
    e = r.color_ramp.elements.new(0.42)
    e.color = (*a, 1)
    link(m, mx.outputs[0], r.inputs["Fac"])
    link(m, r.outputs["Color"], bsdf(m).inputs["Base Color"])
    bp = node(m, "ShaderNodeBump")
    bp.inputs["Strength"].default_value = 0.9
    bp.inputs["Distance"].default_value = 0.02
    link(m, mx.outputs[0], bp.inputs["Height"])
    link(m, bp.outputs["Normal"], bsdf(m).inputs["Normal"])
    return m


def clapboard(name, col):
    """Painted clapboard siding: horizontal boards 0.18 m apart, each shadowed under its lip."""
    m = mat(name, col, 0.6)
    mp = node(m, "ShaderNodeMapping")
    mp.inputs["Scale"].default_value = (0, 0, 1.0 / 0.18)
    link(m, texcoord(m, "Object"), mp.inputs["Vector"])
    sep = node(m, "ShaderNodeSeparateXYZ")
    link(m, mp.outputs["Vector"], sep.inputs[0])
    fr = node(m, "ShaderNodeMath")
    fr.operation = "FRACT"
    link(m, sep.outputs["Z"], fr.inputs[0])
    r = node(m, "ShaderNodeValToRGB")
    r.color_ramp.elements[0].color = (col[0] * 0.55, col[1] * 0.55, col[2] * 0.55, 1)
    r.color_ramp.elements[0].position = 0.0
    r.color_ramp.elements[1].color = (*col, 1)
    r.color_ramp.elements[1].position = 0.16
    link(m, fr.outputs[0], r.inputs["Fac"])
    link(m, r.outputs["Color"], bsdf(m).inputs["Base Color"])
    bp = node(m, "ShaderNodeBump")
    bp.inputs["Strength"].default_value = 0.5
    bp.inputs["Distance"].default_value = 0.01
    link(m, fr.outputs[0], bp.inputs["Height"])
    link(m, bp.outputs["Normal"], bsdf(m).inputs["Normal"])
    return m


def hedge(name, x, y0, y1, G, M, levels=2):
    h = box(name, (x, (y0 + y1) / 2, G + HEDGE_H / 2), (0.95, y1 - y0, HEDGE_H), M["hedge"], 0.14, 3)
    # an even mesh first, so the lumps below have vertices to move (a subdivided box has a few big faces)
    rm = h.modifiers.new("remesh", "REMESH")
    rm.mode = "VOXEL"
    rm.voxel_size = 0.07 if levels > 2 else 0.14
    rm.use_smooth_shade = True
    # clipped, but by hand: a slow wave along the top and the face, then the leaf-mass lumps
    for k, (scale, strength) in enumerate([(1.6, 0.24), (0.45, 0.1), (0.18, 0.07), (0.06, 0.025 if levels > 2 else 0.0)]):
        if strength <= 0:
            continue
        tex = bpy.data.textures.new("hedgeNoise%d" % k, "CLOUDS")
        tex.noise_scale = scale
        dp = h.modifiers.new("disp%d" % k, "DISPLACE")
        dp.texture = tex
        dp.texture_coords = "GLOBAL"
        dp.strength = strength
    return h


def house(name, x0, x1, y0, y1, G, M, siding, hh=5.6, body=True):
    out = []
    if body:
        out.append(box(name, ((x0 + x1) / 2, (y0 + y1) / 2, G + hh / 2), (x1 - x0, y1 - y0, hh), siding, 0.02))
    bm = bmesh.new()
    # the ridge runs across the plot (gable ends to the sides), so the street sees the slope
    hw, rw2, ridge = (x1 - x0) / 2 + 0.45, (y1 - y0) / 2 + 0.3, 2.4
    vs = [bm.verts.new(v) for v in [(-hw, -rw2, 0), (hw, -rw2, 0), (hw, rw2, 0), (-hw, rw2, 0), (0, -rw2, ridge), (0, rw2, ridge)]]
    for f in [(0, 4, 5, 3), (1, 2, 5, 4), (0, 1, 4), (3, 5, 2)]:
        bm.faces.new([vs[i] for i in f])
    # an open shell, so face it outward by hand (the live renderer culls back faces)
    bm.normal_update()
    for f in bm.faces:
        if f.normal.dot(f.calc_center_median() - Vector((0, 0, ridge * 0.3))) < 0:
            f.normal_flip()
    bm.normal_update()
    r = obj_from_bm(name + "Roof", bm, M["roof"])
    r.location = ((x0 + x1) / 2, (y0 + y1) / 2, G + hh)
    out.append(r)
    # the eave: a fascia board and a gutter along the front, a downpipe at the corner; a plinth
    L = y1 - y0 + 0.6
    out.append(box(name + "Fascia", (x0 - 0.44, (y0 + y1) / 2, G + hh - 0.06), (0.04, L, 0.2), M["trim"], 0.005))
    out.append(box(name + "Gutter", (x0 - 0.5, (y0 + y1) / 2, G + hh - 0.1), (0.11, L, 0.1), M["gutter"], 0.02))
    out.append(cyl(name + "Downpipe", (x0 - 0.08, y1 - 0.15, G + (hh - 0.1) / 2), 0.04, hh - 0.1, M["gutter"], 12))
    if body:
        out.append(box(name + "Plinth", ((x0 + x1) / 2, (y0 + y1) / 2, G + 0.18), (x1 - x0 + 0.05, y1 - y0 + 0.05, 0.36), M["concrete"], 0.01))
    return out


def cut(o, center, size):
    """Cuts a box out of o (a boolean, applied when the set is joined); the cutter never renders."""
    c = box("cutter", center, size, o.data.materials[0], 0)  # its faces take o's own material
    c.hide_render = True
    c.display_type = "WIRE"
    m = o.modifiers.new("cut", "BOOLEAN")
    m.operation = "DIFFERENCE"
    m.object = c
    m.solver = "EXACT"
    return c


def window(fx, wy, wz, ww, wh, M, facing=-1, glass=True):
    """Trim, sill and dark glass on a front at x = fx (facing -1: the front looks toward -x)."""
    out = []
    s = facing
    if glass:
        out.append(box("win", (fx + s * 0.02, wy, wz), (0.04, ww, wh), M["darkglass"], 0.004))
        # a dark room behind it, its blind part way down (a different way in every window)
        k = 0.25 + 0.5 * ((math.sin(wy * 12.9898 + wz * 78.233) * 43758.5453) % 1.0)
        out.append(box("blind", (fx + s * 0.045, wy, wz + wh / 2 - wh * k / 2), (0.004, ww - 0.02, wh * k), M["blind"], 0))
    out.append(box("winTrimL", (fx + s * 0.04, wy - ww / 2 - 0.05, wz), (0.06, 0.1, wh + 0.2), M["trim"], 0.006))
    out.append(box("winTrimR", (fx + s * 0.04, wy + ww / 2 + 0.05, wz), (0.06, 0.1, wh + 0.2), M["trim"], 0.006))
    out.append(box("winTrimT", (fx + s * 0.04, wy, wz + wh / 2 + 0.05), (0.06, ww + 0.2, 0.1), M["trim"], 0.006))
    out.append(box("winSill", (fx + s * 0.07, wy, wz - wh / 2 - 0.04), (0.12, ww + 0.3, 0.06), M["trim"], 0.006))
    if glass:
        out.append(box("winMull", (fx + s * 0.035, wy, wz), (0.04, 0.05, wh), M["trim"], 0.004))
    return out


def build(G, M, detail=False, ours=False, props=False):
    """Returns ({street, lit, ours}: lists of objects, the lit room's lamp)."""
    S, L, O = [], [], []
    S.append(box("yard", ((3.72 + 9.5) / 2, 0, G - 0.05), (9.5 - 3.72, 80, 0.1), M["lawn"], 0))
    S.append(box("walk1", (10.25, 0, G - 0.03), (1.5, 80, 0.1), M["concrete"], 0.01))
    S.append(box("road", (14.5, 0, G - 0.1), (7.0, 80, 0.1), M["asphalt"], 0))
    S.append(box("kerb1", (11.0, 0, G - 0.06), (0.15, 80, 0.12), M["concrete"], 0.01))
    S.append(box("kerb2", (18.0, 0, G - 0.06), (0.15, 80, 0.12), M["concrete"], 0.01))
    S.append(box("walk2", (18.75, 0, G - 0.03), (1.5, 80, 0.1), M["concrete"], 0.01))
    S.append(box("yard14", (32, 0, G - 0.05), (23, 80, 0.1), M["lawn"], 0))
    S.append(box("path14", (22.8, 1.0, G - 0.02), (6.4, 1.2, 0.1), M["concrete"], 0.01))
    S.append(box("drive12", (6.6, -6.0, G - 0.02), (5.8, 3.0, 0.1), M["concrete"], 0.01))
    S.append(box("path12", (6.6, -0.9, G - 0.02), (5.8, 1.1, 0.1), M["concrete"], 0.01))
    for i, (y0, y1) in enumerate([(-9.0, 0.35), (1.65, 9.0)]):
        S.append(hedge("hedge%d" % i, 20.3, y0, y1, G, M, 3 if detail else 2))
    hx0 = 26.0
    h14 = house("house14", hx0, 33.5, -4.0, 8.0, G, M, M["siding"])
    S += h14
    for (wy, wz, ww, wh) in [(-1.6, 1.25, 1.4, 1.3), (5.2, 1.25, 1.4, 1.3), (-1.6, 3.9, 1.2, 1.2), (5.2, 3.9, 1.2, 1.2)]:
        S += window(hx0, wy, G + wz, ww, wh, M)
    S.append(box("door14", (hx0 - 0.03, 1.0, G + 1.05), (0.06, 0.95, 2.1), M["door"], 0.01))
    S.append(box("doorTrim", (hx0 - 0.01, 1.0, G + 1.1), (0.05, 1.15, 2.25), M["trim"], 0.01))
    S.append(box("step14", (hx0 - 0.35, 1.0, G + 0.07), (0.7, 1.5, 0.14), M["concrete"], 0.01))
    S.append(box("porchLamp", (hx0 - 0.08, 1.75, G + 2.0), (0.1, 0.12, 0.22), M["lamppost"], 0.01))
    S.append(cyl("mailbox", (19.4, 0.8, G + 1.05), 0.12, 0.42, M["mailbox"], 24, rot=(pi / 2, 0, pi / 2)))
    S.append(box("mailpost", (19.4, 0.8, G + 0.45), (0.08, 0.08, 0.9), M["lamppost"], 0.005))
    # his lit room: a recess behind the open window, warm, a curtain half drawn, a lamp
    wy, wz, ww, wh = 1.8, G + 3.9, 1.2, 1.2
    L += window(hx0, wy, wz, ww, wh, M, glass=False)
    d = 2.6
    # a real opening in No. 14's front, and a room behind it
    cut(h14[0], (hx0 + 0.02, wy, wz), (0.3, ww, wh))
    cut(h14[0], (hx0 + 0.12 + (d - 0.02) / 2, wy, wz), (d - 0.02 + 0.1, 3.2, 2.6))
    L.append(box("litBack", (hx0 + d, wy, wz), (0.05, 3.0, 2.4), M["plaster"], 0))
    e0 = 0.14  # the room's walls start behind the front wall, not in its face
    L.append(box("litL", (hx0 + (e0 + d) / 2, wy - 1.5, wz), (d - e0, 0.05, 2.4), M["plaster"], 0))
    L.append(box("litR", (hx0 + (e0 + d) / 2, wy + 1.5, wz), (d - e0, 0.05, 2.4), M["plaster"], 0))
    L.append(box("litC", (hx0 + (e0 + d) / 2, wy, wz + 1.2), (d - e0, 3.0, 0.05), M["plaster"], 0))
    L.append(box("litF", (hx0 + (e0 + d) / 2, wy, wz - 1.2), (d - e0, 3.0, 0.05), M["plaster"], 0))
    L.append(box("curtain", (hx0 + 0.12, wy - 0.38, wz + 0.02), (0.04, 0.5, 1.26), M["curtain"], 0.02, 3))
    L.append(cyl("shade", (hx0 + 1.6, wy + 0.5, wz - 0.2), 0.2, 0.26, M["shade"], 32, r2=0.13))
    L.append(cyl("lampStem", (hx0 + 1.6, wy + 0.5, wz - 0.75), 0.015, 0.9, M["lamppost"], 12))
    L.append(box("shelf", (hx0 + d - 0.18, wy - 0.7, wz - 0.3), (0.3, 1.0, 1.6), M["roof"], 0.01))
    lamp = light("lit14", "POINT", (hx0 + 1.45, wy + 0.5, wz - 0.12), 140, (1.0, 0.66, 0.36), 0.05)
    for k, yy in enumerate([-16.0, 18.0]):
        S += house("house%d" % k, 26.0, 33.0, yy - 5.0, yy + 5.0, G, M, M["siding"])
        S += window(26.0, yy, G + 3.9, 1.2, 1.2, M)
        S += window(26.0, yy - 2.5, G + 1.25, 1.4, 1.3, M)
    for k, (px, py) in enumerate([(18.4, -6.0), (10.6, 8.5)]):
        S.append(cyl("pole%d" % k, (px, py, G + 3.25), 0.07, 6.5, M["lamppost"], 16, r2=0.05))
        S.append(box("arm%d" % k, (px + (-0.6 if px > 14 else 0.6), py, G + 6.45), (1.2, 0.08, 0.06), M["lamppost"], 0.01))
        S.append(box("head%d" % k, (px + (-1.15 if px > 14 else 1.15), py, G + 6.4), (0.5, 0.22, 0.07), M["lamppost"], 0.01))
    if props:
        # his survey pole against the hedge: red and white, half a metre a band
        for i in range(4):
            S.append(cyl("pole_band%d" % i, (19.72, -4.2, G + 0.25 + i * 0.5), 0.016, 0.5, M["pole_r"] if i % 2 == 0 else M["pole_w"], 16))
    if ours:
        O += no12(G, M)
    return {"street": S, "lit": L, "ours": O}, lamp


def no12(G, M):
    """No. 12 from outside: the front at x = 3.72, the study window upstairs (y 1.70..2.95, z 0.90..2.15)."""
    out = []
    fx = 3.72
    WY0, WY1, WZ0, WZ1 = 1.70, 2.95, 0.90, 2.15
    x0, y0, y1, top = -6.0, -4.5, 7.5, 2.6
    sid = M["siding12"]
    xc, xw = (x0 + fx) / 2, fx - x0
    out.append(box("n12below", (xc, (y0 + y1) / 2, (G + WZ0) / 2), (xw, y1 - y0, WZ0 - G), sid, 0.02))
    out.append(box("n12above", (xc, (y0 + y1) / 2, (WZ1 + top) / 2), (xw, y1 - y0, top - WZ1), sid, 0.02))
    out.append(box("n12left", (xc, (y0 + WY0) / 2, (WZ0 + WZ1) / 2), (xw, WY0 - y0, WZ1 - WZ0), sid, 0.0))
    out.append(box("n12right", (xc, (WY1 + y1) / 2, (WZ0 + WZ1) / 2), (xw, y1 - WY1, WZ1 - WZ0), sid, 0.0))
    # hollow behind the study window: the study, roughly (the walls inset so they don't fight the shell)
    out.append(box("n12in_floor", (1.8, 1.6, 0.0), (3.4, 3.0, 0.02), M["roof"], 0))
    out.append(box("n12in_back", (0.2, 1.6, 1.25), (0.05, 3.0, 2.5), M["plaster"], 0))
    out.append(box("n12in_n", (1.8, 3.1, 1.25), (3.4, 0.05, 2.5), M["plaster"], 0))
    out.append(box("n12in_s", (1.8, 0.1, 1.25), (3.4, 0.05, 2.5), M["plaster"], 0))
    out.append(box("n12in_c", (1.8, 1.6, 2.45), (3.4, 3.0, 0.02), M["plaster"], 0))
    out.append(box("n12desk", (2.0, 2.8, 0.72), (1.6, 0.75, 0.03), M["roof"], 0.004))
    out.append(box("n12mon", (2.0, 2.93, 1.07), (0.65, 0.025, 0.41), M["mailbox"], 0.004))
    out.append(box("n12bc", (0.66, 2.9, 0.95), (0.8, 0.32, 1.9), M["roof"], 0.004))
    out.append(box("n12chair", (2.0, 2.1, 0.86), (0.46, 0.07, 0.55), M["mailbox"], 0.02))
    yc = (WY0 + WY1) / 2
    z = WZ1 - 0.07
    while z > 1.24:
        out.append(box("n12slat", (3.565, yc, z), (0.025, WY1 - WY0 - 0.08, 0.0012), M["trim"], 0.0, rot=(0, rad(22), 0)))
        z -= 0.021
    out += window(fx, yc, (WZ0 + WZ1) / 2, WY1 - WY0, WZ1 - WZ0, M, facing=1, glass=False)
    out += window(fx, -2.4, G + 1.25, 1.4, 1.3, M, facing=1)
    out.append(box("n12door", (fx + 0.03, -0.9, G + 1.05), (0.06, 0.95, 2.1), M["door"], 0.01))
    r = house("n12", x0, fx, y0, y1, top - 5.6, M, sid, body=False)
    out += r
    return out


def night(G, bg_strength=1.0):
    bg = world((0.0035, 0.005, 0.011), bg_strength)
    cool = (0.72, 0.82, 1.0)
    L = [
        light("streetInto", "SPOT", (10.6 + 1.15, 8.5, G + 6.35), 5000, cool, 0.05, target=(3.6, 2.3, 1.2), spot=16, blend=0.5),
        light("street14", "SPOT", (18.4 - 1.15, -6.0, G + 6.35), 9000, cool, 0.1, target=(17.0, -6.0, G), spot=130, blend=0.6),
        light("streetOurs", "SPOT", (10.6 + 1.15, 8.5, G + 6.35), 9000, cool, 0.1, target=(11.7, 8.5, G), spot=130, blend=0.6),
        light("moon", "SUN", (0, 0, 20), 0.05, (0.55, 0.65, 1.0), 0.02, target=(8, 12, 0)),
    ]
    return bg, L


def sky(strength, elev, rot, turbidity=2.5, tint=(1, 1, 1)):
    """A physical sky (whatever model this Blender has)."""
    w = D.worlds.new("Sky")
    w.use_nodes = True
    nt = w.node_tree
    bg = nt.nodes["Background"]
    s = nt.nodes.new("ShaderNodeTexSky")
    for t in ("MULTIPLE_SCATTERING", "NISHITA", "SINGLE_SCATTERING", "HOSEK_WILKIE"):
        try:
            s.sky_type = t
            break
        except Exception:
            continue
    for k, v in (("sun_elevation", rad(elev)), ("sun_rotation", rad(rot)), ("sun_disc", False), ("turbidity", turbidity), ("air_density", 1.0), ("dust_density", 1.0 + turbidity)):
        try:
            setattr(s, k, v)
        except Exception:
            pass
    mixc = nt.nodes.new("ShaderNodeMix")
    mixc.data_type = "RGBA"
    mixc.blend_type = "MULTIPLY"
    mixc.inputs[0].default_value = 1.0
    mixc.inputs[7].default_value = (*tint, 1)
    nt.links.new(s.outputs["Color"], mixc.inputs[6])
    nt.links.new(mixc.outputs[2], bg.inputs["Color"])
    bg.inputs["Strength"].default_value = strength
    bpy.context.scene.world = w
    print("sky", s.sky_type)
    return bg


def leaves(G, n=900, seed=3):
    """November: fallen leaves on the verge and the pavement in front of the hedge."""
    random.seed(seed)
    bm = bmesh.new()
    for i in range(n):
        x = random.uniform(17.0, 21.6)
        y = random.uniform(-9.0, 3.0)
        z = G + (0.0 if x > 18.0 else -0.04) + 0.012
        s = random.uniform(0.035, 0.06)
        r = bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=s)
        a = random.uniform(0, 2 * pi)
        ca, sa = math.cos(a), math.sin(a)
        for v in r["verts"]:
            vx, vy = v.co.x * 1.6, v.co.y
            v.co = Vector((x + vx * ca - vy * sa, y + vx * sa + vy * ca, z + random.uniform(0, 0.006)))
    m = noisy(mat("leaf", rough=0.7), (0.32, 0.12, 0.03), (0.55, 0.3, 0.06), scale=3, detail=1)
    return obj_from_bm("leaves", bm, m)
