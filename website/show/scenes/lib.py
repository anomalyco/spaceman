# Shared helpers for the film's Blender scenes: procedural modelling, materials, light baking into
# atlases, glTF export and emission-only previews of exactly what the browser will draw.
# Run inside Blender (bpy). Nothing here opens or saves .blend files.
import bpy, bmesh, math, json, os, sys
import numpy as np
from mathutils import Vector, Matrix, Euler

D = bpy.data


def args():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    s = bpy.context.scene
    s.unit_settings.system = "METRIC"
    s.view_settings.view_transform = "AgX"
    s.view_settings.look = "AgX - Medium High Contrast"
    return s


def gpu(scene, samples=256):
    scene.render.engine = "CYCLES"
    prefs = bpy.context.preferences.addons["cycles"].preferences
    try:
        prefs.compute_device_type = "METAL"
        prefs.get_devices()
        for d in prefs.devices:
            d.use = True
        scene.cycles.device = "GPU"
    except Exception as e:
        print("gpu: falling back to CPU", e)
    scene.cycles.samples = samples
    scene.cycles.use_denoising = False


# ── geometry ────────────────────────────────────────────────────────────
def obj_from_bm(name, bm, mat=None):
    me = D.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    o = D.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    if mat:
        me.materials.append(mat)
    return o


def box(name, center, size, mat=None, bevel=0.004, segs=2, rot=(0, 0, 0)):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    o = obj_from_bm(name, bm, mat)
    o.location = center
    o.rotation_euler = rot
    if bevel > 0:
        m = o.modifiers.new("bevel", "BEVEL")
        m.width = min(bevel, min(size) * 0.45)
        m.segments = segs
        m.limit_method = "ANGLE"
    return o


def cyl(name, center, r, h, mat=None, verts=32, rot=(0, 0, 0), bevel=0.0, r2=None):
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=verts, radius1=r, radius2=r if r2 is None else r2, depth=h)
    o = obj_from_bm(name, bm, mat)
    o.location = center
    o.rotation_euler = rot
    for f in o.data.polygons:
        f.use_smooth = len(f.vertices) == 4
    if bevel > 0:
        m = o.modifiers.new("bevel", "BEVEL")
        m.width = bevel
        m.segments = 2
        m.limit_method = "ANGLE"
    return o


def plane(name, center, size, mat=None, rot=(0, 0, 0)):
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=0.5)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], 0))
    o = obj_from_bm(name, bm, mat)
    o.location = center
    o.rotation_euler = rot
    return o


def realize(objs):
    """Evaluates every modifier (bevels, arrays) into plain meshes."""
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.convert(target="MESH")


def join(name, objs):
    objs = [o for o in objs if o.type == "MESH"]
    realize(objs)
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    if len(objs) > 1:
        bpy.ops.object.join()
    o = bpy.context.view_layer.objects.active
    o.name = name
    o.data.name = name
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return o


# ── materials ───────────────────────────────────────────────────────────
def mat(name, color=(0.5, 0.5, 0.5), rough=0.5, metal=0.0, emit=None, strength=1.0):
    m = D.materials.new(name)
    m.use_nodes = True
    p = m.node_tree.nodes["Principled BSDF"]
    p.inputs["Base Color"].default_value = (*color, 1)
    p.inputs["Roughness"].default_value = rough
    p.inputs["Metallic"].default_value = metal
    if emit:
        p.inputs["Emission Color"].default_value = (*emit, 1)
        p.inputs["Emission Strength"].default_value = strength
    return m


def node(m, kind, **props):
    n = m.node_tree.nodes.new(kind)
    for k, v in props.items():
        setattr(n, k, v)
    return n


def link(m, a, b):
    m.node_tree.links.new(a, b)


def bsdf(m):
    return m.node_tree.nodes["Principled BSDF"]


def texcoord(m, kind="Object"):
    t = m.node_tree.nodes.get("tc")
    if not t:
        t = node(m, "ShaderNodeTexCoord")
        t.name = "tc"
    return t.outputs[kind]


