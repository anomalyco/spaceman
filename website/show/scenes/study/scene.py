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
CLUES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "clues")  # clues.py draws these


def materials():
    tx = textures.ensure
    M = {}
    M["wall"] = pbr("plaster", tx("painted_plaster_wall"), 2.0, tint=(0.80, 0.78, 0.72), gain=1.15, rough=(0.75, 0.95), bump=0.15, contrast=-0.6)
    M["ceiling"] = pbr("ceiling", tx("plastered_wall_04"), 3.2, tint=(0.95, 0.94, 0.92), gain=1.2, rough=(0.85, 1.0), bump=0.08)
    M["floor"] = pbr("floor", tx("wood_floor"), 1.7, rough=(0.28, 0.55), bump=0.25, coat=0.25, coat_rough=0.18, rot=pi / 2)
    # the desk: the oak scan stained to walnut, its grain kept, under an oiled satin coat
    M["oak"] = pbr("oak", tx("oak_veneer_01"), 1.83, tint=(0.56, 0.36, 0.25), gain=0.62, rough=(0.28, 0.46), bump=0.1, coat=0.35, coat_rough=0.16, contrast=0.12)
    M["oakEdge"] = pbr("oakEdge", tx("oak_veneer_01"), 1.83, tint=(0.56, 0.36, 0.25), gain=0.4, rough=(0.3, 0.5), bump=0.05, coat=0.35, coat_rough=0.16, rot=pi / 2)
    M["oakFrame"] = pbr("oakFrame", tx("oak_veneer_01"), 1.83, tint=(0.56, 0.36, 0.25), gain=0.5, rough=(0.35, 0.5), bump=0.05)
    M["rug"] = pbr("rug", tx("poly_wool_herringbone"), 0.27, tint=(0.62, 0.62, 0.64), rough=(0.85, 1.0), bump=0.6)
    M["fabric"] = pbr("fabric", tx("knitted_fleece"), 0.27, tint=(0.50, 0.52, 0.56), gain=1.0, rough=(0.85, 1.0), bump=0.5)
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
    # the binders' and hardbacks' book cloth: one linen scan, recoloured to each cover
    for k, c in CLOTH.items():
        M["cloth_" + k] = cloth("cloth_" + k, c)
    M["binderC"] = M["cloth_oxblood"]
    for k in range(20):
        M["label%02d" % k] = imgmat("label%02d" % k, os.path.join(CLUES, "label%02d.png" % k), 0.7, val=0.85)
    for k in range(2):
        M["spine%d" % k] = imgmat("spine%d" % k, os.path.join(CLUES, "spine%d.png" % k), 0.6)
    M["sleeve"] = sleeve()
    M["nickel"] = noisy(mat("nickel", rough=0.3, metal=1.0), (0.60, 0.58, 0.54), (0.64, 0.62, 0.58), scale=1500, detail=2, rough=(0.26, 0.36), mscale=(1, 1, 30))
    M["hole"] = mat("hole", (0.006, 0.006, 0.007), 0.9)
    M["ink"] = mat("ink", (0.03, 0.03, 0.05), 0.5)
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
    M["frame"] = noisy(mat("frame", rough=0.4, metal=0.7), (0.035, 0.033, 0.031), (0.042, 0.04, 0.038), scale=800, detail=2, rough=(0.34, 0.44))
    M["mount"] = noisy(mat("mount", rough=0.9), (0.80, 0.79, 0.75), (0.83, 0.82, 0.78), scale=500, detail=4, rough=(0.85, 0.95), bump=0.03)
    M["elevation"] = imgmat("elevation", os.path.join(CLUES, "elevation.png"), 0.85, val=0.92)
    M["led"] = mat("led", (0, 0, 0), 0.3, emit=(1.0, 0.98, 0.95), strength=60.0)
    M["matte"] = noisy(mat("matte", rough=0.62), (0.014, 0.014, 0.015), (0.02, 0.02, 0.021), scale=300, detail=3, rough=(0.55, 0.7))
    M["bulb"] = mat("bulb", (0, 0, 0), 0.3, emit=(1.0, 0.80, 0.55), strength=40.0)
    M["ruler"] = noisy(mat("ruler", rough=0.3, metal=1.0), (0.62, 0.63, 0.64), (0.66, 0.67, 0.68), scale=2000, detail=2, rough=(0.25, 0.35))
    # the drill's housing: a slightly worn, satin yellow plastic, and a soft-touch grey overmould
    M["drill"] = noisy(mat("drill", rough=0.42), (0.55, 0.38, 0.035), (0.60, 0.42, 0.04), scale=220, detail=4, rough=(0.34, 0.46), bump=0.04)
    bsdf(M["drill"]).inputs["Coat Weight"].default_value = 0.25
    bsdf(M["drill"]).inputs["Coat Roughness"].default_value = 0.3
    M["rubber"] = noisy(mat("rubber", rough=0.75), (0.028, 0.028, 0.03), (0.034, 0.034, 0.036), scale=900, detail=3, rough=(0.62, 0.78), bump=0.08)
    M["chrome"] = mat("chrome", (0.8, 0.8, 0.82), 0.12, metal=1.0)
    M["tin"] = noisy(mat("tin", rough=0.35), (0.25, 0.12, 0.36), (0.28, 0.14, 0.40), scale=40, rough=(0.3, 0.4))
    M["tinEnd"] = mat("tinEnd", (0.75, 0.75, 0.77), 0.25, metal=1.0)
    M["plastic"] = polythene()
    M["core"] = noisy(mat("core", rough=0.85), (0.45, 0.36, 0.25), (0.5, 0.4, 0.28), scale=60, rough=(0.8, 0.9))
    M["bandBlue"] = mat("bandBlue", (0.03, 0.09, 0.30), 0.45)
    M["bandWhite"] = mat("bandWhite", (0.78, 0.78, 0.76), 0.5)
    for k in ("sticky", "card4", "card5", "card6", "card7"):
        M[k] = imgmat(k, os.path.join(CLUES, k + ".png"), 0.82)
    M["legalpad"] = imgmat("legalpad", os.path.join(CLUES, "legalpad.png"), 0.85, sat=1.5, val=0.62)
    M["padBand"] = mat("padBand", (0.22, 0.05, 0.05), 0.6)
    M["padBack"] = mat("padBack", (0.40, 0.33, 0.24), 0.85)
    M["pin"] = mat("pin", (0.60, 0.45, 0.22), 0.3, metal=1.0)
    M["glass"] = glass()
    M["screen"] = screen_material()
    return M


