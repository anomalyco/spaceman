# The study at No. 12: upstairs at the front of a terraced house, at night, the blinds down.
# One scene, two uses: hero.py renders it in Cycles for the film's opening plates, and build.py bakes
# it into the live fallback (study.glb). Nothing outside is modelled: a streetlight down the road comes
# through the blinds, the landing light comes through the door, and the night is a dim glow past the slats.
# Run inside Blender. x runs east (the window wall), y north (the desk wall), z up; metres.
import math, random, os
import bpy, bmesh
from mathutils import Vector
from lib import *
import textures

RW, RD, RH = 3.6, 3.2, 2.55
T, TE = 0.12, 0.3  # wall thickness; the front (east) wall is solid brick, deeper
WY0, WY1, WZ0, WZ1 = 1.75, 2.85, 0.85, 2.12  # the window opening in the east wall
DX0, DX1, DH = 0.3, 1.15, 2.05  # the doorway in the south wall
pi = math.pi
rad = math.radians

# the desk, the monitor and its panel (the film's desktop is 16:10)
DESK = (2.0, RD - 0.4, 0.74)
MON = (2.0, RD - 0.27, 1.075)
PW, PH = 0.62, 0.3875
BODY = (0.646, 0.022, 0.4135)
CHAIR = (2.78, RD - 1.02, 140)  # x, y, turned (degrees): pushed back from the desk, turned side-on to the door
DESKTOP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "desktop.jpg")


def materials():
    tx = textures.ensure
    M = {}
    M["wall"] = pbr("plaster", tx("painted_plaster_wall"), 2.0, tint=(0.80, 0.78, 0.72), gain=1.15, rough=(0.75, 0.95), bump=0.15, contrast=-0.6)
    M["ceiling"] = pbr("ceiling", tx("plastered_wall_04"), 3.2, tint=(0.95, 0.94, 0.92), gain=1.2, rough=(0.85, 1.0), bump=0.08)
    M["floor"] = pbr("floor", tx("wood_floor"), 1.7, rough=(0.28, 0.55), bump=0.25, coat=0.25, coat_rough=0.18, rot=pi / 2)
    M["oak"] = pbr("oak", tx("oak_veneer_01"), 1.83, gain=0.85, rough=(0.3, 0.5), bump=0.1, coat=0.3, coat_rough=0.2)
    M["oakEdge"] = pbr("oakEdge", tx("oak_veneer_01"), 1.83, gain=0.5, rough=(0.3, 0.5), bump=0.05, coat=0.3, coat_rough=0.2, rot=pi / 2)
    M["rug"] = pbr("rug", tx("poly_wool_herringbone"), 0.27, tint=(0.62, 0.62, 0.64), rough=(0.85, 1.0), bump=0.6)
    M["fabric"] = pbr("fabric", tx("knitted_fleece"), 0.27, tint=(0.50, 0.52, 0.56), gain=0.85, rough=(0.85, 1.0), bump=0.5)
    # gloss-painted joinery (skirting, frames, the radiator) and satin-painted shelving
    M["paint"] = noisy(mat("paint", rough=0.22), (0.78, 0.77, 0.74), (0.81, 0.80, 0.77), scale=30, rough=(0.18, 0.28), bump=0.01)
    M["shelf"] = noisy(mat("shelf", rough=0.45), (0.74, 0.73, 0.70), (0.77, 0.76, 0.73), scale=12, rough=(0.4, 0.5))
    M["alu"] = noisy(mat("alu", rough=0.35, metal=1.0), (0.81, 0.82, 0.84), (0.83, 0.84, 0.86), scale=900, detail=2, rough=(0.33, 0.37))
    M["steel"] = mat("steel", (0.03, 0.03, 0.032), 0.45)
    M["black"] = mat("black", (0.012, 0.012, 0.014), 0.5)
    M["glossblack"] = mat("glossblack", (0.006, 0.006, 0.007), 0.08)
    bsdf(M["glossblack"]).inputs["Coat Weight"].default_value = 1.0
    bsdf(M["glossblack"]).inputs["Coat Roughness"].default_value = 0.02
    M["keys"] = mat("keys", (0.80, 0.80, 0.79), 0.42)
    M["white"] = mat("whiteplastic", (0.82, 0.82, 0.81), 0.3)
    M["binder"] = noisy(mat("binder", rough=0.5), (0.10, 0.13, 0.17), (0.115, 0.145, 0.185), scale=60, rough=(0.42, 0.55))
    M["binderOld"] = noisy(mat("binderOld", rough=0.6), (0.13, 0.155, 0.18), (0.145, 0.17, 0.195), scale=60, rough=(0.55, 0.65))
    M["label"] = mat("label", (0.74, 0.73, 0.70), 0.6)
    M["binderB"] = noisy(mat("binderB", rough=0.45), (0.05, 0.05, 0.055), (0.06, 0.06, 0.065), scale=60, rough=(0.38, 0.5))
    M["binderC"] = noisy(mat("binderC", rough=0.55), (0.16, 0.06, 0.05), (0.18, 0.07, 0.06), scale=60, rough=(0.5, 0.6))
    M["ink"] = mat("ink", (0.03, 0.03, 0.05), 0.5)
    M["labelOld"] = mat("labelOld", (0.70, 0.66, 0.55), 0.6)
    M["card"] = noisy(mat("card", rough=0.85), (0.40, 0.33, 0.24), (0.45, 0.37, 0.27), scale=80, detail=8, rough=(0.8, 0.9), bump=0.05)
    M["cardLid"] = noisy(mat("cardLid", rough=0.85), (0.37, 0.30, 0.22), (0.42, 0.35, 0.25), scale=80, detail=8, rough=(0.8, 0.9), bump=0.05)
    M["notebook"] = mat("notebook", (0.05, 0.08, 0.065), 0.55)
    M["paper"] = mat("paper", (0.80, 0.78, 0.72), 0.7)
    M["pen"] = mat("pen", (0.015, 0.015, 0.015), 0.25)
    M["felt"] = noisy(mat("felt", rough=1.0), (0.10, 0.10, 0.105), (0.12, 0.12, 0.125), scale=600, detail=4, rough=(0.95, 1.0), bump=0.15)
    M["cork"] = noisy(mat("cork", rough=0.9), (0.35, 0.24, 0.14), (0.45, 0.32, 0.19), scale=400, detail=6, rough=(0.85, 0.95), bump=0.3)
    M["ceramic"] = mat("ceramic", (0.78, 0.76, 0.70), 0.12)
    bsdf(M["ceramic"]).inputs["Coat Weight"].default_value = 0.6
    M["tea"] = mat("tea", (0.05, 0.025, 0.01), 0.05)
    M["slat"] = mat("slat", (0.47, 0.47, 0.455), 0.45)
    M["cord"] = mat("cord", (0.60, 0.59, 0.55), 0.9)
    M["linen"] = noisy(mat("linen", rough=0.9), (0.62, 0.60, 0.55), (0.66, 0.64, 0.59), scale=300, detail=3, rough=(0.85, 0.95), bump=0.1)
    M["brass"] = mat("brass", (0.75, 0.56, 0.30), 0.3, metal=1.0)
    M["cable"] = mat("cable", (0.75, 0.75, 0.74), 0.4)
    M["cableBlack"] = mat("cableBlack", (0.02, 0.02, 0.02), 0.45)
    M["frame"] = mat("frame", (0.02, 0.018, 0.016), 0.4)
    M["mount"] = mat("mount", (0.80, 0.78, 0.72), 0.85)
    M["map"] = contour_map()
    M["led"] = mat("led", (0, 0, 0), 0.3, emit=(1.0, 0.98, 0.95), strength=60.0)
    M["glass"] = glass()
    M["screen"] = screen_material()
    return M