def noisy(m, color_a, color_b, scale=8.0, detail=6.0, rough=(0.45, 0.6), coord="Object", bump=0.0, mscale=(1, 1, 1)):
    """Base colour between a and b by a noise; roughness varies with it; optional bump."""
    mp = node(m, "ShaderNodeMapping")
    mp.inputs["Scale"].default_value = mscale
    link(m, texcoord(m, coord), mp.inputs["Vector"])
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = scale
    n.inputs["Detail"].default_value = detail
    link(m, mp.outputs["Vector"], n.inputs["Vector"])
    r = node(m, "ShaderNodeValToRGB")
    r.color_ramp.elements[0].color = (*color_a, 1)
    r.color_ramp.elements[1].color = (*color_b, 1)
    link(m, n.outputs["Fac"], r.inputs["Fac"])
    link(m, r.outputs["Color"], bsdf(m).inputs["Base Color"])
    rr = node(m, "ShaderNodeMapRange")
    rr.inputs["To Min"].default_value = rough[0]
    rr.inputs["To Max"].default_value = rough[1]
    link(m, n.outputs["Fac"], rr.inputs["Value"])
    link(m, rr.outputs["Result"], bsdf(m).inputs["Roughness"])
    if bump > 0:
        b = node(m, "ShaderNodeBump")
        b.inputs["Strength"].default_value = bump
        b.inputs["Distance"].default_value = 0.002
        link(m, n.outputs["Fac"], b.inputs["Height"])
        link(m, b.outputs["Normal"], bsdf(m).inputs["Normal"])
    return m


def wood(name, base=(0.42, 0.27, 0.15), dark=(0.25, 0.14, 0.07), scale=(1, 14, 1), rough=0.42):
    """Oak: rings stretched along the object's X, broken up by a fine noise."""
    m = mat(name, base, rough)
    mp = node(m, "ShaderNodeMapping")
    mp.inputs["Scale"].default_value = scale
    link(m, texcoord(m, "Object"), mp.inputs["Vector"])
    w = node(m, "ShaderNodeTexWave")
    w.wave_type = "RINGS"
    w.inputs["Scale"].default_value = 2.2
    w.inputs["Distortion"].default_value = 6.0
    w.inputs["Detail"].default_value = 3.0
    link(m, mp.outputs["Vector"], w.inputs["Vector"])
    n = node(m, "ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 30.0
    n.inputs["Detail"].default_value = 8
    link(m, mp.outputs["Vector"], n.inputs["Vector"])
    mx = node(m, "ShaderNodeMix")
    mx.data_type = "FLOAT"
    mx.inputs[0].default_value = 0.35
    link(m, w.outputs["Fac"], mx.inputs[2])
    link(m, n.outputs["Fac"], mx.inputs[3])
    r = node(m, "ShaderNodeValToRGB")
    r.color_ramp.elements[0].position = 0.25
    r.color_ramp.elements[0].color = (*dark, 1)
    r.color_ramp.elements[1].position = 0.85
    r.color_ramp.elements[1].color = (*base, 1)
    link(m, mx.outputs[0], r.inputs["Fac"])
    link(m, r.outputs["Color"], bsdf(m).inputs["Base Color"])
    return m


def boards(name, base=(0.36, 0.23, 0.13), dark=(0.2, 0.12, 0.06), width=0.14, length=1.2):
    """Floorboards: a brick texture lays out the boards, each a slightly different tone."""
    m = wood(name, base, dark, scale=(1, 18, 1))
    b = node(m, "ShaderNodeTexBrick")
    b.offset = 0.37
    b.inputs["Scale"].default_value = 1.0
    b.inputs["Mortar Size"].default_value = 0.0015
    b.inputs["Brick Width"].default_value = length
    b.inputs["Row Height"].default_value = width
    b.inputs["Color1"].default_value = (1, 1, 1, 1)
    b.inputs["Color2"].default_value = (0.8, 0.8, 0.8, 1)
    b.inputs["Mortar"].default_value = (0.25, 0.25, 0.25, 1)
    link(m, texcoord(m, "Object"), b.inputs["Vector"])
    ramp = [n for n in m.node_tree.nodes if n.type == "VALTORGB"][0]
    mul = node(m, "ShaderNodeMix")
    mul.data_type = "RGBA"
    mul.blend_type = "MULTIPLY"
    mul.inputs[0].default_value = 1.0
    link(m, ramp.outputs["Color"], mul.inputs[6])
    link(m, b.outputs["Color"], mul.inputs[7])
    link(m, mul.outputs[2], bsdf(m).inputs["Base Color"])
    return m


# ── cameras and lights ──────────────────────────────────────────────────
def camera(name, loc, target, vfov_deg, dof=None):
    cd = D.cameras.new(name)
    cd.sensor_fit = "VERTICAL"
    cd.sensor_height = 24.0
    cd.lens = 12.0 / math.tan(math.radians(vfov_deg) / 2)
    cd.clip_start = 0.02
    cd.clip_end = 200
    o = D.objects.new(name, cd)
    bpy.context.scene.collection.objects.link(o)
    o.location = loc
    d = Vector(target) - Vector(loc)
    o.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    if dof:
        cd.dof.use_dof = True
        cd.dof.focus_distance = d.length
        cd.dof.aperture_fstop = dof
    return o


def light(name, kind, loc, energy, color=(1, 1, 1), size=0.1, target=None, spot=None, blend=0.3):
    ld = D.lights.new(name, kind)
    ld.energy = energy
    ld.color = color
    if kind == "AREA":
        ld.shape = "RECTANGLE"
        ld.size, ld.size_y = size if isinstance(size, tuple) else (size, size)
    else:
        ld.shadow_soft_size = size
    if kind == "SPOT":
        ld.spot_size = math.radians(spot or 60)
        ld.spot_blend = blend
    o = D.objects.new(name, ld)
    bpy.context.scene.collection.objects.link(o)
    o.location = loc
    if target is not None:
        o.rotation_euler = (Vector(target) - Vector(loc)).to_track_quat("-Z", "Y").to_euler()
    return o


def world(color, strength=1.0):
    w = D.worlds.new("World")
    w.use_nodes = True
    bg = w.node_tree.nodes["Background"]
    bg.inputs["Color"].default_value = (*color, 1)
    bg.inputs["Strength"].default_value = strength
    bpy.context.scene.world = w
    return bg


# ── baking ──────────────────────────────────────────────────────────────
def uv_atlas(o, margin=0.003):
    bpy.ops.object.select_all(action="DESELECT")
    o.select_set(True)
    bpy.context.view_layer.objects.active = o
    uv = o.data.uv_layers.new(name="bake")
    o.data.uv_layers.active = uv
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(60), island_margin=margin, area_weight=1.0, scale_to_bounds=False)
    try:
        bpy.ops.uv.pack_islands(rotate=True, margin=margin)
    except Exception as e:
        print("pack_islands", e)
    bpy.ops.object.mode_set(mode="OBJECT")


