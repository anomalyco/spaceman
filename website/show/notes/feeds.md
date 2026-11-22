# Camera feed sources (public/show/feeds/)

Made with an image model (opencode/minimax-h3-image), Nov 21. Each is the still a security camera would see.
They appear only inside Vigil windows on his screen, through the sensor in `src/show/feeds.js` (its own
resolution and frame rate, noise, compression blocks, dropped frames, exposure hunting, IR, a burned-in
stamp). The film never shows these places any other way. Full-size originals are in `show/feeds/src/`; the
shipped copies are 640 px wide, JPEG, metadata stripped (458 KB together).

| File | Window | What it shows |
| --- | --- | --- |
| `cam-hall.jpg` | Hall (ws 4) | his front hall from a ceiling corner: the glazed front door, coats, the stairs, a radiator |
| `cam-kitchen.jpg` | Kitchen (ws 4) | the kitchen at night, one under-cabinet light on, the sink and kettle |
| `cam-living.jpg` | Living Room (ws 4) | the living room from a high corner: sofa, coffee table, TV, a lamp |
| `cam-landing.jpg` | Landing (ws 4) | the upstairs landing looking down the stairs, a dome camera in frame |
| `cam-back.jpg` | Back Garden (ws 4) | his back garden in IR: fence, leaves on the lawn, a bin |
| `st-08.jpg` | No. 8 (ws 5) | No. 8 across the street at night, porch light on, a car in the drive |
| `st-10.jpg` | No. 10 (ws 5) | Mark's house, No. 10: lit windows, a basketball hoop, the hedge and a bin |
| `st-10-empty.jpg` | No. 10, from June | the same frame with every window dark |
| `st-14.jpg` | No. 14 (ws 5) | No. 14 from across the street: the clipped hedge, a streetlight, one warm upstairs window |
| `st-16.jpg` | No. 16 (ws 5) | No. 16's front at night, a bench and a front garden |
| `st-16-empty.jpg` | No. 16, from March | the same frame, dark, post on the step |
| `st-16-bins.jpg` | No. 16 on a Wednesday (draft 16, ws 5) | an image-model edit (minimax-h3-image) of our own `st-16.jpg`: the same frame with two wheelie bins out at the kerb |
| `st-16-cat.jpg` | No. 16 from March (draft 16, ws 5) | an image-model edit of our own `st-16.jpg`: the house dark, post piled on the step, a cat waiting at the door |
| `st-18.jpg` | No. 18 (ws 5) | the end of the street in IR from the oldest camera: a hedge, a fir, a lamp |
| `cam-cellar.jpg` | Cellar (ws 6, the last shot) | a cellar from a ceiling corner: plastic sheeting taped to the floor, three wrapped forms side by side, shelving. Composed, not gory. |

Draft 10 of the script would make the cellar four forms; it stays at three until that draft is chosen.