def glass():
    """Window glass that lets the streetlight through: Fresnel-mixed transparent and glossy, so it casts
    no shadow (Cycles would need caustics to light a room through refractive glass)."""
    g = D.materials.new("Glass")
    g.use_nodes = True
    nt = g.node_tree
    nt.nodes.remove(nt.nodes["Principled BSDF"])
    out = nt.nodes["Material Output"]
    mix = nt.nodes.new("ShaderNodeMixShader")
    fr = nt.nodes.new("ShaderNodeFresnel")
    fr.inputs["IOR"].default_value = 1.5
    tr = nt.nodes.new("ShaderNodeBsdfTransparent")
    gl = nt.nodes.new("ShaderNodeBsdfGlossy")
    gl.inputs["Roughness"].default_value = 0.02
    nt.links.new(fr.outputs[0], mix.inputs[0])
    nt.links.new(tr.outputs[0], mix.inputs[1])
    nt.links.new(gl.outputs[0], mix.inputs[2])
    nt.links.new(mix.outputs[0], out.inputs["Surface"])
    return g


def contour_map():
    """A contour print: pale paper, brown contour lines, a blue lake. Procedural."""
    m = mat("map", (0.8, 0.78, 0.7), 0.8)
    mp = node(m, "ShaderNodeMapping")
    mp.inputs["Scale"].default_value = (6, 6, 6)
    link(m, texcoord(m, "Object"), mp.inputs["Vector"])
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 1.2
    n.inputs["Detail"].default_value = 3
    link(m, mp.outputs["Vector"], n.inputs["Vector"])
    s = node(m, "ShaderNodeMath")
    s.operation = "MULTIPLY"
    s.inputs[1].default_value = 22
    link(m, n.outputs["Fac"], s.inputs[0])
    fr = node(m, "ShaderNodeMath")
    fr.operation = "FRACT"
    link(m, s.outputs[0], fr.inputs[0])
    ln = node(m, "ShaderNodeMath")
    ln.operation = "LESS_THAN"
    ln.inputs[1].default_value = 0.14
    link(m, fr.outputs[0], ln.inputs[0])
    lake = node(m, "ShaderNodeMath")
    lake.operation = "LESS_THAN"
    lake.inputs[1].default_value = 0.36
    link(m, n.outputs["Fac"], lake.inputs[0])
    c1 = node(m, "ShaderNodeMix")
    c1.data_type = "RGBA"
    c1.inputs[6].default_value = (0.80, 0.77, 0.68, 1)
    c1.inputs[7].default_value = (0.30, 0.17, 0.08, 1)
    link(m, ln.outputs[0], c1.inputs[0])
    c2 = node(m, "ShaderNodeMix")
    c2.data_type = "RGBA"
    c2.inputs[7].default_value = (0.30, 0.45, 0.58, 1)
    link(m, c1.outputs[2], c2.inputs[6])
    link(m, lake.outputs[0], c2.inputs[0])
    link(m, c2.outputs[2], bsdf(m).inputs["Base Color"])
    return m


def screen_material():
    """The panel: the desktop as emission under glossy black glass (so asleep it reflects the room)."""
    m = mat("Screen", (0.006, 0.006, 0.007), 0.17)
    p = bsdf(m)
    p.inputs["IOR"].default_value = 1.5
    p.inputs["Coat Weight"].default_value = 1.0
    p.inputs["Coat Roughness"].default_value = 0.22
    # a faint unevenness in the coat (dust, a wipe), so the sheen from the window isn't a flat gradient
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 18.0
    n.inputs["Detail"].default_value = 3.0
    r = node(m, "ShaderNodeMapRange")
    r.inputs["To Min"].default_value = 0.17
    r.inputs["To Max"].default_value = 0.30
    link(m, n.outputs["Fac"], r.inputs["Value"])
    link(m, r.outputs["Result"], p.inputs["Coat Roughness"])
    t = node(m, "ShaderNodeTexImage")
    t.image = D.images.load(DESKTOP, check_existing=True)
    t.name = "desktop"
    link(m, texcoord(m, "UV"), t.inputs["Vector"])
    link(m, t.outputs["Color"], p.inputs["Emission Color"])
    p.inputs["Emission Strength"].default_value = 0.0
    return m