def bake_into(o, name, res, samples, margin=6):
    """Bakes diffuse light × albedo of object o (all its materials) into one float image."""
    img = D.images.new(name, res, res, float_buffer=True, alpha=False)
    for slot in o.material_slots:
        if slot.material is None:
            continue
        nt = slot.material.node_tree
        n = nt.nodes.new("ShaderNodeTexImage")
        n.image = img
        n.name = "BAKE_TARGET"
        nt.nodes.active = n
    o.data.uv_layers.active = o.data.uv_layers["bake"]
    bpy.ops.object.select_all(action="DESELECT")
    o.select_set(True)
    bpy.context.view_layer.objects.active = o
    sc = bpy.context.scene
    sc.cycles.samples = samples
    sc.render.bake.margin = margin
    bpy.ops.object.bake(type="DIFFUSE", pass_filter={"DIRECT", "INDIRECT", "COLOR"}, margin=margin, use_clear=True)
    for slot in o.material_slots:
        if slot.material is None:
            continue
        nt = slot.material.node_tree
        if "BAKE_TARGET" in nt.nodes:
            nt.nodes.remove(nt.nodes["BAKE_TARGET"])
    return img


def pixels(img):
    a = np.empty(img.size[0] * img.size[1] * 4, dtype=np.float32)
    img.pixels.foreach_get(a)
    return a.reshape(img.size[1], img.size[0], 4)


def smooth(a, k=1, sigma=0.12):
    """An edge-aware smoothing of bake noise (bilateral on luminance)."""
    rgb = a[..., :3]
    lum = rgb.mean(-1)
    acc = np.zeros_like(rgb)
    wsum = np.zeros(lum.shape, dtype=np.float32)
    for dy in range(-k, k + 1):
        for dx in range(-k, k + 1):
            s = np.roll(np.roll(rgb, dy, 0), dx, 1)
            sl = s.mean(-1)
            w = np.exp(-((sl - lum) ** 2) / (2 * (sigma * (lum + 0.01)) ** 2)) * math.exp(-(dx * dx + dy * dy) / (2.0 * k * k))
            acc += s * w[..., None]
            wsum += w
    out = a.copy()
    out[..., :3] = acc / wsum[..., None]
    return out


def auto_scale(img, pct=99.8, target=0.95):
    a = pixels(img)[..., :3]
    v = float(np.percentile(a.max(-1), pct))
    return target / max(v, 1e-5)