def imgmat(name, path, rough=0.8, sat=1.0, val=1.0):
    """Paper with a picture on it (clues.py), mapped by the plane's UVs. sat/val keep a sheet that sits
    under the lamp from washing out to white."""
    m = mat(name, (0.8, 0.8, 0.8), rough)
    t = node(m, "ShaderNodeTexImage")
    t.image = D.images.load(path, check_existing=True)
    link(m, texcoord(m, "UV"), t.inputs["Vector"])
    hsv = node(m, "ShaderNodeHueSaturation")
    hsv.inputs["Saturation"].default_value = sat
    hsv.inputs["Value"].default_value = val
    link(m, t.outputs["Color"], hsv.inputs["Color"])
    link(m, hsv.outputs["Color"], bsdf(m).inputs["Base Color"])
    return m


def polythene():
    """Heavy clear sheeting on a roll: a milky, slightly rough transmissive film."""
    m = mat("plastic", (0.86, 0.88, 0.90), 0.22)
    p = bsdf(m)
    p.inputs["Transmission Weight"].default_value = 0.85
    p.inputs["IOR"].default_value = 1.45
    p.inputs["Coat Weight"].default_value = 0.4
    p.inputs["Coat Roughness"].default_value = 0.08
    return m


def decal(name, center, w, h, m, rot=(0, 0, 0)):
    """A flat UV-mapped sheet (paper, a card, a note), w × h, its picture upright along +y before rot."""
    bm = bmesh.new()
    bm.loops.layers.uv.new("UVMap")
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=0.5, calc_uvs=True)
    for v in bm.verts:
        v.co = Vector((v.co.x * w, v.co.y * h, 0))
    o = obj_from_bm(name, bm, m)
    o.location = center
    o.rotation_euler = rot
    return o


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