def set_screen(M, strength):
    bsdf(M["screen"]).inputs["Emission Strength"].default_value = strength


def bm_open(o, thick=0.002):
    """Removes a cylinder's caps and gives its wall a thickness (a lampshade, a mug)."""
    me = o.data
    bm = bmesh.new()
    bm.from_mesh(me)
    caps = [f for f in bm.faces if len(f.verts) > 4]
    bmesh.ops.delete(bm, geom=caps, context="FACES")
    bm.to_mesh(me)
    bm.free()
    s = o.modifiers.new("solid", "SOLIDIFY")
    s.thickness = thick


# ── the room ────────────────────────────────────────────────────────────
def shell(M, R):
    W, C, F, P = M["wall"], M["ceiling"], M["floor"], M["paint"]
    R.append(box("floor", (RW / 2, RD / 2, -0.05), (RW + 2 * T, RD + 2 * T, 0.1), F, 0))
    R.append(box("ceiling", (RW / 2, RD / 2, RH + 0.05), (RW + 2 * TE, RD + 2 * T, 0.1), C, 0))
    R.append(box("wallN", (RW / 2, RD + T / 2, RH / 2), (RW + 2 * T, T, RH), W, 0))
    R.append(box("wallW", (-T / 2, RD / 2, RH / 2), (T, RD, RH), W, 0))
    R.append(box("wallS1", ((DX0 - T) / 2, -T / 2, RH / 2), (DX0 + T, T, RH), W, 0))
    R.append(box("wallS2", ((DX1 + RW + TE) / 2, -T / 2, RH / 2), (RW + TE - DX1, T, RH), W, 0))
    R.append(box("wallS3", ((DX0 + DX1) / 2, -T / 2, (DH + RH) / 2), (DX1 - DX0, T, RH - DH), W, 0))
    ex = RW + TE / 2
    R.append(box("wallE1", (ex, RD / 2, WZ0 / 2), (TE, RD + 2 * T, WZ0), W, 0))
    R.append(box("wallE2", (ex, RD / 2, (WZ1 + RH) / 2), (TE, RD + 2 * T, RH - WZ1), W, 0))
    R.append(box("wallE3", (ex, (WY0 - T) / 2, (WZ0 + WZ1) / 2), (TE, WY0 + T, WZ1 - WZ0), W, 0))
    R.append(box("wallE4", (ex, (WY1 + RD + T) / 2, (WZ0 + WZ1) / 2), (TE, RD + T - WY1, WZ1 - WZ0), W, 0))
    # the landing past the door: enough walls and floor for its light to bounce in warm
    lx = (DX0 + DX1) / 2 + 0.4
    R.append(box("landingF", (lx, -1.0, -0.05), (2.4, 1.8, 0.1), F, 0))
    R.append(box("landingS", (lx, -1.95, RH / 2), (2.4, 0.1, RH), W, 0))
    R.append(box("landingE", (lx + 1.25, -1.0, RH / 2), (0.1, 1.8, RH), W, 0))
    R.append(box("landingW", (lx - 1.25, -1.0, RH / 2), (0.1, 1.8, RH), W, 0))
    R.append(box("landingC", (lx, -1.0, RH + 0.05), (2.4, 1.8, 0.1), C, 0))

    # mouldings: a tall torus skirting, a picture rail, a cove at the ceiling
    sk = [(0, 0), (0.024, 0), (0.024, 0.15)] + arc((0.006, 0.15), 0.018, 0, 90, 6)[1:] + [(0, 0.18)]
    rail = [(0, -0.03), (0.01, -0.026), (0.02, -0.014), (0.022, 0.0), (0.016, 0.012), (0.006, 0.016), (0, 0.018)]
    rr = 0.085
    cove = [(0, -rr - 0.018), (0.008, -rr - 0.018), (0.008, -rr)] + [(0.008 + (rr - 0.008) * (1 - math.cos(rad(a))), -rr + rr * math.sin(rad(a))) for a in range(10, 90, 10)] + [(rr, -0.008), (rr + 0.02, -0.008), (rr + 0.02, 0), (0, 0)]
    runs = [((0, RD, 0), (RW, RD, 0), (0, -1, 0)), ((0, 0, 0), (0, RD, 0), (1, 0, 0)), ((RW, 0, 0), (RW, RD, 0), (-1, 0, 0))]
    for i, (a, b, n) in enumerate(runs):
        R.append(prism("skirt%d" % i, sk, a, b, n, P))
        R.append(prism("cove%d" % i, cove, (a[0], a[1], RH), (b[0], b[1], RH), n, P))
        if i != 2:
            R.append(prism("rail%d" % i, rail, (a[0], a[1], 2.08), (b[0], b[1], 2.08), n, P))
    for k, (a, b) in enumerate((((0, 0, 0), (DX0 - 0.07, 0, 0)), ((DX1 + 0.07, 0, 0), (RW, 0, 0)))):
        R.append(prism("skirtS%d" % k, sk, a, b, (0, 1, 0), P))
    R.append(prism("coveS", cove, (0, 0, RH), (RW, 0, RH), (0, 1, 0), P))
    R.append(prism("railE1", rail, (RW, 0, 2.08), (RW, WY0 - 0.09, 2.08), (-1, 0, 0), P))
    R.append(prism("railE2", rail, (RW, WY1 + 0.09, 2.08), (RW, RD, 2.08), (-1, 0, 0), P))
    # the doorway's architrave and lining
    for n_, c, s in [("arL", (DX0 - 0.035, 0.012, DH / 2), (0.07, 0.024, DH + 0.07)), ("arR", (DX1 + 0.035, 0.012, DH / 2), (0.07, 0.024, DH + 0.07)),
                     ("arT", ((DX0 + DX1) / 2, 0.012, DH + 0.035), (DX1 - DX0 + 0.14, 0.024, 0.07)),
                     ("liL", (DX0 + 0.01, -T / 2, DH / 2), (0.02, T, DH)), ("liR", (DX1 - 0.01, -T / 2, DH / 2), (0.02, T, DH)), ("liT", ((DX0 + DX1) / 2, -T / 2, DH - 0.01), (DX1 - DX0, T, 0.02))]:
        R.append(box(n_, c, s, P, 0.003))
    # the door, open flat against the landing wall
    R.append(box("door", (DX0 + 0.02, -T - 0.39, 1.0), (0.04, 0.78, 1.98), P, 0.004))
    R.append(cyl("knob", (DX0 + 0.06, -T - 0.7, 1.0), 0.026, 0.04, M["brass"], 24, rot=(0, pi / 2, 0), bevel=0.006))

    # ceiling rose, pendant flex and a drum shade, off
    R.append(cyl("rose", (RW / 2, RD / 2, RH - 0.015), 0.12, 0.03, P, 48, bevel=0.008))
    R.append(cyl("flex", (RW / 2, RD / 2, RH - 0.27), 0.003, 0.5, M["cord"], 8))
    sh = cyl("shade", (RW / 2, RD / 2, RH - 0.6), 0.2, 0.22, M["linen"], 64)
    bm_open(sh)
    R.append(sh)
    R.append(cyl("bulb", (RW / 2, RD / 2, RH - 0.58), 0.03, 0.08, M["white"], 16))


