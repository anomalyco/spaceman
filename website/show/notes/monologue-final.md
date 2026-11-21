# Final monologue: "Leave it on"

Built from draft B (critique-m-B.md, astra: best voice, 8/10 straight face) with fixes from all three
critiques (critique-m-A.md, critique-m-C.md, sol).

## What it is

A cassette recording made by No. 14 at his kitchen table, near midnight on Friday, November 20.
No. 12 has walked across the street because No. 14's light was on, and because No. 14 finally
answered the re-circulated minutes, with a photograph of No. 12's study attached. No. 14 never
speaks. No. 12 explains, calmly and in order, how he keeps things. It is played back years later
(the tape hiss, the band-limit, the wow). Nothing says what happened next.

The film's picture is No. 12's screen: the story he tells, as he lived it. The first line ("your
light was on") only makes sense at the end, when the doorbell feed shows No. 14's light coming on,
the camera pulls back through No. 12's study (the chair is empty) and crosses the street to a
kitchen table: two mugs, a chair pulled out, a cassette recorder with its red lamp lit.
"Leave it on." Hard cut to "For your windows."

## Fixes taken from the critiques

- Route Chatter, not "Dana's messages" (all three). "I did read both messages" lets her unanswered
  message do the work.
- No "it's on two" after the browser went to three (all three). He goes back to check instead, and
  the picture shows ⌥2 / ⌥3 finding them.
- No six-foot-two / six-oh-two rhyme (A, C). The count stays: "A hundred and sixty mornings."
- No explanation of the reversal (B): the photograph arrives attached to No. 14's reply to the
  minutes, and he recognises his own study "from this side".
- "Are you recording this? / Good." cut (B). Only "Leave it on." remains, after a silence long
  enough for someone to reach for the recorder. It also means the light.
- The visitor's turn is a pause in the tape (room tone), not him addressing the visitor.
- No horror button lines ("All of it", "Your light's on", "I'd keep it that way").
- About 200 words, so he never has to hurry.

## Kit's character note (Nov 21)

> He's clearly hypnotized by the pure efficiency.

That is the motive and the through-line. He isn't angry or cruel; he is entranced by how well
everything works once it has a place, and he keeps applying it: to things (windows), then tasks (the
rule, the drill), then people (Dana on two, the street on five, and No. 14, "the last thing"). The
detail he lingers on is how clean it was, how nothing was ever lost again, how quickly he could get to
anything; never the thing he did. The product is what he rhapsodises about.

## Text (performance tags in brackets are not spoken)

> — your light was on. I hope that's all— I hope that's all right. I won't stay long.
>
> I want to explain how I keep things. Then it'll make sense.
>
> I'd had some of those windows open since March. February, some of them. Three of them were, uh... the
> same note. I closed two.
>
> And then everything went side by side. Nothing on top of anything. Nothing behind anything. You could
> see all of it at once... It's very clean.
>
> Mail went to two. The browser went to three. I went back to make sure, and they were just... there.
> Exactly where I'd put them. Nothing was ever lost again. There's a bar along the bottom, so you always
> know where everything is. You're never more than one key away from anything.
>
> *(the tape runs on: 6.4 s of room tone, the visitor's turn)*
>
> Then I made a rule. Chatter opens on two. You don't have to decide anymore. It just goes. I did read
> both messages.
>
> I gave Mark his drill back. Four years. It's a better drill than mine, actually. But it had a place,
> and the place was his.
>
> Four is the house. Five is the street. Nobody else wanted to take the minutes.
>
> Your hedge is six foot two. The limit is six feet. We've talked about it... twice. I've photographed it
> every morning since. A hundred and sixty mornings, and I can find any one of them in a second. Any
> morning you like.
>
> I sent the minutes round again. You're the only one who replied.
>
> There was a photograph attached. It took me a minute to recognise the study. From this side.
>
> Everything else has a place now. You're the last thing.
>
> *(an exhale)* Can I sit? ...I'll sit.
>
> *(3.2 s more: a chair, a cup set down, the tape running)*
>
> Leave it on.

## Performance

Eleven v4, Kit's PVC, one request per take with timestamps (`show/voice/mono.ts`, `mono.json`).
Three rounds of takes:

- m1–m6 (first text, stability 0.35/0.5/0.65): 62–92 s. m1 misread "were" as "with".
- n1–n4 (with a written self-correction and an "uh"): n1 had the best arc (−21 → −25 LUFS by quarter,
  pauses 0.5–1.4 s) but predates Kit's character note.
- p1–p5 (final text, after the character note): **p1 picked**: 101.9 s, loudness falls gently
  (−23.4 → −24.5 LUFS by quarter, never louder), the written stumble ("I hope that's all— I hope that's
  all right") and the exhale before "Can I sit?" both came out. p3 doubled "I won't stay long", p4 ran
  127 s, p5 got louder at the end, p2 ran 115 s.

Nobody has listened to any take. Checked by Scribe transcript (all words, no spoken tags), silence
detection and loudness per quarter and per sentence (`show/tools/voice-check.py`).

## Tape

`show/voice/process.py`: the record button (on the tape), 0.95 s of tape before he speaks, the take
with two pauses opened in the silences between sentences (6.4 s after "away from anything.", 3.2 s
after "I'll sit." with a chair and a cup), then a bed of hiss, 60 Hz hum and a kitchen, band-limited
140 Hz–5.6 kHz, a presence lift, gentle compression, wow (0.55 Hz, about ±0.14 %) and flutter (7.3 Hz),
soft saturation, two-pass loudness to −17.5 LUFS (measured −17.9 integrated, LRA 3.4; every sentence
between −16.2 and −19.7 LUFS after lifting "there.", "It's very clean." and "Leave it on.").
113.4 s, 22 kHz mono 40 kb/s.