def save_jpeg(img, path, scale, quality=88, k=1):
    """Float linear pixels × scale → sRGB JPEG. The renderer divides by scale."""
    a = pixels(img)
    if k:
        a = smooth(a, k)
    rgb = np.clip(a[..., :3] * scale, 0, 1)
    srgb = np.where(rgb <= 0.0031308, rgb * 12.92, 1.055 * np.power(rgb, 1 / 2.4) - 0.055)
    h, w = srgb.shape[:2]
    out = D.images.new(os.path.basename(path), w, h, alpha=False)
    buf = np.ones((h, w, 4), dtype=np.float32)
    buf[..., :3] = srgb
    out.pixels.foreach_set(buf.ravel())
    out.filepath_raw = path
    out.file_format = "JPEG"
    sc = bpy.context.scene
    sc.render.image_settings.file_format = "JPEG"
    sc.render.image_settings.quality = quality
    os.makedirs(os.path.dirname(path), exist_ok=True)
    out.save(filepath=path, quality=quality)
    D.images.remove(out)
    img2 = D.images.load(path, check_existing=False)
    img2.colorspace_settings.name = "sRGB"
    return img2


def baked_material(name, base_img, emit_img=None):
    m = D.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    p = nt.nodes["Principled BSDF"]
    uv = nt.nodes.new("ShaderNodeUVMap")
    uv.uv_map = "UVMap"
    t = nt.nodes.new("ShaderNodeTexImage")
    t.image = base_img
    nt.links.new(uv.outputs["UV"], t.inputs["Vector"])
    nt.links.new(t.outputs["Color"], p.inputs["Base Color"])
    p.inputs["Roughness"].default_value = 1.0
    if emit_img is not None:
        e = nt.nodes.new("ShaderNodeTexImage")
        e.image = emit_img
        nt.links.new(uv.outputs["UV"], e.inputs["Vector"])
        nt.links.new(e.outputs["Color"], p.inputs["Emission Color"])
        p.inputs["Emission Strength"].default_value = 1.0
    return m


def only_uv(o, keep="bake"):
    for uv in list(o.data.uv_layers):
        if uv.name != keep:
            o.data.uv_layers.remove(uv)
    o.data.uv_layers[keep].name = "UVMap"


def set_material(o, m):
    o.data.materials.clear()
    o.data.materials.append(m)
    for p in o.data.polygons:
        p.material_index = 0


def export_glb(path, objs):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.hide_set(False)
        o.select_set(True)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    kw = dict(filepath=path, export_format="GLB", use_selection=True, export_cameras=True, export_lights=False,
              export_apply=True, export_yup=True, export_image_format="JPEG", export_materials="EXPORT")
    props = bpy.ops.export_scene.gltf.get_rna_type().properties.keys()
    for k in ("export_image_quality", "export_jpeg_quality"):
        if k in props:
            kw[k] = 88
    bpy.ops.export_scene.gltf(**kw)


def render_still(path, cam, w, h, samples=64, denoise=True):
    sc = bpy.context.scene
    sc.camera = cam
    sc.render.resolution_x = w
    sc.render.resolution_y = h
    sc.render.resolution_percentage = 100
    sc.render.image_settings.file_format = "JPEG"
    sc.render.image_settings.quality = 85
    sc.cycles.samples = samples
    sc.cycles.use_denoising = denoise
    sc.render.filepath = path
    bpy.ops.render.render(write_still=True)


def emission_preview(o, base_img, base_scale, emit_img=None, emit_scale=1.0, screen=1.0):
    """o's material becomes emission = what the browser draws (base/scale + emit/scale × screen)."""
    m = D.materials.new(o.name + "_prev")
    m.use_nodes = True
    nt = m.node_tree
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    em = nt.nodes.new("ShaderNodeEmission")
    uv = nt.nodes.new("ShaderNodeUVMap")
    uv.uv_map = "UVMap"
    a = nt.nodes.new("ShaderNodeTexImage")
    a.image = base_img
    nt.links.new(uv.outputs["UV"], a.inputs["Vector"])
    sa = nt.nodes.new("ShaderNodeVectorMath")
    sa.operation = "SCALE"
    sa.inputs["Scale"].default_value = 1.0 / base_scale
    nt.links.new(a.outputs["Color"], sa.inputs[0])
    col = sa.outputs[0]
    if emit_img is not None:
        b = nt.nodes.new("ShaderNodeTexImage")
        b.image = emit_img
        nt.links.new(uv.outputs["UV"], b.inputs["Vector"])
        sb = nt.nodes.new("ShaderNodeVectorMath")
        sb.operation = "SCALE"
        sb.inputs["Scale"].default_value = screen / emit_scale
        nt.links.new(b.outputs["Color"], sb.inputs[0])
        ad = nt.nodes.new("ShaderNodeVectorMath")
        ad.operation = "ADD"
        nt.links.new(sa.outputs[0], ad.inputs[0])
        nt.links.new(sb.outputs[0], ad.inputs[1])
        col = ad.outputs[0]
    nt.links.new(col, em.inputs["Color"])
    nt.links.new(em.outputs[0], out.inputs["Surface"])
    set_material(o, m)
