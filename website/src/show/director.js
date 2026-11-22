// The film, as cues on a clock (seconds after the key press, which is tape time). Cues fire once,
// in order, as the clock passes them; spans run every frame between two times. Picture beats are
// pinned to the words of the tape (tape.js), so the cut follows the voice exactly. Everything here
// only calls the world's verbs, so the same timeline drives WebGPU, Canvas 2D, capture and reduced
// motion.
import { easeInOut, smooth, clamp, mix } from "./springs.js"
import { BAR_H } from "./content.js"
import { WORDS, TAPE } from "./tape.js"
import { MORNINGS, PICK, gridOf, indexOf, heightAt } from "./library.js"

// the visitor's turn (Option+1/2/3 really switch) is the first long silence on the tape
const G0 = TAPE.marks.gaps.find((g) => g[1] > 5)
export const HOLD = [G0[0], G0[0] + G0[1]]
// the silence after "Nothing behind anything.": what happens downstairs
const ACT = TAPE.marks.gaps.filter((g) => g[1] > 3).at(-1)

const norm = (s) => s.toLowerCase().replace(/[^a-z0-9']/g, "")
const starts = (phrase) => {
  const ws = phrase.split(/\s+/).map(norm)
  const out = []
  for (let i = 0; i < WORDS.length; i++) if (ws.every((x, j) => WORDS[i + j] && norm(WORDS[i + j][0]) === x)) out.push(WORDS[i][1])
  return out
}
// When a phrase starts on the tape. Lookups read the tape in order: each takes the next match within
// half a minute of the previous lookup, else the nearest one before it, so a repeated phrase ("I
// closed two", "Side by side") resolves to the right occurrence without hard-coded times. A numeric
// `after` takes the first match from that time instead.
let cursor = 0
export function word(phrase, after) {
  const all = starts(phrase)
  if (after !== undefined) {
    const t = all.find((t) => t >= after)
    if (t === undefined) throw new Error("tape has no " + phrase + " after " + after)
    return t
  }
  const next = all.find((t) => t >= cursor - 1.5 && t < cursor + 30)
  if (next !== undefined) {
    cursor = Math.max(cursor, next)
    return next
  }
  const prev = all.filter((t) => t < cursor).at(-1)
  if (prev === undefined) throw new Error("tape has no " + phrase)
  return prev
}
// when a phrase ends
export function wordEnd(phrase, after) {
  const t = word(phrase, after)
  const n = phrase.split(/\s+/).length
  const i = WORDS.findIndex((w) => w[1] === t)
  return WORDS[i + n - 1][2]
}
const LAST = WORDS[WORDS.length - 1]
// the film ends on six, a held beat after the last word; the interview runs on under the card
export const END = LAST[2] + 2.6

const NOTE = ["call about the gutters", "drill → return", "passwords??", "organize notes", "hedge (ask)"]

export function pile(SW, SH) {
  const W = (f) => f * SW
  const H = (f) => f * SH
  const tall = SH > SW
  const F = (x, y, w, h) => (tall ? [W(y * 0.9), H(0.06 + x * 0.8), W(Math.min(0.92, h * 1.5)), H(w * 0.62)] : [W(x), H(y), W(w), H(h)])
  return [
    { id: "browsy", app: "browsy", kind: "browser", title: "Gutter guards: which kind? — Browsy", at: F(0.38, 0.08, 0.5, 0.62), data: { tabs: 38, url: "homedesk.example/gutter-guards-which-kind", heading: "Gutter guards: which kind?" } },
    { id: "postbox", app: "postbox", kind: "mail", title: "Inbox — 4,812 messages", at: F(0.04, 0.1, 0.48, 0.5), data: mail() },
    { id: "shot", app: "viewer", kind: "shot", title: "Screenshot 2026-03-02 at 11.48.07 PM", at: F(0.5, 0.42, 0.36, 0.4) },
    { id: "recipes", app: "paper", kind: "paper", title: "Recipes", at: F(0.62, 0.16, 0.3, 0.44), data: { heading: "Recipes", sub: "6 recipes", lines: ["Chili (Mom's)", "Banana bread", "Lasagna — the good one", "Pancakes, Sunday", "Lemon chicken", "Soda bread"] } },
    { id: "daybook", app: "daybook", kind: "daybook", title: "Daybook — June", at: F(0.12, 0.36, 0.32, 0.5), data: { date: "Tuesday, June 2", events: [{ t: "Trash out", at: 19, len: 0.7, color: "#d9e3cf" }, { t: "HOA meeting — June", at: 8, len: 1, color: "#d6dff0" }] } },
    { id: "untitled", app: "jot", kind: "jot", title: "Untitled 47", at: F(0.3, 0.3, 0.3, 0.36), data: { heading: "Untitled 47", edited: "Edited February 9 at 11:02 PM", lines: ["call about the gutters?", "ask Mark re: drill", "hedge — measure?"] } },
    { id: "drill", app: "jot", kind: "jot", title: "return drill", at: F(0.68, 0.5, 0.27, 0.34), data: { heading: "return drill", check: 1, lines: ["to No. 10 (Mark)", { t: "borrowed spring 2022", soft: 1, nobox: 1 }] } },
    { id: "notes2", app: "jot", kind: "jot", title: "notes — real", at: F(0.2, 0.2, 0.3, 0.42), data: { heading: "notes", edited: "Edited April 30", lines: NOTE } },
    { id: "notes1", app: "jot", kind: "jot", title: "notes (1)", at: F(0.1, 0.46, 0.3, 0.42), data: { heading: "notes", edited: "Edited March 14", lines: NOTE } },
    { id: "notes", app: "jot", kind: "jot", title: "notes", at: F(0.44, 0.24, 0.3, 0.42), data: { heading: "notes", edited: "Edited March 2", lines: NOTE } },
    { id: "sticky", app: "jot", kind: "sticky", title: "", at: F(0.78, 0.07, 0.17, 0.2), data: { text: "organize notes" } },
  ]
}

function mail() {
  const list = [
    { from: "Pinecrest HOA", subject: "Reminder: hedge and fence heights", when: "9:02 PM", unread: 1 },
    { from: "Dana", subject: "friday?", when: "8:40 PM", unread: 1 },
    { from: "Orders", subject: "Your order has shipped", when: "6:15 PM", unread: 1 },
    { from: "Pinecrest HOA", subject: "Minutes needed for June meeting", when: "Mon" },
    { from: "City of Pinecrest", subject: "Collection day changes", when: "Mon", unread: 1 },
    { from: "Weekly", subject: "Your week in photos", when: "Sun" },
    { from: "Mark (No. 10)", subject: "re: drill", when: "Sat" },
    { from: "Pinecrest HOA", subject: "Volunteers", when: "Fri", unread: 1 },
  ]
  return { list, sel: 0, open: { subject: "Reminder: hedge and fence heights", from: "Pinecrest HOA", when: "9:02 PM", body: ["Hedges and fences along the front", "may not exceed six feet.", "", "Thank you for keeping Pinecrest", "looking its best."] } }
}

// The grade follows the story: a warm, tired night; clean light once things have places; mornings;
// the months cooling; a cold November night; six.
const LK = {
  night: { exposure: 0.93, warmth: 0.35, saturation: 0.92, contrast: 1.02, vignette: 0.3, bloom: 0.16, g: [0, 0, 0] },
  clean: { exposure: 1.0, warmth: 0.12, saturation: 1.0, contrast: 1.03, vignette: 0.22, bloom: 0.16, g: [0, 0, 0] },
  morning: { exposure: 1.04, warmth: 0.04, saturation: 1.02, contrast: 1.03, vignette: 0.2, bloom: 0.18, g: [0, 0, 0] },
  house: { exposure: 0.97, warmth: 0.26, saturation: 0.96, contrast: 1.03, vignette: 0.26, bloom: 0.16, g: [0, 0, 0] },
  hedge: { exposure: 1.02, warmth: 0.1, saturation: 0.98, contrast: 1.03, vignette: 0.24, bloom: 0.16, g: [0, 0, 0] },
  cool: { exposure: 0.95, warmth: -0.2, saturation: 0.82, contrast: 1.05, vignette: 0.28, bloom: 0.16, g: [0, 0.2, 0.05] },
  nov: { exposure: 0.9, warmth: -0.3, saturation: 0.82, contrast: 1.05, vignette: 0.3, bloom: 0.17, g: [0.05, 0.35, 0.08] },
  six: { exposure: 0.84, warmth: -0.5, saturation: 0.7, contrast: 1.08, vignette: 0.38, bloom: 0.18, g: [0.14, 0.5, 0.1] },
  room: { exposure: 1.0, warmth: 0.05, saturation: 0.95, contrast: 1.02, vignette: 0.32, bloom: 0.16, g: [0, 0, 0] },
}
// The grade follows the story: a warm, tired night; clean light once things have places; mornings;
// the months cooling; a cold November night; six.
let LOOKS = null
function looks(T) {
  return [
    [0, LK.room], [T.intro - 1.5, LK.room], [T.intro, LK.night], [T.install, LK.night], [T.tiled, LK.clean], [T.hold0, LK.clean], [T.hold1, LK.morning], [T.house - 0.3, LK.morning], [T.house + 0.7, LK.house],
    [T.hedge - 0.4, LK.house], [T.hedge + 1, LK.hedge], [T.again - 2, LK.cool], [T.again + 1.5, LK.nov], [T.six - 0.2, LK.nov], [T.six + 0.8, LK.six],
    [T.place, LK.six], [1e4, LK.six],
  ]
}

function applyLook(S, t) {
  if (!LOOKS) return
  let i = 0
  while (i + 2 < LOOKS.length && LOOKS[i + 1][0] <= t) i++
  const [t0, a] = LOOKS[i]
  const [t1, b] = LOOKS[i + 1]
  const u = smooth((t - t0) / (t1 - t0))
  for (const k of ["exposure", "warmth", "saturation", "contrast", "vignette", "bloom"]) S.fx[k] = mix(a[k], b[k], u)
  for (let j = 0; j < 3; j++) S.grade[j] = mix(a.g[j], b.g[j], u)
}

// A camera move between two views as one zoom about the point they agree on (no dog-legs).
function zoomView(A, B, e) {
  const lk = Math.log(B.D / A.D)
  if (Math.abs(lk) < 0.02) return { cx: mix(A.cx, B.cx, e), cy: mix(A.cy, B.cy, e), D: mix(A.D, B.D, e) }
  const inv = 1 / A.D - 1 / B.D
  const px = (A.cx / A.D - B.cx / B.D) / inv
  const py = (A.cy / A.D - B.cy / B.D) / inv
  const D = A.D * Math.exp(lk * e)
  return { cx: px - ((px - A.cx) * D) / A.D, cy: py - ((py - A.cy) * D) / A.D, D }
}

// A hand on a keyboard: uneven gaps, a hesitation before the first key.
function rhythm(n, seed = 1) {
  const out = []
  let t = 0
  for (let i = 0; i < n; i++) {
    const h = Math.abs(Math.sin((i + 1) * 12.9898 * seed) * 43758.5453) % 1
    t += i === 0 ? 0 : 0.075 + h * 0.08 + (i === 1 ? 0.05 : 0)
    out.push(t)
  }
  return out
}

export function build(S) {
  const { world: w } = S
  const cues = []
  const spans = []
  const at = (t, fn) => cues.push({ t, fn })
  const span = (t0, t1, fn) => spans.push({ t0, t1, fn })
  const R = (id) => w.wins.get(id)
  const closeBtn = (id) => {
    const x = R(id)
    if (!x) throw new Error("no window " + id)
    return [x.x.t + 16, x.y.t + BAR_H / 2]
  }
  const titleOf = (id) => {
    const x = R(id)
    return [x.x.t + x.w.t * 0.42, x.y.t + BAR_H / 2]
  }
  // The cursor sets off in time to arrive a beat before the click, never cutting short the last one.
  let lastClick = -10
  const clickAt = (t, id, where = titleOf, fn, maxLead = 1.2) => {
    const lead = Math.min(maxLead, t - lastClick - 0.08)
    lastClick = t
    at(t - lead, () => {
      const p = typeof where === "function" ? where(id) : where
      w.cursorTo(p[0], p[1], t - 0.2)
    })
    at(t, () => {
      w.click()
      S.sfx("key2", 0.28)
      fn?.()
    })
  }
  const key = (t, label, fn) =>
    at(t, () => {
      w.keyCast(label)
      S.sfx("key", 0.7)
      fn?.()
    })
  const sw = (t, n, clock) =>
    at(t, () => {
      if (clock) w.clock = clock
      w.switchTo(n)
      S.sfx("key", 0.6)
    })
  const type = (t0, s, fn, seed = 1) =>
    rhythm(s.length, seed).forEach((dt, i) =>
      at(t0 + dt, () => {
        fn(s.slice(0, i + 1), i === s.length - 1)
        S.sfx(i % 2 ? "key" : "key2", 0.22)
      }),
    )
  const frameOf = (id, margin = 1.1) => {
    const x = R(id)
    const D = Math.min(1, Math.max(x.w.t / S.SW, x.h.t / S.SH) * margin)
    // keep the view inside the screen: never show what's past its edge
    const hx = (S.SW * D) / 2
    const hy = (S.SH * D) / 2
    return { cx: clamp(x.x.t + x.w.t / 2, hx, S.SW - hx), cy: clamp(x.y.t + x.h.t / 2, hy, S.SH - hy), D }
  }
  // One camera move from wherever it is to a framing, over [t0, t1]: one eased progress value.
  const push = (t0, t1, target, focus) => {
    let A = null
    let B = null
    span(t0, t1, (u) => {
      if (!A) {
        A = { ...S.cam }
        B = typeof target === "function" ? target() : target
        if (focus !== undefined) S.look.focus = focus
      }
      const e = S.reduced ? (u < 0.5 ? 0 : 1) : easeInOut(u)
      Object.assign(S.cam, zoomView(A, B, e))
    })
  }
  const home = () => ({ cx: S.SW / 2, cy: S.SH / 2, D: 1 })
  const open = (t, ws, def, focus = true) =>
    at(t, () => {
      w.open({ ws, ...def, at: def.at || [S.SW * 0.3, S.SH * 0.3, S.SW * 0.4, S.SH * 0.4] }, { focus })
      S.sfx("thunk", 0.3)
    })

  const openMany = (t, ws, defs) =>
    at(t, () => {
      w.openMany(defs.map((d) => ({ ws, ...d })))
      S.sfx("thunk", 0.3)
    })

  // a camera window in Vigil; its stamp is set per scene by feedsAt
  const CAM = (id, src, title, data) => ({ id, app: "vigil", kind: "feed", title, dark: 1, data: { src, name: title.toUpperCase(), us: 1, ...data } })
  // every camera's burned-in clock, as of now in the story (date mm-dd-yyyy, seconds after midnight)
  const feedsAt = (date, secs) => {
    for (const x of w.wins.values()) if (x.kind === "feed") w.set(x.id, { data: { date, clock: secs - w.time } })
  }

  span(0, END, (u, t) => applyLook(S, t))
  const BAR_BASE = 1.5
  const BAR_BIG = 2.2

  // A camera move through the room: a → b over [t0, t1], eased (cut at the middle when reduced).
  // A move that ends on the screen hands over to the flat desktop as it lands.
  const move = (t0, t1, a, b, e = easeInOut) =>
    span(t0, t1, (u) => {
      if (b === "screen" && u >= 1) return S.flat()
      S.shot(a, b, S.reduced ? (u < 0.5 ? 0 : 1) : e(u))
    })
  // ── 0 · The study at night; the tone; the screen wakes; in through it ──
  cursor = 0
  const intro = TAPE.marks.head + 0.7
  at(0, () => {
    w.bar.scale.x = w.bar.scale.t = 1.5
    S.three.glow = 0
    S.shot("CamChair", "CamChair", 0)
  })
  // the screen wakes as the recorder's tone sounds, with a panel's small settle
  span(0.42, 0.95, (u) => (S.three.glow = smooth(u) * (1 + 0.06 * Math.sin(u * 9) * (1 - u))))
  at(0.42, () => S.sfx("key2", 0.2))
  move(1.2, intro, "CamChair", "screen", (u) => easeInOut(u) ** 1.15)
  at(intro, () => S.flat())
  cursor = TAPE.marks.head
  const march = word("I'd had some")
  push(intro + 0.05, march - 0.2, { cx: S.SW * 0.52, cy: S.SH * 0.5, D: 0.93 })
  at(2.2, () => w.cursorTo(S.SW * 0.56, S.SH * 0.6, 3.2))
  at(word("I want to explain", TAPE.marks.head) + 0.6, () => w.cursorTo(S.SW * 0.5, S.SH * 0.56, march - 0.6))
  // "...open since March": the March screenshot of this same mess
  clickAt(march + 0.5, "shot", titleOf, () => w.setFocus("shot"))
  push(march, march + 1.4, () => frameOf("shot", 1.45), "shot")
  const feb = word("February")
  clickAt(feb + 0.25, "untitled", titleOf, () => w.setFocus("untitled"))
  push(feb + 0.1, feb + 1.2, () => frameOf("untitled", 1.5), "untitled")
  // "Three of them were, uh... the same note": each raised in turn
  const three = word("Three of them")
  push(three - 0.1, three + 1.0, { cx: S.SW * 0.5, cy: S.SH * 0.5, D: 0.95 }, null)
  clickAt(three + 0.35, "notes", titleOf, () => w.setFocus("notes"))
  clickAt(three + 1.0, "notes1", titleOf, () => w.setFocus("notes1"))
  clickAt(three + 1.65, "notes2", titleOf, () => w.setFocus("notes2"))
  // "I closed two."
  clickAt(word("closed") + 0.02, "notes1", closeBtn, () => (w.close("notes1"), S.sfx("close", 0.7)))
  clickAt(word("two") + 0.05, "notes2", closeBtn, () => (w.close("notes2"), S.sfx("close", 0.7)))

  // ── Spaceman: installed, then everything side by side ──
  const install = word("Then I installed")
  const named = word("Spaceman")
  const tiled = word("tiled themselves")
  // up to the corner of the menu bar, where its item appears on its name, then back for the tiling
  push(install + 0.15, named - 0.15, () => ({ cx: S.SW * 0.83, cy: S.SH * 0.17, D: 0.34 }), null)
  at(named - 0.05, () => {
    w.status = true
    w.statusIn = w.time
    S.sfx("key2", 0.45)
  })
  push(named + 0.9, tiled - 0.1, home)
  at(tiled, () => {
    w.tileNow()
    S.sfx("thunk", 0.8)
  })
  at(tiled + 0.7, () => {
    w.ensureCell(3)
    w.showBar()
  })
  at(tiled + 0.2, () => w.cursorTo(S.SW * 0.6, S.SH * 0.7, tiled + 1.2))
  const flesh = word("I tasted")
  push(word("Side by side") + 0.2, flesh + 1.4, { cx: S.SW * 0.5, cy: S.SH * 0.47, D: 0.94 })

  // ── Moving, then switching ──
  const mail2 = word("Mail went to workspace two")
  push(mail2 - 0.9, mail2 - 0.1, home)
  clickAt(mail2 - 0.55, "postbox", titleOf, () => w.setFocus("postbox"))
  key(mail2 + 0.35, ["⌥", "⇧", "2"], () => w.move("postbox", 2))
  const brow = word("browser went")
  clickAt(brow - 0.4, "browsy", titleOf, () => w.setFocus("browsy"))
  key(brow + 0.15, ["⌥", "⇧", "3"], () => w.move("browsy", 3))
  sw(word("back to make"), 2)
  sw(word("they were just") + 0.25, 3)
  sw(word("Exactly where") + 0.5, 1)

  // ── The bar ──
  const bar = word("bar along")
  at(bar - 0.2, () => (w.bar.scale.t = BAR_BIG))
  // its sentences sit high on the screen while the camera is down at the bar
  S.capHigh = [word("There's a bar") - 0.1, word("You're never more") + 0.1]
  at(bar - 0.1, () => (w.keysAbove = true))
  at(HOLD[0] - 0.9, () => (w.keysAbove = false))
  push(bar - 0.1, bar + 1.2, () => {
    // close enough to read the cells, never so close that a narrow (phone) screen crops the bar's ends
    const g = w.barGeom()
    const D = Math.min(1, Math.max(0.42, ((g.w / g.sc) * BAR_BIG * 1.1) / S.SW))
    return { cx: S.SW / 2, cy: S.SH - (S.SH * D) / 2, D }
  })
  // "You're never more than one key away": 2, then 3, a beat each, then home before the visitor's turn
  sw(word("never more") - 0.05, 2)
  sw(word("key away") - 0.05, 3)
  sw(HOLD[0] - 0.25, 1)
  push(HOLD[0] - 0.9, HOLD[0], home)

  // ── The visitor's turn ──
  at(HOLD[0] + 0.2, () => S.hint(true))
  at(HOLD[1] - 0.4, () => {
    S.hint(false)
    if (w.active !== 1) w.switchTo(1)
    w.bar.scale.t = BAR_BASE
  })

  // ── A rule: Chatter opens on 2 ──
  const settingsAt = () => {
    const sw = Math.min(S.SW * 0.86, 860)
    const sh = Math.min(S.SH * 0.6, 380)
    return [S.SW / 2 - sw / 2, S.SH * 0.44 - sh / 2, sw, sh]
  }
  const settingsFrame = () => {
    const [x, y, ww, hh] = settingsAt()
    return { cx: x + ww / 2, cy: y + hh / 2, D: Math.min(1, Math.max(ww / S.SW, hh / S.SH) * 1.14) }
  }
  // the table's columns, as the settings painter lays them out
  const cell = (col) => {
    const [x, y, ww] = settingsAt()
    const f = [0, 0.27, 0.6, 0.8][col]
    return [x + 184 + f * (ww - 206) + 30, y + BAR_H + 117]
  }
  const rule0 = word("Then I made a rule")
  at(rule0 - 0.25, () => {
    w.clock = "Wed Jun 3  7:10 AM"
    w.open({ id: "settings", app: "spaceman", kind: "settings", title: "Spaceman Settings", float: true, at: settingsAt(), data: { rules: [], add: 1 } })
    S.sfx("thunk", 0.3)
  })
  push(rule0 - 0.15, rule0 + 0.8, settingsFrame, "settings")
  clickAt(rule0 + 0.75, "settings", () => {
    const [x, y] = settingsAt()
    return [x + 230, y + BAR_H + 122]
  }, () => w.set("settings", { data: { add: 0, rules: [{ app: "", action: "", ws: null, cursor: 0 }] } }))
  const chatter = word("Chatter opens")
  type(chatter - 0.1, "Chatter", (s, done) => w.set("settings", { data: { rules: [{ app: s, icon: done ? "chatter" : null, action: "", ws: null, cursor: 0 }] } }))
  clickAt(word("on two") - 0.05, "settings", () => cell(3), () => w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Open on", ws: null, cursor: 3 }] } }), 0.6)
  at(word("two") + 0.12, () => {
    w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Open on", ws: 2, hl: 1 }] } })
    S.sfx("key", 0.3)
  })
  const decide = word("You don't have to decide")
  push(decide - 0.15, decide + 0.75, home, null)
  clickAt(decide + 0.3, "settings", closeBtn, () => (w.close("settings"), S.sfx("close", 0.5)))
  // a new Chatter window opens straight onto 2: he stays on 1, and it appears in 2's place in the bar
  const goes = word("It just goes")
  at(goes - 0.15, () => {
    w.openMany([{ ws: 2, id: "dana", app: "chatter", kind: "chat", title: "Dana", data: { who: "Dana", when: "Today 6:12 AM", msgs: [{ t: "are we still on for friday?" }, { t: "you can say no", note: "Read 6:14 AM" }] } }])
    w.pulse(2)
    S.sfx("thunk", 0.3)
  })
  // "I did read both messages."
  const read = word("did read both")
  sw(read - 0.05, 2)
  push(read + 0.15, read + 1.0, () => frameOf("dana", 1.3), "dana")
  // "I gave Mark his drill back."
  const gave = word("gave Mark")
  sw(gave - 0.1, 1, "Sat Jun 6  10:05 AM")
  push(gave - 0.05, gave + 0.85, () => frameOf("drill", 1.5), "drill")
  clickAt(word("back"), "drill", (id) => [R(id).x.t + 34, R(id).y.t + BAR_H + 72], () => w.set("drill", { data: { check: 1, lines: [{ t: "to No. 10 (Mark)", done: 1 }, { t: "borrowed spring 2022", soft: 1, nobox: 1 }] } }))
  const butIt = word("But it had")
  push(word("Four years") - 0.5, butIt + 0.4, () => ({ ...frameOf("drill", 1.36) }))
  const his = word("was his") + 0.25
  push(his - 0.95, his - 0.1, home, null)
  // "...the place was his.": the note goes, and the others close the gap
  clickAt(his, "drill", closeBtn, () => (w.close("drill"), S.sfx("close", 0.6)))

  // ── 4 · The house ──
  const house = word("Four is the house")
  sw(house, 4, "Tue Jun 9  9:30 PM")
  // one camera window per room (not the cellar), in Vigil
  openMany(house + 0.25, 4, [
    CAM("c-hall", "cam-hall", "Hall", { ir: 1, q: 0.35, seed: 3 }),
    CAM("c-kitchen", "cam-kitchen", "Kitchen", { q: 0.45, seed: 5 }),
    CAM("c-living", "cam-living", "Living Room", { ir: 1, q: 0.35, seed: 7 }),
    CAM("c-landing", "cam-landing", "Landing", { ir: 1, q: 0.4, seed: 11 }),
    CAM("c-back", "cam-back", "Back Garden", { ir: 1, q: 0.5, seed: 13, life: [{ moth: [0.5, 0.18] }, { leaves: 6 }] }),
  ])
  at(house + 0.3, () => feedsAt("06-09-2026", 21 * 3600 + 30 * 60 + 4))

  // ── 5 · The street ──
  const street = word("Five is the street")
  sw(street + 0.05, 5)
  // one camera per house, in house-number order (he is No. 12), and the minutes
  openMany(street + 0.3, 5, [
    CAM("f08", "st-08", "No. 8", { q: 0.45, seed: 17, life: [{ sweep: [0, 0.78], every: 19 }] }),
    CAM("f10", "st-10", "No. 10", { ir: 1, q: 0.4, seed: 19 }),
    CAM("f14", "st-14", "No. 14", { q: 0.3, seed: 23 }),
    CAM("f16", "st-16", "No. 16", { q: 0.45, seed: 29 }),
    CAM("f18", "st-18", "No. 18", { ir: 1, q: 0.85, seed: 31, lost: [[street + 6.5, street + 8.1], [street + 41, street + 42.2]] }),
    { id: "minutes", app: "paper", kind: "paper", title: "HOA — Minutes", data: { heading: "Pinecrest HOA — Minutes", sub: "Meeting of June 9 · taken by No. 12", size: 14, gap: 23, lines: ["Present: 9 of 9", "1. Trash cans out of view by 8 PM", "    on collection days.", "2. Hedges and fences: six feet maximum.", "3. Minutes: No. 12."] } },
  ])
  at(street + 0.35, () => feedsAt("06-09-2026", 21 * 3600 + 31 * 60 + 2))
  const mins = word("Nobody else wanted")
  at(mins, () => w.setFocus("minutes"))
  push(mins + 0.1, mins + 1.1, () => frameOf("minutes", 1.2), "minutes")
  const hedge = word("fourteen's hedge")
  push(hedge - 0.85, hedge, home, null)

  // ── The hedge ──
  // His photo library, open on the first morning: June 11, measured.
  const first = indexOf(0)
  open(hedge, 5, { id: "hedge", app: "viewer", kind: "photos", title: "Photos", data: { count: 1, open: first, openT: 1 } })
  at(hedge, () => (w.clock = dayClock(0)))
  push(hedge + 0.2, hedge + 1.1, () => frameOf("hedge", 1.04), "hedge")
  at(word("six foot two") + 0.1, () => (w.set("hedge", { data: { measured: 1, measure: "6′ 2″" } }), S.sfx("shutter", 0.35)))
  at(word("The limit") + 0.2, () => w.set("hedge", { data: { limit: 1 } }))
  // "I've photographed it every morning since." The photo closes into its tile, and the mornings
  // fill the library, the newest always at the bottom. A hundred and sixty of them.
  const since = word("photographed")
  span(since - 0.1, since + 0.45, (u) => w.set("hedge", { data: { openT: 1 - (S.reduced ? (u < 0.5 ? 0 : 1) : easeInOut(u)) } }))
  at(since + 0.46, () => w.set("hedge", { data: { open: null, measured: 0, limit: 0 } }))
  let shots = 1
  span(since + 0.35, since + 3.4, (u) => {
    const n = 1 + Math.floor((MORNINGS - 1) * u ** 1.7)
    if (n !== shots) {
      if (n <= 4 || n === 24 || n === 70) S.sfx("shutter", n <= 4 ? 0.35 : 0.25)
      shots = n
      w.set("hedge", { data: { count: n } })
      w.clock = dayClock(n - 1)
    }
  })
  // "...find any one of them in a second." Two flicks up through November and October, each one
  // a throw that slows on its own (velocity decaying exponentially), the second landing on August.
  const find = word("find any one")
  const grid = () => gridOf(R("hedge").w.t, R("hedge").h.t, MORNINGS)
  const pickAt = indexOf(PICK)
  const flick = (t0, t1, tau, to) => {
    let from = null
    let goal = null
    span(t0, t1, (u, t) => {
      if (from == null) {
        const g = grid()
        const cur = R("hedge").data.scroll
        from = cur == null ? g.maxScroll : cur
        goal = to(g, from)
      }
      const k = (1 - Math.exp(-(Math.min(t, t1) - t0) / tau)) / (1 - Math.exp(-(t1 - t0) / tau))
      w.set("hedge", { data: { scroll: S.reduced ? (u < 0.5 ? from : goal) : from + (goal - from) * k, bar: 1 } })
    })
  }
  at(find - 0.6, () => {
    const p = R("hedge")
    w.cursorTo(p.x.t + p.w.t * 0.56, p.y.t + p.h.t * 0.6, find - 0.1)
  })
  flick(find + 0.05, find + 1.2, 0.36, (g, from) => g.scrollTo(pickAt) + (from - g.scrollTo(pickAt)) * 0.16)
  flick(find + 1.25, word("Any morning") - 0.15, 0.24, (g) => g.scrollTo(pickAt))
  span(word("Any morning") - 0.15, word("Any morning") + 0.5, (u) => w.set("hedge", { data: { bar: 1 - smooth(u) } }))
  // "Any morning you like." August 14: the click, and the photo opens out of its tile.
  const anyM = word("Any morning")
  const tileAt = () => {
    const p = R("hedge")
    const g = grid()
    const r = g.tile(pickAt, p.data.scroll ?? g.scrollTo(pickAt))
    return [p.x.t + r.x + r.w * 0.5, p.y.t + r.y + r.h * 0.55]
  }
  clickAt(anyM + 0.05, "hedge", tileAt, () => w.set("hedge", { data: { sel: pickAt } }), 0.55)
  at(anyM + 0.14, () => {
    w.set("hedge", { data: { open: pickAt, openT: 0, measure: heightAt(PICK), measured: 1, limit: 1 } })
    w.clock = dayClock(MORNINGS - 1)
  })
  span(anyM + 0.14, anyM + 0.52, (u) => w.set("hedge", { data: { openT: S.reduced ? (u < 0.5 ? 0 : 1) : 1 - (1 - u) ** 3 } }))

  // ── The minutes, again ──
  const again = word("sent the minutes round")
  push(again - 0.5, again + 0.4, home, null)
  at(again - 0.1, () => {
    w.clock = "Wed Nov 18  9:12 PM"
    w.setFocus("minutes")
    w.set("minutes", { title: "HOA — Minutes (awaiting corrections)", data: { banner: ["Sent to 8 households · Jun 10"] } })
  })
  at(again + 0.75, () => (w.set("minutes", { data: { banner: ["Sent to 8 households · Jun 10", "Sent again to 8 households · Nov 18"] } }), S.sfx("thunk", 0.2)))

  // ── The only reply ──
  const only = word("He was the only")
  sw(only - 0.1, 2, "Fri Nov 20  7:41 PM")
  const reply = { from: "No. 14", subject: "Re: Minutes — corrections", when: "7:41 PM", unread: 1, clip: 1 }
  at(only + 0.25, () => {
    const d = R("postbox").data
    w.set("postbox", { data: { list: [reply, ...d.list], sel: -1, slide: 0 } })
    S.sfx("thunk", 0.25)
  })
  span(only + 0.25, only + 0.6, (u) => w.set("postbox", { data: { slide: smooth(u) } }))
  push(only + 0.1, only + 1.0, () => frameOf("postbox", 1.12), "postbox")
  const attached = word("photograph attached")
  clickAt(only + 0.95, "postbox", (id) => [R(id).x.t + 120, R(id).y.t + BAR_H + 34], () => w.set("postbox", { data: { sel: 0, open: { subject: reply.subject, from: "No. 14 · to No. 12", when: "Today 7:41 PM", body: [], attach: "IMG_4471.jpg" } } }))
  // the chip sits under the (empty) message
  const chip = () => {
    const p = R("postbox")
    const listW = Math.min(320, p.w.t * 0.44)
    return [p.x.t + listW + 28 + 110, p.y.t + BAR_H + 116 + 4 + 32]
  }
  clickAt(attached - 0.35, "postbox", chip, () => w.set("postbox", { data: { open: { ...R("postbox").data.open, attachSel: 1 } } }))
  const photoAt = () => {
    const ww = Math.min(S.SW * 0.78, (S.SH - 160) * 1.495 + 0)
    const hh = ww / 1.495 + BAR_H
    return [S.SW / 2 - ww / 2, (S.SH - 60) / 2 - hh / 2 + 10, ww, hh]
  }
  at(attached + 0.05, () => {
    w.open({ id: "photo", app: "viewer", kind: "photo", title: "IMG_4471.jpg", float: true, at: photoAt(), data: { plate: "across" } })
    S.sfx("thunk", 0.3)
  })
  push(attached + 0.1, attached + 1.0, () => frameOf("photo", 1.06), "photo")
  // "It took me a minute to recognise the study." The camera leans in.
  const took = word("It took me")
  const fromSide = word("his side")
  push(took, fromSide + 0.6, () => {
    const p = R("photo")
    const D = Math.max(p.w.t / S.SW, p.h.t / S.SH) * 0.62
    return { cx: p.x.t + p.w.t * 0.52, cy: p.y.t + BAR_H + (p.h.t - BAR_H) * 0.46, D }
  })

  // ── "Three of you signed the same note. About the cameras. I closed two." ──
  // The same three notes as at the start, and the same two clicks.
  const signed = word("Three of them signed")
  push(signed - 0.6, signed + 0.2, home, null)
  clickAt(signed - 0.3, "photo", closeBtn, () => (w.close("photo"), S.sfx("close", 0.45)), 0.7)
  // one petition, three households: the same two lines, each signed (and added to) in its own hand
  const NOTE14 = (sig, who, extra = []) => ({ heading: "About the cameras", list: ["cameras", "hedge", "minutes", "six"], edited: "Signed " + who, lines: ["We'd like the two cameras on No. 12", "that face our houses taken down.", ...extra, { gap: 1 }, { t: "— " + sig, soft: 1 }] })
  openMany(signed + 0.25, 2, [
    { id: "cam16", app: "jot", kind: "jot", title: "cameras — No. 16", data: NOTE14("Ruth and Alan, No. 16", "Feb 26") },
    { id: "cam10", app: "jot", kind: "jot", title: "cameras — No. 10", data: NOTE14("Mark (10)", "Feb 26", ["Also still waiting on my drill."]) },
    { id: "cam14", app: "jot", kind: "jot", title: "cameras — No. 14", data: NOTE14("The Abeles, No. 14", "Feb 27", ["Thank you."]) },
  ])
  const closed2 = word("I closed two")
  clickAt(word("closed") + 0.02, "cam16", closeBtn, () => (w.close("cam16"), S.sfx("close", 0.7)))
  clickAt(word("two") + 0.05, "cam10", closeBtn, () => (w.close("cam10"), S.sfx("close", 0.7)))
  void closed2

  // ── Six ──
  // Six is kept empty: its cell in the bar never shows what's moved there. A neighbour's camera goes
  // quiet (the empty house), and its window is moved from five to six.
  at(0, () => w.veiled.add(6))
  const toSix = (t, id, clockAt, date, secs, src) => {
    sw(t - 0.1, 5, clockAt)
    at(t - 0.05, () => feedsAt(date, secs))
    at(t + 0.25, () => w.set(id, { data: { src, lost: [[t + 0.25, t + 0.85]] } }))
    clickAt(t + 0.95, id, titleOf, () => w.setFocus(id), 0.6)
    key(t + 1.35, ["⌥", "⇧", "6"], () => w.move(id, 6))
    // once it has landed, six holds nothing but what the film ends on
    at(t + 1.9, () => w.close(id))
  }
  const sixteen = word("Sixteen was in March")
  toSix(sixteen, "f16", "Tue Mar 3  11:52 PM", "03-03-2026", 23 * 3600 + 52 * 60 + 9, "st-16-empty")
  const june = word("Mark was in June")
  toSix(june, "f10", "Thu Jun 11  11:58 PM", "06-11-2026", 23 * 3600 + 58 * 60 + 31, "st-10-empty")
  // "Six is downstairs. They're both on six." The bar, and six's cell, empty.
  const both = word("Six is downstairs")
  at(both - 0.2, () => {
    w.clock = "Fri Nov 20  11:40 PM"
    feedsAt("11-20-2026", 23 * 3600 + 40 * 60)
    w.showBar()
  })
  push(both - 0.1, both + 1.1, () => {
    const g = w.barGeom()
    const D = Math.min(1, Math.max(0.42, ((g.w / g.sc) * 1.15) / S.SW))
    return { cx: S.SW / 2, cy: S.SH - (S.SH * D) / 2, D }
  }, null)
  at(both + 0.3, () => {
    const c = w.cellRect(6)
    w.cursorTo(c.x + c.w / 2, c.y + c.h / 2, word("both on six"))
  })

  // ── "Everything else had a place. He was the last thing." No. 14, live ──
  const place = word("Everything else had")
  push(place - 0.4, place + 0.6, home, null)
  at(place - 0.3, () => w.cursorTo(S.SW * 0.62, S.SH * 0.42, place + 0.4))
  const lastThing = word("last thing")
  clickAt(lastThing - 0.4, "f14", titleOf, () => w.setFocus("f14"), 0.8)
  const lastEnd = TAPE.marks.gaps.find((g) => g[0] > lastThing && g[0] < lastThing + 2)[0]
  push(lastThing - 0.3, lastEnd + 0.4, () => frameOf("f14", 1.12), "f14")
  // "I went over just before midnight. His light was on." In on his window.
  const went = word("I went over")
  at(went, () => {
    w.clock = "Fri Nov 20  11:56 PM"
    feedsAt("11-20-2026", 23 * 3600 + 56 * 60 + 5)
  })
  const light = word("His light")
  // in on his window, but never past its edges: it stays a camera on a screen
  push(light - 0.1, word("He took") + 0.2, () => frameOf("f14", 1.03))
  // "He took a while to come down." The light goes off.
  at(word("come down") + 0.1, () => w.set("f14", { data: { unlit: [[0.465, 0.415, 0.5, 0.472]] } }))
  // what happens downstairs happens on the tape; his camera loses the house for a while
  at(ACT[0] + 0.2, () => w.set("f14", { data: { lost: [[ACT[0] + 0.2, ACT[0] + ACT[1] - 0.3]] } }))
  push(ACT[0], ACT[0] + 1.2, () => frameOf("f14", 1.12))
  // "He's on six now. With the others." Moved; then six, for the first time: one window.
  const now = word("on six now")
  key(now + 0.3, ["⌥", "⇧", "6"], () => w.move("f14", 6))
  push(now - 0.2, now + 0.4, home, null)
  at(now + 0.9, () => w.close("f14"))
  const others = word("With the others")
  at(others - 0.6, () => {
    w.veiled.delete(6)
    w.add({ ws: 6, id: "cellar", app: "vigil", kind: "feed", title: "Cellar", dark: 1, data: { src: "cam-cellar", name: "CELLAR", us: 1, ir: 1, q: 0.3, seed: 37, date: "11-21-2026", clock: 4 * 60 + 12 - w.time } })
  })
  sw(others - 0.4, 6)
  at(END, () => S.end())

  LOOKS = looks({ intro, install, tiled, hold0: HOLD[0], hold1: HOLD[1], house, hedge, again, six: sixteen, place })

  cues.sort((a, b) => a.t - b.t)
  return { cues, spans }
}

function dayClock(i) {
  const months = [["Jun", 30], ["Jul", 31], ["Aug", 31], ["Sep", 30], ["Oct", 31], ["Nov", 30]]
  const wd = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"] // Jun 11 was a Thursday
  let d = i + 11
  for (const [m, n] of months) {
    if (d <= n) return `${wd[i % 7]} ${m} ${d}  6:${String(2 + (i % 3)).padStart(2, "0")} AM`
    d -= n
  }
  return "Fri Nov 20  6:02 AM"
}
