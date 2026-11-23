<!-- gpt-6.1-sol, 2026-11-22T05:52:37.269Z -->

**Ranked by impact, ignoring t=4 framing:**

1. **The desktop has a different tonal response from the photographed room.** At t=4, and unmistakably at 6.8, RIGHT has much blacker text/window shadows, darker wallpaper, stronger yellow stickies and creamier whites. LEFT looks like an emissive image seen through the rendered screen; RIGHT reads more like UI pasted over it. This isn’t simply excessive brightness.  
   **Fix:** Calibrate a screen-only colour/tonal transform against a gray ramp and colour patches rendered through the Cycles screen material and AgX. Match black level, white chromaticity, saturation and contrast separately. Preserve any rendered glass contribution above the insert rather than substituting bloom.

2. **The room loses fine definition, especially during the push-in.** At t=4, RIGHT’s keyboard, pens, picture and monitor surround are softer; the 100% crop confirms softer bezel/wall detail. The desktop’s stronger contrast makes that difference more conspicuous.  
   **Fix:** Compare a lossless frame through the identical browser pipeline to isolate encoding from resizing/motion blur. Test native 2880×1800 delivery for this @2x viewport, higher bitrate and fewer intermediate resizes. Don’t disguise lost detail with sharpening or grain.

3. **The added contrast/edge darkening eats the room’s shadow separation.** At 0.2, RIGHT’s chair and rug collapse further toward black; LEFT retains more material information. Near landing, the broad darkening across the desktop reads more like a finishing filter than screen behaviour.  
   **Fix:** Ease the shadow toe and vignette strength; progressively remove the room vignette as the desktop fills the view.

**Not demonstrated:** I don’t see convincing macroblocking, obvious contour banding, a conspicuous corner-pin error, or a clearly missing reflection. Grain and bloom aren’t the dominant giveaways here. A hand-off pop cannot be established from stills: check adjacent frames and converge geometry, tone, blur and grain *before* switching to flat drawing.

**RIGHT is better:** Desktop legibility and window hierarchy are substantially stronger. Keep that clarity for the landed, interactive desktop; introduce it gradually rather than imposing it throughout the photographed-screen portion.