def cloth(name, colour, tile=0.2):
    """Book cloth: the linen scan's weave (its brightness, roughness and height) under one cover colour."""
    maps = textures.ensure("rough_linen")
    m = mat(name, colour, 0.7)
    mp = node(m, "ShaderNodeMapping")
    mp.inputs["Scale"].default_value = (1 / tile, 1 / tile, 1 / tile)
    link(m, texcoord(m, "Object"), mp.inputs["Vector"])

    def img(path, colour_space):
        t = node(m, "ShaderNodeTexImage")
        t.image = D.images.load(path, check_existing=True)
        t.image.colorspace_settings.name = colour_space
        t.projection = "BOX"
        t.projection_blend = 0.2
        link(m, mp.outputs["Vector"], t.inputs["Vector"])
        return t

    bw = node(m, "ShaderNodeRGBToBW")
    link(m, img(maps["diff"], "sRGB").outputs["Color"], bw.inputs["Color"])
    lift = node(m, "ShaderNodeMapRange")
    lift.inputs["From Min"].default_value, lift.inputs["From Max"].default_value = 0.25, 0.55  # the scan's linear range
    lift.inputs["To Min"].default_value, lift.inputs["To Max"].default_value = 0.7, 1.25
    link(m, bw.outputs["Val"], lift.inputs["Value"])
    sc_ = node(m, "ShaderNodeVectorMath")
    sc_.operation = "SCALE"
    sc_.inputs[0].default_value = colour
    link(m, lift.outputs["Result"], sc_.inputs["Scale"])
    link(m, sc_.outputs[0], bsdf(m).inputs["Base Color"])
    r = node(m, "ShaderNodeMapRange")
    r.inputs["To Min"].default_value, r.inputs["To Max"].default_value = 0.62, 0.9
    link(m, img(maps["rough"], "Non-Color").outputs["Color"], r.inputs["Value"])
    link(m, r.outputs["Result"], bsdf(m).inputs["Roughness"])
    b = node(m, "ShaderNodeBump")
    b.inputs["Strength"].default_value = 0.35
    b.inputs["Distance"].default_value = 0.0015
    link(m, img(maps["disp"], "Non-Color").outputs["Color"], b.inputs["Height"])
    link(m, b.outputs["Normal"], bsdf(m).inputs["Normal"])
    return m


# cover colours: charcoal, navy and forest linen, kraft board, slate, oxblood
CLOTH = {"charcoal": (0.042, 0.043, 0.046), "navy": (0.022, 0.034, 0.07), "forest": (0.026, 0.052, 0.038),
         "kraft": (0.30, 0.215, 0.13), "slate": (0.10, 0.108, 0.115), "oxblood": (0.11, 0.035, 0.03)}


def sleeve():
    """The clear PVC pocket over a spine label: a faint, slightly rough gloss and nothing else."""
    g = D.materials.new("Sleeve")
    g.use_nodes = True
    nt = g.node_tree
    nt.nodes.remove(nt.nodes["Principled BSDF"])
    out = nt.nodes["Material Output"]
    mix = nt.nodes.new("ShaderNodeMixShader")
    fr = nt.nodes.new("ShaderNodeFresnel")
    fr.inputs["IOR"].default_value = 1.54
    tr = nt.nodes.new("ShaderNodeBsdfTransparent")
    gl = nt.nodes.new("ShaderNodeBsdfGlossy")
    gl.inputs["Roughness"].default_value = 0.12
    nt.links.new(fr.outputs[0], mix.inputs[0])
    nt.links.new(tr.outputs[0], mix.inputs[1])
    nt.links.new(gl.outputs[0], mix.inputs[2])
    nt.links.new(mix.outputs[0], out.inputs["Surface"])
    return g


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
    binders(M, R, sx, sy, rng)
    for i in range(3):
        R.append(box("arch%d" % i, (sx - 0.25 + i * 0.25, sy, 1.571 + 0.12), (0.24, 0.28, 0.24), M["card"], 0.004))
        R.append(box("archLid%d" % i, (sx - 0.25 + i * 0.25, sy, 1.571 + 0.235), (0.245, 0.285, 0.03), M["cardLid"], 0.003))
        R.append(slab("archHole%d" % i, (sx - 0.25 + i * 0.25, sy - 0.141, 1.571 + 0.17), 0.08, 0.025, 0.002, 0.012, M["black"]))
    bookcase_clues(M, R, sx, sy)
    # a shelf of paperbacks and manuals, muted spines, one leaning
    x = sx - 0.36
    cols = [(0.20, 0.17, 0.13), (0.12, 0.14, 0.15), (0.32, 0.30, 0.26), (0.16, 0.10, 0.08), (0.42, 0.40, 0.34), (0.10, 0.11, 0.10), (0.25, 0.22, 0.15)]
    i = 0
    while x < sx + 0.12:
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


# the two shelves of files: what he keeps on the house and the street. Listed from the right-hand end,
# the end the door camera sees: (cover, spine width, height, label from clues.py)
SHELVES = [
    [("charcoal", 0.075, 0.318, 5), ("charcoal", 0.05, 0.318, 4), ("navy", 0.075, 0.318, 6), ("kraft", 0.062, 0.296, 7),
     ("charcoal", 0.05, 0.318, 14), ("forest", 0.075, 0.318, 15), ("slate", 0.05, 0.31, 16), ("charcoal", 0.05, 0.318, 17),
     ("navy", 0.05, 0.31, 18), ("charcoal", 0.075, 0.318, 19)],
    [("navy", 0.075, 0.318, 0), ("navy", 0.05, 0.318, 1), ("forest", 0.05, 0.31, 2), ("kraft", 0.062, 0.296, 3),
     ("charcoal", 0.05, 0.318, 8), ("charcoal", 0.075, 0.318, 9), ("slate", 0.05, 0.31, 10)],
]


