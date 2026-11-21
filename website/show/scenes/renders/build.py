# Offline renders of our own scenes, for pictures inside his apps and the desktop wallpaper:
#   hedge-0..3  No. 14's hedge from his drive, the same spot every morning (clear, wet, autumn, mist)
#   across      the photograph No. 14 sent: his study window at night, from across the street
#   door        his doorbell camera at 11:47 PM, wide, across to No. 14
#   wall, wall-s  his wallpaper: dunes at dusk
# and stills.json (where the hedge top, the 6 ft line and No. 14's lit window land in the frames).
# usage: Blender -b --factory-startup -P build.py -- [quick] [only=hedge,across,door,wall] [out=<dir>]
import os, sys, math, json
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import bpy, bmesh
from mathutils import Vector
from bpy_extras.object_utils import world_to_camera_view
from lib import *
import street as st

A = args()
QUICK = "quick" in A
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = next((a[4:] for a in A if a.startswith("out=")), os.path.join(HERE, "out"))
ONLY = next((a[5:].split(",") for a in A if a.startswith("only=")), ["hedge", "across", "door", "wall"])
os.makedirs(OUT, exist_ok=True)
G = -3.0
rad = math.radians
SPP = 48 if QUICK else 256
meta = {}


def save(name, cam, w, h, spp=SPP):
    s = 0.5 if QUICK else 1.0
    render_still(os.path.join(OUT, name + ".jpg"), cam, int(w * s), int(h * s), spp)


def proj(cam, p):
    sc = bpy.context.scene
    v = world_to_camera_view(sc, cam, Vector(p))
    return [round(v.x, 4), round(1 - v.y, 4)]


def street_scene(season=0.0, wet=0.0, detail=True, ours=False, props=False):
    sc = reset()
    gpu(sc)
    M = st.materials(season, wet)
    parts, lamp = st.build(G, M, detail=detail, ours=ours, props=props)
    return sc, M, parts, lamp


# ── the hedge, four mornings ──
if "hedge" in ONLY:
    looks = [
        # season, wet, sun elevation, sun rotation, sky strength, sun strength, sun colour, mist
        ("hedge-0", 0.0, 0.0, 28, 120, 0.9, 3.5, (1.0, 0.95, 0.86), 0.0),
        ("hedge-1", 0.25, 1.0, 20, 150, 0.6, 0.0, (1, 1, 1), 0.0),
        ("hedge-2", 0.85, 0.0, 9, 210, 0.7, 4.0, (1.0, 0.72, 0.45), 0.0),
        ("hedge-3", 0.5, 0.3, 4, 100, 0.5, 0.6, (1.0, 0.8, 0.65), 0.07),
    ]
    for name, season, wet, el, rot, skys, suns, sunc, mist in looks:
        sc, M, parts, lamp = street_scene(season, wet, props=True)
        sc.view_settings.exposure = -1.3
        lamp.hide_render = True
        if season > 0.6:
            st.leaves(G)
        st.sky(skys, el, rot, turbidity=2.0 + 3 * wet + 4 * mist, tint=(0.85, 0.88, 1.0) if wet > 0.5 else (1, 1, 1))
        if suns > 0:
            sun = light("sun", "SUN", (0, 0, 30), suns, sunc, 0.02)
            # the sun comes from the south-east, over No. 14's shoulder
            sun.rotation_euler = (rad(90 - el), 0, rad(rot - 90))
        if mist > 0:
            vol = box("mist", (14, 0, G + 6), (60, 80, 14), None, 0)
            vm = D.materials.new("mist")
            vm.use_nodes = True
            nt = vm.node_tree
            nt.nodes.remove(nt.nodes["Principled BSDF"])
            pv = nt.nodes.new("ShaderNodeVolumePrincipled")
            pv.inputs["Density"].default_value = mist
            pv.inputs["Color"].default_value = (0.85, 0.88, 0.92, 1)
            nt.links.new(pv.outputs["Volume"], nt.nodes["Material Output"].inputs["Volume"])
            vol.data.materials.append(vm)
        cam = camera("CamDrive", (7.2, -5.6, G + 1.62), (20.3, -3.6, G + 1.25), 21)
        sc.camera = cam
        save(name, cam, 1200, 800)
        if name == "hedge-0":
            meta["hedge"] = {"top": proj(cam, (19.83, -4.2, G + st.HEDGE_H))[1], "limit": proj(cam, (19.83, -4.2, G + 1.829))[1],
                             "x0": proj(cam, (19.83, -6.5, G + 1.8))[0], "x1": proj(cam, (19.83, -1.0, G + 1.8))[0]}

