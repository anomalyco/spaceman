# The phone photos in his album (public/show/photos/)

Made with an image model (opencode/minimax-h3-image), Nov 22 (draft 18), from two references: the
earlier hedge photo (a Blender render of our street as its own reference) for the daylight ones, and
No. 14's camera still (`feeds/st-14.jpg`) for dusk and night. They appear only inside the Photos window
on his screen. Each was downscaled to 720 px (Pillow, JPEG q80); the dusk zoom and the night ones get
added sensor grain. The grid crops every tile differently (zoomed and slid along the frame) and never
puts the same photo next to itself, the row above or two rows up (`library.js assign`).

| File | Prompt, in short |
| --- | --- |
| `d-sun.jpg` | bright cold winter morning, low sun from the left, long shadows, straight on, a degree crooked |
| `d-angle.jpg` | steep angle down the length of the hedge, flat overcast December morning, damp pavement |
| `d-frost.jpg` | close-up: frost on the clipped top of the hedge at dawn, the house soft behind |
| `d-drizzle.jpg` | light drizzle, wet pavement, raindrops blurring a corner of the lens |
| `d-tape.jpg` | a hand holding a yellow tape measure against the hedge, the top at 6 ft 2 in |
| `d-road.jpg` | from across the road, wider, a parked hatchback cut off at the left, blue sky |
| `d-shoots.jpg` | close-up: unruly shoots above the clipped top, white sky |
| `d-corner.jpg` | low angle at the hedge's corner, warm afternoon sun raking along it |
| `d-fog.jpg` | foggy morning, the far end softening into grey |
| `d-leaves.jpg` | windy January afternoon, leaves blowing along the pavement, a bin at the kerb |
| `d-snow.jpg` | a dusting of snow on the hedge's flat top, slushy pavement |
| `d-car.jpg` | through a parked car's windscreen, dashboard edge in the corner |
| `e-gold.jpg` | golden hour, sun low behind the house, lens flare |
| `e-blue.jpg` | blue hour, the streetlight just on, every window dark |
| `e-zoom.jpg` | blurry digital zoom on the front door at dusk |
| `n-rain.jpg` | night mode from across the street in light rain, all windows dark |
| `n-light.jpg` | grainy 3x night shot up at the house, one upstairs window lit, nobody in it |
| `n-alan.jpg` | the last photo: Dax at his lit window facing the lens, looking straight back; cropped to a telephoto frame, softened like a digital zoom, blacks crushed, heavy grain. Since draft 23 it is Dax Raad, with his blessing: an image-model edit (minimax-h3-image) of the old photo with two reference photos of him, then halved and scaled back up, desaturated a little, levelled and grained in ImageMagick |

Rejected: a second take of the last photo (a19) that read as a doorway and looked posed. Retired
with draft 18: the summer hedge mornings (`h-*`), the drill, the label, the plastic in the boot,
No. 16, and the old night shots; the grid that alternated two of them is gone with them.