def binders(M, R, sx, sy, rng):
    """Lever-arch files on two shelves: book-cloth covers, a printed label in a clear pocket near the top,
    a nickel-rimmed finger hole near the bottom. Packed upright, each a hair off square. On the upper
    shelf two cloth hardbacks stand at the end, the last leaning on the bookcase's side."""
    from mathutils import Matrix
    xr = sx + 0.4 - 0.011  # the right-hand side's inner face
    for row, hz in enumerate([0.831, 1.201]):
        z0 = hz + 0.011
        x = xr - 0.003
        if row == 1:
            lean = rad(8)
            t1, h1 = 0.042, 0.268
            x1 = xr - h1 * math.tan(lean) - 0.001 - t1
            hardback(M, R, x1, sy, z0, t1, h1, 0.2, 1, lean)
            t0, h0 = 0.036, 0.252
            hardback(M, R, x1 - 0.007 - t0, sy, z0, t0, h0, 0.185, 0, 0.0)
            x = x1 - 0.007 - t0 - 0.012
        for i, (c, wdt, h, lab) in enumerate(SHELVES[row]):
            d = rng.uniform(0.28, 0.29)
            dy = rng.choice((0.0, 0.0, -0.004, -0.009, 0.003))
            fy = sy - 0.15 + 0.006 + dy  # the spine's face
            xc = x - wdt / 2
            n0 = len(R)
            R.append(slab("binder%d_%d" % (row, i), (xc, fy + d / 2, z0 + h / 2), wdt - 0.002, h, d, 0.006, M["cloth_" + c], axis="Y", bevel=0.0015))
            # the label: 42 × 120 mm on a 75 mm spine, scaled down on a narrower one
            lw = min(0.042, wdt - 0.016)
            lh = lw * 120 / 42
            lz = z0 + h - 0.03 - lh / 2
            R.append(decal("label%d_%d" % (row, i), (xc, fy - 0.0004, lz), lw, lh, M["label%02d" % lab], rot=(pi / 2, 0, 0)))
            R.append(box("pocket%d_%d" % (row, i), (xc, fy - 0.0009, lz), (lw + 0.006, 0.0005, lh + 0.008), M["sleeve"], 0.0))
            R[-1].visible_shadow = False
            # the finger hole: a dark recess in a pressed nickel grommet
            hzc = z0 + 0.072
            R.append(cyl("hole%d_%d" % (row, i), (xc, fy - 0.0003, hzc), 0.0112, 0.0008, M["hole"], 32, rot=(pi / 2, 0, 0)))
            bpy.ops.mesh.primitive_torus_add(major_radius=0.0128, minor_radius=0.0021, major_segments=40, minor_segments=10, location=(xc, fy - 0.0006, hzc), rotation=(pi / 2, 0, 0))
            g = bpy.context.active_object
            g.name = "grommet%d_%d" % (row, i)
            g.scale = (1.0, 1.0, 0.45)  # pressed flat against the spine
            g.data.materials.append(M["nickel"])
            for f in g.data.polygons:
                f.use_smooth = True
            R.append(g)
            # a hair off square, about its foot
            a = rad(rng.uniform(-0.5, 0.5))
            T = Matrix.Translation((xc, fy, z0)) @ Matrix.Rotation(a, 4, "Y") @ Matrix.Translation((-xc, -fy, -z0))
            for o in R[n0:]:
                o.matrix_basis = T @ o.matrix_basis
            x -= wdt + rng.uniform(0.0015, 0.004)
            if x - 0.05 < sx - 0.389:
                break