# ── the photograph he took of my window ──
night_lit = None
if "across" in ONLY or "door" in ONLY:
    sc, M, parts, lamp = street_scene(0.85, 0.2, ours=True)
    bg, L = st.night(G)
    mon = light("monitor", "AREA", (2.0, 2.9, 1.07), 26, (0.86, 0.9, 1.0), (0.62, 0.39), target=(2.0, -3, 1.0))
    mon.data.spread = rad(160)
    glow = box("monGlow", (2.0, 2.9, 1.07), (0.62, 0.004, 0.39), mat("monGlowM", (0, 0, 0), 1.0, emit=(0.8, 0.86, 1.0), strength=6.0), 0)
    if "across" in ONLY:
        cam = camera("CamAcross", (21.2, 1.0, G + 1.62), (3.72, 2.3, 1.45), 9.5)
        save("across", cam, 1600, 1200)
    if "door" in ONLY:
        L[2].data.energy = 2500  # the streetlight on our side floods the doorbell camera's foreground
        cam = camera("CamDoor", (3.86, -0.35, G + 1.45), (20.0, 0.6, G + 2.1), 64)
        cam.data.clip_start = 0.05
        save("door", cam, 1280, 720)
        a = proj(cam, (26.0, 1.8 - 0.6, G + 3.9 + 0.6))
        b = proj(cam, (26.0, 1.8 + 0.6, G + 3.9 - 0.6))
        meta["doorWin"] = [a[0], a[1], b[0], b[1]]

# ── the wallpaper: dunes at dusk ──
if "wall" in ONLY:
    sc = reset()
    gpu(sc)
    sc.view_settings.exposure = -0.8
    sand = noisy(mat("sand", rough=0.85), (0.46, 0.22, 0.1), (0.56, 0.29, 0.14), scale=0.8, detail=4, bump=0.2)
    tex = D.textures.new("dune", "CLOUDS")
    tex.noise_scale = 9.0
    tex.noise_depth = 1
    # one grid, dense by the camera (at y = -40) and coarse out to the horizon
    segs = 500 if not QUICK else 200
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=segs, y_segments=segs, size=1.0)
    for v in bm.verts:
        u, w = v.co.x, (v.co.y + 1) / 2
        v.co.x = math.copysign(abs(u) ** 2.2, u) * 1800
        v.co.y = -46 + 1846 * w ** 3
    land = obj_from_bm("dunes", bm, None)
    dp = land.modifiers.new("d", "DISPLACE")
    dp.texture = tex
    dp.strength = 4.5
    dp.texture_coords = "GLOBAL"
    for p in land.data.polygons:
        p.use_smooth = True
    land.data.materials.append(sand)
    st.sky(0.55, 3.5, 250, turbidity=3.0, tint=(1.0, 0.9, 0.95))
    sun = light("sun", "SUN", (0, 0, 30), 2.2, (1.0, 0.62, 0.38), 0.6)
    sun.rotation_euler = (rad(86.5), 0, rad(250 - 90))
    cam = camera("CamWall", (0, -40, 5.5), (0, 10, 2.0), 22)
    save("wall", cam, 2560, 1600, 128 if not QUICK else 32)
    render_still(os.path.join(OUT, "wall-s.jpg"), cam, 960, 600, 64 if not QUICK else 16)

with open(os.path.join(OUT, "stills.json"), "w") as f:
    json.dump(meta, f, indent=1)
print("STILLS", json.dumps(meta))
