# Textures

Every texture the scenes use comes from [Poly Haven](https://polyhaven.com) under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (public domain, no attribution
required; credited here anyway). `../textures.py` fetches the 2k JPEG maps into this folder on first
use; the images themselves aren't committed. No image-model textures.

| Texture | Poly Haven id | Author | Tile size | Maps | Used for |
| --- | --- | --- | --- | --- | --- |
| Wood Floor | [wood_floor](https://polyhaven.com/a/wood_floor) | Dimitrios Savva | 1.70 m | diff, rough, disp | study floorboards |
| Painted Plaster Wall | [painted_plaster_wall](https://polyhaven.com/a/painted_plaster_wall) | Amal Kumar | 2.00 m | diff, rough, disp | walls (tinted to the paint) |
| Plastered Wall 04 | [plastered_wall_04](https://polyhaven.com/a/plastered_wall_04) | Rob Tuytel | 3.20 m | diff, rough, disp | ceiling |
| Oak Veneer 01 | [oak_veneer_01](https://polyhaven.com/a/oak_veneer_01) | Jenelle van Heerden | 1.83 m | diff, rough, disp | desk top (stained to walnut), corkboard frame |
| Poly Wool Herringbone | [poly_wool_herringbone](https://polyhaven.com/a/poly_wool_herringbone) | colormass, Rico Cilliers | 0.27 m | diff, rough, disp | rug |
| Knitted Fleece | [knitted_fleece](https://polyhaven.com/a/knitted_fleece) | colormass, Rico Cilliers | 0.27 m | diff, rough, disp | chair upholstery |

Everything else (paint, metal, plastic, glass, card, paper) is a procedural Blender material.

The paper on the desk and the corkboard (`study/clues/*.png`: the legal pad, the sticky note, the index
cards) is drawn by `study/clues.py` with Pillow. The handwriting is
[Caveat](https://github.com/googlefonts/caveat) (SIL Open Font License 1.1), fetched to a local cache at
build time. Only the rendered PNGs are committed, not the font.