def window(M, R):
    P, S = M["paint"], M["slat"]
    yc, zc = (WY0 + WY1) / 2, (WZ0 + WZ1) / 2
    w, h = WY1 - WY0, WZ1 - WZ0
    fx = RW + TE - 0.09  # the sash box, set back in the brick
    for n, c, s in [("boxL", (fx, WY0 + 0.03, zc), (0.1, 0.06, h)), ("boxR", (fx, WY1 - 0.03, zc), (0.1, 0.06, h)),
                    ("boxT", (fx, yc, WZ1 - 0.03), (0.1, w, 0.06)), ("boxB", (fx + 0.02, yc, WZ0 + 0.025), (0.14, w, 0.05))]:
        R.append(box(n, c, s, P, 0.004))
    # two sashes, the upper outside the lower, each two panes with a glazing bar
    sw = w - 0.12
    for k, (x, z0, z1) in enumerate([(fx + 0.025, zc - 0.015, WZ1 - 0.06), (fx - 0.025, WZ0 + 0.05, zc + 0.03)]):
        zm = (z0 + z1) / 2
        hh = z1 - z0
        for n, c, s in [("sL%d" % k, (x, WY0 + 0.085, zm), (0.04, 0.05, hh)), ("sR%d" % k, (x, WY1 - 0.085, zm), (0.04, 0.05, hh)),
                        ("sT%d" % k, (x, yc, z1 - 0.025), (0.04, sw, 0.05)), ("sB%d" % k, (x, yc, z0 + 0.03), (0.04, sw, 0.06)),
                        ("sBar%d" % k, (x, yc, zm), (0.03, 0.022, hh))]:
            R.append(box(n, c, s, P, 0.003))
        R.append(box("pane%d" % k, (x, yc, zm), (0.004, sw - 0.06, hh - 0.06), M["glass"], 0))
        R[-1].visible_shadow = False  # glass that casts no shadow: the streetlight comes through it
    # the window board (the inside sill) and the architrave around the opening
    R.append(box("board", (RW + TE / 2 - 0.08, yc, WZ0 - 0.012), (TE - 0.05 + 0.1, w + 0.12, 0.025), P, 0.004))
    for n, c, s in [("waL", (RW + 0.01, WY0 - 0.035, zc), (0.022, 0.07, h + 0.07)), ("waR", (RW + 0.01, WY1 + 0.035, zc), (0.022, 0.07, h + 0.07)),
                    ("waT", (RW + 0.01, yc, WZ1 + 0.035), (0.022, w + 0.14, 0.07))]:
        R.append(box(n, c, s, P, 0.003))
    # venetian blinds hung in the reveal, down, the slats tipped so the streetlight comes through in bands
    bx = RW + 0.07
    R.append(box("headrail", (bx, yc, WZ1 - 0.025), (0.035, w - 0.03, 0.03), S, 0.004))
    z = WZ1 - 0.06
    i = 0
    while z > WZ0 + 0.05:
        R.append(box("slat%d" % i, (bx, yc, z), (0.025, w - 0.04, 0.0014), S, 0.0, rot=(0, rad(SLAT), 0)))
        z -= 0.0215
        i += 1
    R.append(box("bottomrail", (bx, yc, z), (0.03, w - 0.04, 0.009), S, 0.002))
    for k in (-0.4, 0.0, 0.4):
        for side in (-1, 1):
            R.append(box("ladder", (bx + side * 0.012, yc + k, (z + WZ1 - 0.04) / 2), (0.0012, 0.004, WZ1 - 0.04 - z), M["cord"], 0))
    R.append(box("pull", (bx - 0.02, WY1 - 0.07, WZ1 - 0.45), (0.003, 0.003, 0.8), M["cord"], 0))
    R.append(cyl("acorn", (bx - 0.02, WY1 - 0.07, WZ1 - 0.86), 0.007, 0.03, M["white"], 12))
    # a double-panel radiator under the window, with its valve
    ry, rz = yc, 0.42
    R.append(box("radBack", (RW - 0.035, ry, rz), (0.025, 1.0, 0.5), P, 0.006))
    R.append(box("radFront", (RW - 0.085, ry, rz), (0.025, 1.0, 0.5), P, 0.006))
    for i in range(30):
        R.append(box("radRib%d" % i, (RW - 0.099, ry - 0.48 + i * 0.0331, rz), (0.004, 0.016, 0.46), P, 0.0015))
    R.append(box("radTop", (RW - 0.06, ry, rz + 0.255), (0.07, 1.0, 0.006), P, 0.002))
    R.append(cyl("valve", (RW - 0.06, ry - 0.54, 0.16), 0.022, 0.07, M["white"], 24, bevel=0.004))
    R.append(cyl("pipe", (RW - 0.06, ry - 0.54, 0.06), 0.008, 0.12, M["brass"], 12))