def hardback(M, R, x0, sy, z0, t, h, d, k, lean):
    """A cloth hardback, spine out: two boards, the spine (its foil title is a picture), the page block set
    between them. x0 is its left foot; lean tips it to the right about its right foot."""
    from mathutils import Matrix
    cl = M["cloth_" + ("forest" if k == 0 else "oxblood")]
    n0 = len(R)
    fy = sy - 0.15 + 0.012
    cx, cy, cz = x0 + t / 2, fy + d / 2, z0 + h / 2
    R.append(box("hbBoardL%d" % k, (x0 + 0.00125, cy, cz), (0.0025, d, h), cl, 0.0006))
    R.append(box("hbBoardR%d" % k, (x0 + t - 0.00125, cy, cz), (0.0025, d, h), cl, 0.0006))
    R.append(slab("hbSpine%d" % k, (cx, fy + 0.0015, cz), t, h, 0.003, 0.004, cl, axis="Y"))
    R.append(decal("hbTitle%d" % k, (cx, fy - 0.0002, cz), t - 0.002, h - 0.004, M["spine%d" % k], rot=(pi / 2, 0, 0)))
    R.append(box("hbPages%d" % k, (cx, cy + 0.002, cz), (t - 0.005, d - 0.006, h - 0.007), M["paper"], 0.0008))
    if lean:
        T = Matrix.Translation((x0 + t, 0, z0)) @ Matrix.Rotation(lean, 4, "Y") @ Matrix.Translation((-x0 - t, 0, -z0))
        for o in R[n0:]:
            o.matrix_basis = T @ o.matrix_basis


def wall_things(M, R):
    # an architect's elevation of the terrace, in a slim black aluminium frame hung flat to the wall
    fx, fz = 3.08, 1.62
    fw, fh, fd, rim = 0.40, 0.30, 0.016, 0.011
    for n, c, sz in (("frameT", (fx, RD - fd / 2, fz + fh / 2 - rim / 2), (fw, fd, rim)), ("frameB", (fx, RD - fd / 2, fz - fh / 2 + rim / 2), (fw, fd, rim)),
                     ("frameL", (fx - fw / 2 + rim / 2, RD - fd / 2, fz), (rim, fd, fh - 2 * rim)), ("frameR", (fx + fw / 2 - rim / 2, RD - fd / 2, fz), (rim, fd, fh - 2 * rim))):
        R.append(box(n, c, sz, M["frame"], 0.0012))
    R.append(box("frameBack", (fx, RD - 0.002, fz), (fw - 0.004, 0.003, fh - 0.004), M["mount"], 0))
    R.append(box("mount", (fx, RD - 0.0045, fz), (fw - 2 * rim, 0.002, fh - 2 * rim), M["mount"], 0))
    R.append(decal("print", (fx, RD - 0.0057, fz + 0.006), 0.26, 0.182, M["elevation"], rot=(pi / 2, 0, 0)))
    R.append(box("frameGlass", (fx, RD - fd + 0.003, fz), (fw - 2 * rim + 0.004, 0.002, fh - 2 * rim + 0.004), M["glass"], 0))
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
    # everything on the desk square to its edges, a hand's width apart: he tiles the desk as he tiles the Mac
    N.append(slab("trackpad", (dx + 0.262, RD - 0.6, top + 0.0075), 0.16, 0.115, 0.009, 0.01, M["white"], axis="Z", bevel=0.002))
    # the Mac mini to the left of the stand, its light on
    N.append(slab("mini", (dx - 0.45, RD - 0.26, top + 0.019), 0.197, 0.197, 0.036, 0.022, M["alu"], axis="Z", bevel=0.003))
    N.append(slab("miniFoot", (dx - 0.45, RD - 0.26, top + 0.0006), 0.17, 0.17, 0.0012, 0.02, M["black"], axis="Z"))
    N.append(cyl("miniLED", (dx - 0.45 + 0.07, RD - 0.26 - 0.0986, top + 0.012), 0.0013, 0.001, M["led"], 12, rot=(pi / 2, 0, 0)))
    # three pens, parallel, a finger apart; a steel rule beside them
    for i in range(3):
        pen(M, N, Vector((dx + 0.40 + i * 0.024, RD - 0.6, top + 0.0045)), 0.0, i == 1)
    N.append(box("ruler", (dx + 0.5, RD - 0.6, top + 0.0007), (0.025, 0.3, 0.0012), M["ruler"], 0.0003))
    for k in range(30):
        N.append(box("tick", (dx + 0.5 - 0.0125 + (0.003 if k % 5 else 0.0045), RD - 0.6 - 0.145 + k * 0.01, top + 0.00135), (0.006 if k % 5 else 0.009, 0.0004, 0.0001), M["ink"], 0))
    N.append(slab("notebook", (dx + 0.62, RD - 0.32, top + 0.0075), 0.13, 0.21, 0.015, 0.006, M["notebook"], axis="Z"))
    N.append(box("pages", (dx + 0.6215, RD - 0.32, top + 0.0075), (0.125, 0.2, 0.012), M["paper"], 0.001))
    N.append(box("band", (dx + 0.67, RD - 0.32, top + 0.0075), (0.006, 0.212, 0.0162), M["black"], 0.001))
    N.append(slab("labelmaker", (dx - 0.7, RD - 0.6, top + 0.018), 0.065, 0.13, 0.036, 0.012, M["black"], axis="Z", bevel=0.003))
    legal_pad(M, N, dx - 0.48, RD - 0.58, top)
    drill(M, N, dx + 0.64, RD - 0.12, top)
    mx_, my_ = dx + 0.62, RD - 0.6
    N.append(cyl("coaster", (mx_, my_, top + 0.003), 0.048, 0.006, M["cork"], 40, bevel=0.0015))
    mug = cyl("mug", (mx_, my_, top + 0.006 + 0.048), 0.041, 0.095, M["ceramic"], 48)
    bm_open(mug, 0.004)
    N.append(mug)
    N.append(cyl("mugBase", (mx_, my_, top + 0.008), 0.039, 0.004, M["ceramic"], 48))
    N.append(cyl("tea", (mx_, my_, top + 0.06), 0.0365, 0.002, M["tea"], 48))
    # the handle square to the desk's edge
    bpy.ops.mesh.primitive_torus_add(major_radius=0.026, minor_radius=0.0055, major_segments=32, minor_segments=12, location=(mx_ + 0.045, my_, top + 0.056), rotation=(pi / 2, 0, 0))
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
    """An architect's lamp, matte black with brass joints, on: a warm pool on the legal pad."""
    b, br = M["matte"], M["brass"]
    N.append(cyl("lampBase", (lx, ly, top + 0.012), 0.075, 0.024, b, 48, bevel=0.006))
    N.append(cyl("lampBaseRing", (lx, ly, top + 0.0245), 0.03, 0.004, br, 32, bevel=0.001))
    p0 = Vector((lx, ly, top + 0.03))
    p1 = p0 + Vector((0, -0.06, 0.38))
    p2 = p1 + Vector((0.04, -0.24, 0.05))
    for n, a, c in (("lampArm1", p0, p1), ("lampArm2", p1, p2)):
        d = c - a
        for off, r, s_ in ((0, 0.0055, 1.0), (0.016, 0.0035, 0.9)):
            o = cyl(n, (a + c) / 2 + Vector((off, 0, 0)), r, d.length * s_, b, 12)
            o.rotation_euler = d.to_track_quat("Z", "Y").to_euler()
            N.append(o)
    for p in (p0, p1, p2):
        N.append(cyl("joint", p, 0.012, 0.03, br, 16, rot=(0, pi / 2, 0), bevel=0.002))
    # the head looks down at the pad; its wide mouth toward it
    aim = Vector(LAMP_AIM)
    hc = p2 + Vector((0, -0.02, -0.06))
    d = (aim - hc).normalized()
    head = cyl("lampHead", hc, 0.058, 0.13, b, 48, r2=0.022)
    head.rotation_euler = (-d).to_track_quat("Z", "Y").to_euler()
    bm_open(head, 0.0015)
    N.append(head)
    rim = cyl("lampRim", hc + d * 0.064, 0.059, 0.005, br, 48, bevel=0.001)
    rim.rotation_euler = head.rotation_euler
    bm_open(rim, 0.002)  # a ring, not a lid over the mouth
    N.append(rim)
    bulb = cyl("lampBulb", hc + d * 0.03, 0.02, 0.03, M["bulb"], 24)
    bulb.rotation_euler = head.rotation_euler
    N.append(bulb)
    LAMP["at"] = tuple(hc + d * 0.05)


