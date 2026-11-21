# A Blender render made to read as a phone photo taken at night: held a little crooked, the sensor's
# noise in the shadows (fine luminance grain, blotchier colour noise), the phone's sharpening, its JPEG.
# usage: uv run --with pillow --with numpy show/tools/phonify.py in.png out.jpg [tilt_degrees] [gain] [falloff]
#   gain darkens or lifts the whole frame; falloff dims it toward the right, away from the streetlight
import sys

import numpy as np
from PIL import Image, ImageFilter

src, dst = sys.argv[1], sys.argv[2]
tilt = float(sys.argv[3]) if len(sys.argv) > 3 else 1.4
gain = float(sys.argv[4]) if len(sys.argv) > 4 else 1.0
falloff = float(sys.argv[5]) if len(sys.argv) > 5 else 0.0
im = Image.open(src).convert("RGB")
W, H = im.size
# crooked, then cropped back to the frame (no empty corners)
r = im.rotate(tilt, resample=Image.BICUBIC, expand=False)
k = 1 + 0.035 * abs(tilt)
cw, ch = int(W / k), int(H / k)
r = r.crop(((W - cw) // 2 + 14, (H - ch) // 2 - 6, (W - cw) // 2 + 14 + cw, (H - ch) // 2 - 6 + ch)).resize((W, H), Image.BICUBIC)
a = np.asarray(r).astype(np.float32) / 255.0
# exposure in linear light: the gain, and the streetlight's spill falling off across the frame
lin = a ** 2.2
xs = np.linspace(0, 1, W, dtype=np.float32)[None, :, None]
ys = np.linspace(0, 1, H, dtype=np.float32)[:, None, None]
lin = lin * gain * (1 - falloff * (0.75 * xs + 0.25 * ys) ** 1.3)
a = lin.clip(0, 1) ** (1 / 2.2)
rng = np.random.default_rng(4471)
lum = a.mean(axis=2, keepdims=True)
dark = np.clip(1.0 - lum * 1.6, 0.15, 1.0)
grain = rng.normal(0, 0.028, (H, W, 1)) * dark
cn = rng.normal(0, 1, (H // 4, W // 4, 3)).astype(np.float32)
cn = np.asarray(Image.fromarray(((cn * 0.25 + 0.5).clip(0, 1) * 255).astype(np.uint8)).resize((W, H), Image.BICUBIC)).astype(np.float32) / 255.0 - 0.5
a = a + grain + cn * 0.05 * dark
# the night mode lifts the blacks a little and warms them
a = a * 0.94 + np.array([0.022, 0.02, 0.026])
out = Image.fromarray((a.clip(0, 1) * 255).astype(np.uint8))
out = out.filter(ImageFilter.UnsharpMask(radius=1.2, percent=70, threshold=2))
out.save(dst, quality=80, subsampling=2)
print("phonified", dst, W, H)