SLAT = 18  # degrees; tips the slats' outside edge down, so about 40% of the streetlight comes through


def bookcase(M, R):
    P = M["shelf"]
    sx, sy = 0.66, RD - 0.17
    for n, c, s in [("bcL", (sx - 0.4, sy, 0.95), (0.022, 0.32, 1.9)), ("bcR", (sx + 0.4, sy, 0.95), (0.022, 0.32, 1.9)), ("bcBack", (sx, sy + 0.155, 0.95), (0.8, 0.01, 1.9)),
                    ("bcPlinth", (sx, sy - 0.14, 0.04), (0.78, 0.02, 0.08))]:
        R.append(box(n, c, s, P, 0.003))
    for k, hz in enumerate([0.08, 0.45, 0.82, 1.19, 1.56, 1.89]):
        R.append(box("bcS%d" % k, (sx, sy, hz), (0.78, 0.3, 0.022), P, 0.003))
    rng = random.Random(14)
    for row, hz in enumerate([0.831, 1.201]):
        x = sx - 0.36 + 0.004
        for i in range(11):
            wdt = rng.choice((0.058, 0.058, 0.052, 0.066))
            if x + wdt > sx + 0.37:
                break
            xc = x + wdt / 2
            dy = rng.choice((0.0, 0.0, 0.0, -0.012, -0.022, 0.006))
            mb = M["binderOld"] if row == 0 else M["binder"]
            mb = {0: mb, 1: mb, 2: mb, 3: M["binderB"], 4: M["binderC"] if row == 1 else mb}[rng.randrange(5)]
            R.append(slab("binder%d_%d" % (row, i), (xc, sy + 0.005 + dy, hz + 0.011 + 0.155), wdt - 0.003, 0.31, 0.28, 0.008, mb, axis="Y"))
            R.append(box("label%d_%d" % (row, i), (xc + rng.uniform(-0.002, 0.002), sy - 0.1355 + dy, hz + 0.011 + 0.215 + rng.uniform(-0.006, 0.006)),
                         (0.034, 0.002, 0.075), M["labelOld"] if row == 0 else M["label"], 0.0, rot=(0, rad(rng.uniform(-2.5, 2.5)), 0)))
            # the label's handwriting: two or three short strokes of ink
            for k in range(rng.choice((1, 2, 2, 3))):
                R.append(box("ink%d_%d_%d" % (row, i, k), (xc + rng.uniform(-0.004, 0.004), sy - 0.137 + dy, hz + 0.011 + 0.235 - k * 0.016), (rng.uniform(0.012, 0.026), 0.0005, 0.0035), M["ink"], 0))
            R.append(cyl("ring%d_%d" % (row, i), (xc, sy - 0.1355 + dy, hz + 0.011 + 0.075), 0.011, 0.003, M["black"], 20, rot=(pi / 2, 0, 0)))
            x += wdt + 0.008
    for i in range(3):
        R.append(box("arch%d" % i, (sx - 0.25 + i * 0.25, sy, 1.571 + 0.12), (0.24, 0.28, 0.24), M["card"], 0.004))
        R.append(box("archLid%d" % i, (sx - 0.25 + i * 0.25, sy, 1.571 + 0.235), (0.245, 0.285, 0.03), M["cardLid"], 0.003))
        R.append(slab("archHole%d" % i, (sx - 0.25 + i * 0.25, sy - 0.141, 1.571 + 0.17), 0.08, 0.025, 0.002, 0.012, M["black"]))
    for i in range(2):
        R.append(box("archLow%d" % i, (sx - 0.19 + i * 0.38, sy, 0.091 + 0.15), (0.36, 0.28, 0.3), M["card"], 0.004))
        R.append(box("archLowLid%d" % i, (sx - 0.19 + i * 0.38, sy, 0.091 + 0.295), (0.365, 0.285, 0.03), M["cardLid"], 0.003))
    # a shelf of paperbacks and manuals, muted spines, one leaning
    x = sx - 0.36
    cols = [(0.20, 0.17, 0.13), (0.12, 0.14, 0.15), (0.32, 0.30, 0.26), (0.16, 0.10, 0.08), (0.42, 0.40, 0.34), (0.10, 0.11, 0.10), (0.25, 0.22, 0.15)]
    i = 0
    while x < sx + 0.3:
        bw = rng.uniform(0.018, 0.04)
        bh = rng.uniform(0.18, 0.24)
        bd = rng.uniform(0.13, 0.17)
        bk = noisy(mat("book%d" % i, rough=0.7), cols[i % 7], tuple(c * 1.1 for c in cols[i % 7]), scale=40, rough=(0.6, 0.8))
        R.append(box("book%d" % i, (x + bw / 2, sy - 0.14 + bd / 2 + 0.005, 0.461 + bh / 2), (bw - 0.002, bd, bh), bk, 0.002))
        x += bw
        i += 1
    # a hardback leaning on the rest: boards, a cloth spine, the page block inset between them
    lean = (0, rad(-18), 0)
    from mathutils import Matrix, Euler
    rm = Euler(lean).to_matrix()
    c0 = Vector((x + 0.05, sy - 0.07, 0.461 + 0.1))
    for k, (o, sz, m_) in enumerate((((-0.0115, 0, 0), (0.002, 0.15, 0.21), M["binderC"]), ((0.0115, 0, 0), (0.002, 0.15, 0.21), M["binderC"]),
                                       ((0, -0.074, 0), (0.025, 0.003, 0.21), M["binderC"]), ((0, 0.0015, 0), (0.021, 0.145, 0.204), M["paper"]))):
        R.append(box("bookLean%d" % k, c0 + rm @ Vector(o), sz, m_, 0.0008, rot=lean))