LAMP_AIM = (1.53, 2.63, 0.74)  # the middle of the legal pad
LAMP = {}


def legal_pad(M, N, px, py, top):
    """A yellow legal pad squared to the desk: its glued band at the top, a card back, the page of timings."""
    w, h = 0.216, 0.356
    N.append(box("padBack", (px, py, top + 0.0006), (w, h, 0.0012), M["padBack"], 0))
    N.append(box("padBlock", (px, py - 0.004, top + 0.0045), (w - 0.001, h - 0.012, 0.006), M["paper"], 0.0005))
    N.append(decal("padTop", (px, py - 0.004, top + 0.0076), w - 0.001, h - 0.012, M["legalpad"]))
    N.append(box("padBand", (px, py + h / 2 - 0.009, top + 0.005), (w + 0.001, 0.018, 0.0085), M["padBand"], 0.0008))
    # his pen, capped, squared beside it
    pen(M, N, Vector((px + w / 2 + 0.018, py, top + 0.0045)), 0.0, True)


def drill(M, N, x, y, top):
    """Mark's cordless drill, stood on its battery in the far corner, the chuck square to the wall; the
    sticky note on the battery says whose it is. Soft, moulded forms: the housing in satin yellow, a grey
    overmould on the grip, a ribbed chuck."""
    # the battery pack and the foot it clips into
    N.append(sbox("drillBatt", (x, y, top + 0.033), (0.088, 0.08, 0.064), M["rubber"], 0.012, 3, sub=1))
    N.append(sbox("drillFoot", (x - 0.002, y, top + 0.071), (0.074, 0.07, 0.016), M["drill"], 0.007, 3, sub=1))
    # the grip, rising and leaning back a little, waisted where the hand goes
    g = sbox("drillGrip", (x + 0.012, y, top + 0.138), (0.038, 0.044, 0.124), M["rubber"], 0.016, 6, rot=(0, rad(-10), 0), sub=2)
    w = g.modifiers.new("waist", "SIMPLE_DEFORM")
    w.deform_method = "TAPER"
    w.deform_axis = "Z"
    w.factor = -0.12
    g.modifiers.move(len(g.modifiers) - 1, 0)
    N.append(g)
    # the motor housing along x, its back rounded off, the chuck pointing at -x
    N.append(cyl("drillBody", (x - 0.012, y, top + 0.217), 0.035, 0.15, M["drill"], 48, rot=(0, pi / 2, 0), bevel=0.014))
    N.append(sbox("drillNeck", (x + 0.006, y, top + 0.19), (0.05, 0.05, 0.04), M["drill"], 0.014, 4, sub=1))
    for k in range(5):  # the vents
        N.append(box("drillVent", (x + 0.035 - k * 0.007, y - 0.0345, top + 0.225), (0.0028, 0.003, 0.022), M["rubber"], 0.0008))
    N.append(cyl("drillCollar", (x - 0.093, y, top + 0.217), 0.029, 0.018, M["rubber"], 48, rot=(0, pi / 2, 0), bevel=0.004))
    N.append(cyl("drillChuck", (x - 0.121, y, top + 0.217), 0.022, 0.04, M["rubber"], 48, rot=(0, pi / 2, 0), r2=0.017, bevel=0.003))
    for k in range(12):  # the chuck's grip ribs
        a = k * pi / 6
        N.append(box("drillRib", (x - 0.12, y + math.cos(a) * 0.0205, top + 0.217 + math.sin(a) * 0.0205), (0.034, 0.0022, 0.0022), M["rubber"], 0.0007, rot=(a, 0, 0)))
    N.append(cyl("drillJaw", (x - 0.145, y, top + 0.217), 0.0075, 0.01, M["chrome"], 24, rot=(0, pi / 2, 0), r2=0.005))
    N.append(cyl("drillBit", (x - 0.172, y, top + 0.217), 0.0033, 0.048, M["chrome"], 16, rot=(0, pi / 2, 0)))
    N.append(sbox("drillTrigger", (x - 0.012, y, top + 0.165), (0.012, 0.016, 0.026), M["rubber"], 0.005, 3, rot=(0, rad(-10), 0)))
    # the note, on the battery's face toward the room
    N.append(decal("sticky", (x, y - 0.0406, top + 0.036), 0.05, 0.05, M["sticky"], rot=(pi / 2, 0, 0)))