def wall_things(M, R):
    # the contour print, framed, right of the desk
    fx, fz = 3.08, 1.62
    R.append(box("frame", (fx, RD - 0.012, fz), (0.40, 0.022, 0.31), M["frame"], 0.003))
    R.append(box("mount", (fx, RD - 0.0235, fz), (0.36, 0.002, 0.27), M["mount"], 0))
    R.append(box("print", (fx, RD - 0.025, fz), (0.29, 0.002, 0.2), M["map"], 0))
    R.append(box("frameGlass", (fx, RD - 0.028, fz), (0.37, 0.002, 0.28), M["glass"], 0))
    R[-1].visible_shadow = False
    # a double socket under the desk, two plugs in
    R.append(box("socket", (2.55, RD - 0.006, 0.3), (0.146, 0.012, 0.086), M["white"], 0.002))
    for k, xo in enumerate((-0.036, 0.036)):
        R.append(box("plug%d" % k, (2.55 + xo, RD - 0.03, 0.3), (0.045, 0.04, 0.05), M["white"], 0.004))
    R.append(box("switch", (DX1 + 0.18, 0.006, 1.2), (0.086, 0.012, 0.086), M["white"], 0.002))


def desk(M, N):
    dx, dy, top = DESK
    N.append(slab("desktop", (dx, dy, top - 0.0125), 1.6, 0.75, 0.025, 0.01, M["oak"], axis="Z", bevel=0.003))
    N.append(box("edgeband", (dx, dy - 0.375 - 0.0004, top - 0.0125), (1.58, 0.001, 0.0235), M["oakEdge"], 0))
    for s_ in (-1, 1):
        N.append(box("edgebandS", (dx + s_ * (0.8 + 0.0004), dy, top - 0.0125), (0.001, 0.73, 0.0235), M["oakEdge"], 0))
    st = M["steel"]
    for lx in (dx - 0.74, dx + 0.74):
        for ly in (dy - 0.32, dy + 0.32):
            N.append(box("leg", (lx, ly, (top - 0.025) / 2), (0.04, 0.04, top - 0.025), st, 0.004))
            N.append(cyl("foot", (lx, ly, 0.004), 0.016, 0.008, M["black"], 16))
        N.append(box("rail", (lx, dy, top - 0.06), (0.04, 0.64, 0.05), st, 0.004))
    N.append(box("railB", (dx, dy + 0.32, top - 0.06), (1.44, 0.03, 0.05), st, 0.004))
    N.append(slab("pad", (dx + 0.06, RD - 0.6, top + 0.0015), 0.8, 0.34, 0.003, 0.012, M["felt"], axis="Z"))
    mon(M, N)
    keyboard(M, N, dx - 0.04, RD - 0.6, top + 0.003)
    N.append(slab("trackpad", (dx + 0.262, RD - 0.585, top + 0.0075), 0.16, 0.115, 0.009, 0.01, M["white"], axis="Z", bevel=0.002, rot=(0, 0, rad(-4))))
    # the Mac mini to the left of the stand, its light on
    N.append(slab("mini", (dx - 0.45, RD - 0.26, top + 0.019), 0.197, 0.197, 0.036, 0.022, M["alu"], axis="Z", bevel=0.003))
    N.append(slab("miniFoot", (dx - 0.45, RD - 0.26, top + 0.0006), 0.17, 0.17, 0.0012, 0.02, M["black"], axis="Z"))
    N.append(cyl("miniLED", (dx - 0.45 + 0.07, RD - 0.26 - 0.0986, top + 0.012), 0.0013, 0.001, M["led"], 12, rot=(pi / 2, 0, 0)))
    # three pens, parallel; a notebook squared to the edge, its band; the label maker; a mug on a coaster
    for i, (ox, oy, a) in enumerate(((0.0, 0.0, 4), (0.026, 0.012, -9), (0.075, -0.03, 31))):
        pen(M, N, Vector((dx + 0.56 + ox, RD - 0.62 + oy, top + 0.0045)), rad(a), i == 1)
    # a letter, opened and folded in thirds, left by the keyboard
    for k, (oz, tilt) in enumerate(((0.0006, 0), (0.0035, 2.2))):
        N.append(box("letter%d" % k, (dx - 0.3, RD - 0.66, top + 0.003 + oz), (0.21, 0.099, 0.0004), M["paper"], 0, rot=(rad(tilt), 0, rad(-7))))
    N.append(slab("notebook", (dx + 0.62, RD - 0.32, top + 0.0075), 0.13, 0.21, 0.015, 0.006, M["notebook"], axis="Z"))
    N.append(box("pages", (dx + 0.6215, RD - 0.32, top + 0.0075), (0.125, 0.2, 0.012), M["paper"], 0.001))
    N.append(box("band", (dx + 0.67, RD - 0.32, top + 0.0075), (0.006, 0.212, 0.0162), M["black"], 0.001))
    N.append(slab("labelmaker", (dx - 0.56, RD - 0.6, top + 0.018), 0.065, 0.13, 0.036, 0.012, M["black"], axis="Z", bevel=0.003))
    N.append(cyl("coaster", (dx + 0.45, RD - 0.48, top + 0.003), 0.048, 0.006, M["cork"], 40, bevel=0.0015))
    mug = cyl("mug", (dx + 0.45, RD - 0.48, top + 0.006 + 0.048), 0.041, 0.095, M["ceramic"], 48)
    bm_open(mug, 0.004)
    N.append(mug)
    N.append(cyl("mugBase", (dx + 0.45, RD - 0.48, top + 0.008), 0.039, 0.004, M["ceramic"], 48))
    N.append(cyl("tea", (dx + 0.45, RD - 0.48, top + 0.06), 0.0365, 0.002, M["tea"], 48))
    bpy.ops.mesh.primitive_torus_add(major_radius=0.026, minor_radius=0.0055, major_segments=32, minor_segments=12, location=(dx + 0.45 + 0.045, RD - 0.48, top + 0.056), rotation=(pi / 2, 0, 0))
    h = bpy.context.active_object
    h.name = "handle"
    h.scale = (0.8, 1.0, 1.0)
    h.data.materials.append(M["ceramic"])
    for f in h.data.polygons:
        f.use_smooth = True
    N.append(h)
    lamp(M, N, dx - 0.64, RD - 0.15, top)
    cables(M, N)


def pen(M, N, p, a, clip):
    """A capped pen lying flat: barrel, a tapered tip with a nib, and a clip on one."""
    d = Vector((-math.sin(a), math.cos(a), 0))
    rot = (pi / 2, 0, a)
    N.append(cyl("pen", p, 0.0045, 0.118, M["pen"], 20, rot=rot, bevel=0.0008))
    tip = cyl("penTip", p - d * 0.067, 0.0045, 0.016, M["pen"], 20, rot=rot, r2=0.0016)
    N.append(tip)
    N.append(cyl("penNib", p - d * 0.0765, 0.0012, 0.004, M["alu"], 12, rot=rot))
    N.append(cyl("penEnd", p + d * 0.0605, 0.0042, 0.003, M["alu"], 20, rot=rot, bevel=0.0008))
    if clip:
        N.append(box("penClip", p + d * 0.04 + Vector((0, 0, 0.0048)), (0.0022, 0.035, 0.0012), M["alu"], 0.0005, rot=(0, 0, a)))


def mon(M, N):
    mx, my, mz = MON
    bw, bd, bh = BODY
    N.append(slab("monBody", (mx, my + 0.0005, mz), bw, bh, bd - 0.001, 0.012, M["alu"], axis="Y", bevel=0.002))
    N.append(slab("monFront", (mx, my - bd / 2 + 0.0004, mz), bw - 0.002, bh - 0.002, 0.0008, 0.011, M["glossblack"], axis="Y"))
    # the stand: a foot, and an arm leaning back to a hinge behind the panel
    top = DESK[2]
    N.append(slab("standFoot", (mx, my + 0.04, top + 0.004), 0.2, 0.2, 0.008, 0.03, M["alu"], axis="Z", bevel=0.002))
    a = rad(12)
    L = 0.33
    N.append(slab("standArm", (mx, my + 0.11 - math.sin(a) * L / 2, top + 0.006 + math.cos(a) * L / 2), 0.15, L, 0.012, 0.004, M["alu"], axis="Y", rot=(-a, 0, 0)))
    N.append(cyl("hinge", (mx, my + 0.05, mz - 0.03), 0.012, 0.15, M["alu"], 24, rot=(0, pi / 2, 0)))
    N.append(box("monBack", (mx, my + bd / 2 + 0.012, mz - 0.03), (0.17, 0.025, 0.07), M["alu"], 0.004))


def keyboard(M, N, kx, ky, z):
    N.append(slab("kb", (kx, ky, z + 0.004), 0.279, 0.115, 0.007, 0.006, M["alu"], axis="Z", bevel=0.0015, rot=(rad(1.8), 0, 0)))
    rows = [([1] * 14, 0.5), ([1] * 13 + [1.5], 1), ([1.5] + [1] * 13, 1), ([1.75] + [1] * 11 + [1.75], 1), ([2.25] + [1] * 10 + [2.25], 1), ([1, 1, 1, 1.25, 5.0, 1.25, 1, 1, 1, 1], 1)]
    u = 0.0188
    W = 14.5 * u
    y = ky + 0.05
    for widths, hgt in rows:
        dyr = u * hgt
        x = kx - W / 2
        k = 14.5 / sum(widths)
        for wu in widths:
            ww = wu * u * k
            N.append(box("key", (x + ww / 2, y - dyr / 2, z + 0.0092 + (y - ky) * 0.03), (ww - 0.0028, dyr - 0.0028, 0.003), M["keys"], 0.0011, 2))
            x += ww
        y -= dyr


def lamp(M, N, lx, ly, top):
    b = M["black"]
    N.append(cyl("lampBase", (lx, ly, top + 0.012), 0.075, 0.024, b, 48, bevel=0.006))
    p0 = Vector((lx, ly, top + 0.03))
    p1 = p0 + Vector((0, -0.06, 0.38))
    p2 = p1 + Vector((0, -0.24, 0.05))
    for n, a, c in (("lampArm1", p0, p1), ("lampArm2", p1, p2)):
        d = c - a
        for off, r, s in ((0, 0.0055, 1.0), (0.016, 0.0035, 0.9)):
            o = cyl(n, (a + c) / 2 + Vector((off, 0, 0)), r, d.length * s, b, 12)
            o.rotation_euler = d.to_track_quat("Z", "Y").to_euler()
            N.append(o)
    for p in (p0, p1, p2):
        N.append(cyl("joint", p, 0.012, 0.03, b, 16, rot=(0, pi / 2, 0)))
    head = cyl("lampHead", p2 + Vector((0, -0.03, -0.05)), 0.055, 0.12, b, 40, r2=0.02)
    head.rotation_euler = (rad(-30), 0, 0)
    N.append(head)


def cables(M, N):
    dx, dy, top = DESK
    mx, my, mz = MON
    c = M["cable"]
    N.append(cable("monCable", [(mx, my + 0.12, top + 0.02), (mx + 0.05, RD - 0.03, top + 0.006), (mx + 0.25, RD - 0.03, top - 0.12), (2.586, RD - 0.05, 0.36), (2.586, RD - 0.05, 0.31)], 0.0028, c))
    N.append(cable("miniCable", [(dx - 0.45, RD - 0.16, top + 0.012), (dx - 0.3, RD - 0.03, top + 0.004), (dx - 0.1, RD - 0.04, top - 0.2), (2.514, RD - 0.05, 0.4), (2.514, RD - 0.05, 0.31)], 0.0028, c))
    N.append(cable("lampCable", [(dx - 0.64, RD - 0.08, top + 0.01), (dx - 0.7, RD - 0.03, top - 0.02), (dx - 0.75, RD - 0.03, 0.2), (dx - 0.4, RD - 0.04, 0.012), (2.4, RD - 0.04, 0.012)], 0.002, M["cableBlack"]))