def corkboard(M, R):
    """A small corkboard left of the screen: four index cards in a level row, four, five, six, and seven,
    blank, each pinned at its top. It hangs clear of the bookcase, so all four read from the door."""
    cx, cz = 1.42, 1.66
    w, h = 0.6, 0.36
    R.append(box("corkFrame", (cx, RD - 0.012, cz), (w + 0.04, 0.022, h + 0.04), M["oakFrame"], 0.003))
    R.append(box("cork", (cx, RD - 0.0235, cz), (w, 0.004, h), M["cork"], 0))
    for i, n in enumerate("4567"):
        # 5 × 3 in cards, 14 cm apart, squared to the frame
        x = cx - 0.21 + i * 0.14
        R.append(decal("card" + n, (x, RD - 0.0262, cz + 0.035), 0.127, 0.076, M["card" + n], rot=(pi / 2, 0, 0)))
        R.append(cyl("pin" + n, (x, RD - 0.031, cz + 0.064), 0.0045, 0.008, M["pin"], 16, rot=(pi / 2, 0, 0), bevel=0.001))


def bookcase_clues(M, R, sx, sy):
    """On the shelves to the left, where the screen's light doesn't reach: a roll of heavy clear sheeting
    on the bottom shelf, and two tins of cat food stacked at the end of the paperbacks."""
    z = 0.091 + 0.082
    R.append(cyl("plasticCore", (sx, sy - 0.02, z), 0.038, 0.745, M["core"], 32, rot=(0, pi / 2, 0)))
    R.append(cyl("plastic", (sx, sy - 0.02, z), 0.08, 0.72, M["plastic"], 64, rot=(0, pi / 2, 0)))
    R.append(cyl("plasticBand", (sx + 0.08, sy - 0.02, z), 0.0805, 0.05, M["bandBlue"], 64, rot=(0, pi / 2, 0)))
    R.append(cyl("plasticBand2", (sx + 0.17, sy - 0.02, z), 0.0805, 0.13, M["bandWhite"], 64, rot=(0, pi / 2, 0)))
    for k in range(2):
        zz = 0.461 + 0.0225 + k * 0.046
        R.append(cyl("tin%d" % k, (sx + 0.3, sy - 0.06, zz), 0.0425, 0.04, M["tin"], 40))
        for e in (-1, 1):
            R.append(cyl("tinRim%d" % k, (sx + 0.3, sy - 0.06, zz + e * 0.021), 0.0428, 0.003, M["tinEnd"], 40, bevel=0.0008))


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
    # a streetlight down the road to the south-east, its lamp a big diffuser eight metres off (about 11°
    # across), so the blinds and glazing bars throw a soft, feathered glow on the desk wall, not a stencil
    L["street"] = light("street", "SPOT", (RW + 5.2, WY0 - 5.8, 3.2), STREET_W, (0.56, 0.68, 1.0), 0.75, target=(RW, (WY0 + WY1) / 2 + 0.25, 1.75), spot=20, blend=1.0)
    # the sky past the slats, a dim cool panel inside the window: a rim on the chair, a little fill
    L["sky"] = light("sky", "AREA", (RW - 0.04, (WY0 + WY1) / 2, (WZ0 + WZ1) / 2), SKY_W, (0.50, 0.62, 1.0), (1.2, 1.0), target=(RW - 2, (WY0 + WY1) / 2, 1.0))
    L["sky"].visible_camera = False
    L["sky"].visible_glossy = False
    # the lamp's light bounced off the desk and the pale walls, from behind the door: a low warm fill so the
    # chair's fabric reads, never enough to cast a shadow of its own
    L["fill"] = light("fill", "AREA", (1.5, 0.75, 1.7), FILL_W, (1.0, 0.80, 0.60), (1.8, 1.4), target=(CHAIR[0], CHAIR[1], 0.7))
    L["fill"].visible_camera = False
    L["fill"].visible_glossy = False
    # the landing light, warm, through the open door
    lx = (DX0 + DX1) / 2 + 0.3
    L["landing"] = light("landing", "AREA", (lx, -1.0, RH - 0.05), LANDING_W, (1.0, 0.70, 0.45), (0.3, 0.3), target=(lx, -1.0, 0))
    # the night past the slats: a faint glow
    # the last of the dusk past the slats: a deep, cold blue
    sky = mat("Night", (0, 0, 0), 1.0, emit=(0.035, 0.06, 0.15), strength=0.34)
    L["night"] = box("Night", (RW + 6, (WY0 + WY1) / 2, 2.0), (0.1, 16, 10), sky, 0)
    L["bg"] = world((0.002, 0.0025, 0.004), 1.0)
    # the desk lamp, the room's key: a warm pool on the pad that spills across the desk, and the shade's
    # glow up its back and onto the wall
    L["lamp"] = light("lamp", "SPOT", LAMP["at"], LAMP_W, (1.0, 0.70, 0.40), 0.03, target=LAMP_AIM, spot=78, blend=0.75)
    L["spill"] = light("spill", "POINT", tuple(Vector(LAMP["at"]) + Vector((0, 0.05, 0.06))), SPILL_W, (1.0, 0.66, 0.36), 0.04)
    L["spill"].visible_camera = False
    return L


STREET_W = 5200
SKY_W = 18
FILL_W = 7
LAMP_W = 15
SPILL_W = 1.2
LANDING_W = 140


def build():
    M = materials()
    NEAR, ROOM = [], []
    shell(M, ROOM)
    window(M, ROOM)
    bookcase(M, ROOM)
    wall_things(M, ROOM)
    corkboard(M, ROOM)
    ROOM.append(slab("rug", (2.0, RD - 1.05, 0.004), 1.7, 1.25, 0.008, 0.01, M["rug"], axis="Z", bevel=0.003))
    desk(M, NEAR)
    chair(M, NEAR)
    scr = screen(M)
    return dict(M=M, NEAR=NEAR, ROOM=ROOM, screen=scr, lights=lights())