def chair(M, N):
    n0 = len(N)
    chair_parts(M, N)
    cx, cy, turn = CHAIR
    from mathutils import Matrix
    R = Matrix.Translation((cx, cy, 0)) @ Matrix.Rotation(rad(turn), 4, "Z") @ Matrix.Translation((-cx, -cy, 0))
    for o in N[n0:]:
        o.matrix_basis = R @ o.matrix_basis


def chair_parts(M, N):
    cx, cy, _ = CHAIR
    F, st = M["fabric"], M["steel"]
    seat = sbox("seat", (cx, cy, 0.475), (0.5, 0.48, 0.075), F, 0.035, 6, sub=2)
    cast(seat, 0.22)
    N.append(seat)
    back = sbox("back", (cx, cy - 0.27, 0.88), (0.46, 0.065, 0.56), F, 0.03, 6, rot=(rad(-9), 0, 0), sub=2)
    cast(back, 0.18)
    bd = back.modifiers.new("bend", "SIMPLE_DEFORM")
    bd.deform_method = "BEND"
    bd.deform_axis = "Z"
    bd.angle = rad(-28)
    back.modifiers.move(len(back.modifiers) - 1, 0)
    N.append(back)
    N.append(box("backFrame", (cx, cy - 0.31, 0.66), (0.06, 0.03, 0.42), st, 0.01, 2, rot=(rad(-11), 0, 0)))
    for s in (-1, 1):
        N.append(box("armPost", (cx + s * 0.27, cy - 0.03, 0.57), (0.03, 0.04, 0.2), st, 0.008))
        N.append(slab("armPad", (cx + s * 0.27, cy - 0.01, 0.675), 0.065, 0.25, 0.025, 0.02, M["black"], axis="Z", bevel=0.004))
    N.append(box("seatPan", (cx, cy, 0.42), (0.24, 0.24, 0.04), st, 0.008))
    N.append(cyl("gas", (cx, cy, 0.26), 0.024, 0.3, M["alu"], 24))
    N.append(cyl("gasShroud", (cx, cy, 0.19), 0.03, 0.16, st, 24))
    N.append(cyl("hub", (cx, cy, 0.1), 0.045, 0.05, st, 24, bevel=0.006))
    for k in range(5):
        a = rad(90 + k * 72 + 18)
        N.append(box("leg%d" % k, (cx + math.cos(a) * 0.17, cy + math.sin(a) * 0.17, 0.088), (0.31, 0.045, 0.03), st, 0.01, 2, rot=(0, rad(-5), a)))
        N.append(cyl("caster%d" % k, (cx + math.cos(a) * 0.31, cy + math.sin(a) * 0.31, 0.03), 0.028, 0.026, M["black"], 20, rot=(pi / 2, 0, a)))


def screen(M):
    mx, my, mz = MON
    bm = bmesh.new()
    bm.loops.layers.uv.new("UVMap")
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=0.5, calc_uvs=True)
    for v in bm.verts:
        v.co = Vector((v.co.x * PW, v.co.y * PH, 0))
    s = obj_from_bm("Screen", bm, M["screen"])
    s.location = (mx, my - BODY[1] / 2 - 0.0002, mz + 0.004)
    s.rotation_euler = (pi / 2, 0, 0)
    return s


def panel_corners(s):
    """The panel's corners in world space: [bl, br, tr, tl]."""
    return [s.matrix_world @ Vector(c) for c in [(-PW / 2, -PH / 2, 0), (PW / 2, -PH / 2, 0), (PW / 2, PH / 2, 0), (-PW / 2, PH / 2, 0)]]


# ── light ───────────────────────────────────────────────────────────────
def lights():
    L = {}
    # a streetlight down the road to the south-east: through the blinds in bands across the desk wall
    L["street"] = light("street", "SPOT", (RW + 5.2, WY0 - 5.8, 3.2), STREET_W, (0.78, 0.86, 1.0), 0.06, target=(RW, (WY0 + WY1) / 2 + 0.25, 1.75), spot=11, blend=0.9)
    L["street"].data.shadow_soft_size = 0.11
    # the landing light, warm, through the open door
    lx = (DX0 + DX1) / 2 + 0.3
    L["landing"] = light("landing", "AREA", (lx, -1.0, RH - 0.05), LANDING_W, (1.0, 0.70, 0.45), (0.3, 0.3), target=(lx, -1.0, 0))
    # the night past the slats: a faint glow
    sky = mat("Night", (0, 0, 0), 1.0, emit=(0.06, 0.07, 0.09), strength=0.22)
    L["night"] = box("Night", (RW + 6, (WY0 + WY1) / 2, 2.0), (0.1, 16, 10), sky, 0)
    L["bg"] = world((0.002, 0.0025, 0.004), 1.0)
    return L


STREET_W = 9000
LANDING_W = 140


def build():
    M = materials()
    NEAR, ROOM = [], []
    shell(M, ROOM)
    window(M, ROOM)
    bookcase(M, ROOM)
    wall_things(M, ROOM)
    ROOM.append(slab("rug", (2.0, RD - 1.05, 0.004), 1.7, 1.25, 0.008, 0.01, M["rug"], axis="Z", bevel=0.003))
    desk(M, NEAR)
    chair(M, NEAR)
    scr = screen(M)
    return dict(M=M, NEAR=NEAR, ROOM=ROOM, screen=scr, lights=lights())
