// The film, as cues on a clock (seconds after the key press, which is tape time). Cues fire once,
// in order, as the clock passes them; spans run every frame between two times. Picture beats are
// pinned to the words of the tape (tape.js), so the cut follows the voice exactly. Everything here
// only calls the world's verbs, so the same timeline drives WebGPU, Canvas 2D, capture and reduced
// motion.
import { easeInOut, smooth, clamp, mix } from "./springs.js"
import { BAR_H } from "./content.js"
import { WORDS } from "./tape.js"

export const HOLD = [47.6, 54.0] // the visitor's turn: Option+1/2/3 really switch
export const END = 113.0

const norm = (s) => s.toLowerCase().replace(/[^a-z0-9']/g, "")
// When a phrase starts on the tape (the first match after a time).
export function word(phrase, after = 0) {
  const ws = phrase.split(/\s+/).map(norm)
  for (let i = 0; i < WORDS.length; i++) {
    if (WORDS[i][1] < after) continue
    if (ws.every((x, j) => WORDS[i + j] && norm(WORDS[i + j][0]) === x)) return WORDS[i][1]
  }
  return after
}

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
// the months cooling; a cold November night; the dark study; the kitchen's lamp.
const LOOKS = [
  [0, { exposure: 0.93, warmth: 0.35, saturation: 0.92, contrast: 1.02, vignette: 0.3, bloom: 0.16, g: [0, 0, 0] }],
  [19.6, { exposure: 0.93, warmth: 0.35, saturation: 0.92, contrast: 1.02, vignette: 0.3, bloom: 0.16, g: [0, 0, 0] }],
  [21.6, { exposure: 1.0, warmth: 0.12, saturation: 1.0, contrast: 1.03, vignette: 0.22, bloom: 0.16, g: [0, 0, 0] }],
  [47, { exposure: 1.0, warmth: 0.12, saturation: 1.0, contrast: 1.03, vignette: 0.22, bloom: 0.16, g: [0, 0, 0] }],
  [54, { exposure: 1.04, warmth: 0.04, saturation: 1.02, contrast: 1.03, vignette: 0.2, bloom: 0.18, g: [0, 0, 0] }],
  [70.6, { exposure: 1.04, warmth: 0.04, saturation: 1.02, contrast: 1.03, vignette: 0.2, bloom: 0.18, g: [0, 0, 0] }],
  [71.6, { exposure: 0.97, warmth: 0.26, saturation: 0.96, contrast: 1.03, vignette: 0.26, bloom: 0.16, g: [0, 0, 0] }],
  [76.6, { exposure: 0.97, warmth: 0.26, saturation: 0.96, contrast: 1.03, vignette: 0.26, bloom: 0.16, g: [0, 0, 0] }],
  [78, { exposure: 1.02, warmth: 0.1, saturation: 0.98, contrast: 1.03, vignette: 0.24, bloom: 0.16, g: [0, 0, 0] }],
  [88, { exposure: 0.95, warmth: -0.2, saturation: 0.82, contrast: 1.05, vignette: 0.28, bloom: 0.16, g: [0, 0.2, 0.05] }],
  [91.4, { exposure: 0.9, warmth: -0.3, saturation: 0.82, contrast: 1.05, vignette: 0.3, bloom: 0.17, g: [0.05, 0.35, 0.08] }],
  [101.6, { exposure: 0.86, warmth: -0.45, saturation: 0.78, contrast: 1.06, vignette: 0.36, bloom: 0.2, g: [0.12, 0.45, 0.1] }],
  [104.4, { exposure: 0.96, warmth: -0.2, saturation: 0.9, contrast: 1.04, vignette: 0.42, bloom: 0.24, g: [0.12, 0.45, 0.1] }],
  [108.1, { exposure: 1.0, warmth: 0.35, saturation: 0.95, contrast: 1.06, vignette: 0.46, bloom: 0.3, g: [0.12, 0.45, 0.1] }],
  [200, { exposure: 1.0, warmth: 0.35, saturation: 0.95, contrast: 1.06, vignette: 0.46, bloom: 0.3, g: [0.12, 0.45, 0.1] }],
]

function applyLook(S, t) {
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

  span(0, END, (u, t) => applyLook(S, t))
  const BAR_BASE = 1.5
  const BAR_BIG = 2.2

  // ── 0 · The pile, before anything has a place ──
  at(0, () => {
    S.bed("room", 0.16, 1.5)
    w.bar.scale.x = w.bar.scale.t = 1.5
  })
  push(0.3, 10.4, { cx: S.SW * 0.52, cy: S.SH * 0.5, D: 0.93 })
  at(2.2, () => w.cursorTo(S.SW * 0.56, S.SH * 0.6, 3.2))
  at(6.4, () => w.cursorTo(S.SW * 0.5, S.SH * 0.56, 7.6))
  // "...open since March": the March screenshot of this same mess
  const march = word("I'd had some", 10)
  clickAt(march + 0.5, "shot", titleOf, () => w.setFocus("shot"))
  push(march, march + 1.4, () => frameOf("shot", 1.45), "shot")
  const feb = word("February", 13)
  clickAt(feb + 0.25, "untitled", titleOf, () => w.setFocus("untitled"))
  push(feb + 0.1, feb + 1.3, () => frameOf("untitled", 1.5), "untitled")
  // "Three of them were, uh... the same note": each raised in turn
  const three = word("Three of them", 15)
  push(three - 0.1, three + 1.0, { cx: S.SW * 0.5, cy: S.SH * 0.5, D: 0.95 }, null)
  clickAt(three + 0.35, "notes", titleOf, () => w.setFocus("notes"))
  clickAt(three + 1.05, "notes1", titleOf, () => w.setFocus("notes1"))
  clickAt(three + 1.75, "notes2", titleOf, () => w.setFocus("notes2"))
  // "I closed two."
  clickAt(word("closed", 18) + 0.02, "notes1", closeBtn, () => (w.close("notes1"), S.sfx("close", 0.7)))
  clickAt(word("two", 18.8) + 0.05, "notes2", closeBtn, () => (w.close("notes2"), S.sfx("close", 0.7)))

  // ── Spaceman starts: everything side by side ──
  const every = word("everything went", 20)
  push(19.3, 20.3, home)
  at(every - 0.55, () => {
    w.status = true
    S.sfx("key2", 0.45)
  })
  at(every, () => {
    w.tileNow()
    S.sfx("thunk", 0.8)
  })
  at(every + 0.7, () => {
    w.ensureCell(3)
    w.showBar()
  })
  at(every + 0.2, () => w.cursorTo(S.SW * 0.6, S.SH * 0.7, every + 1.2))
  push(22.4, 28.8, { cx: S.SW * 0.5, cy: S.SH * 0.47, D: 0.94 })
  push(29.4, 30.2, home)

  // ── Moving, then switching ──
  const mail2 = word("Mail went to two", 30)
  clickAt(mail2 - 0.6, "postbox", titleOf, () => w.setFocus("postbox"))
  key(mail2 + 0.25, ["⌥", "⇧", "2"], () => w.move("postbox", 2))
  const brow = word("browser went", 32)
  clickAt(brow - 0.45, "browsy", titleOf, () => w.setFocus("browsy"))
  key(brow + 0.15, ["⌥", "⇧", "3"], () => w.move("browsy", 3))
  sw(word("back to make", 34), 2)
  sw(word("they were just", 35) + 0.25, 3)
  sw(word("Exactly where", 37) + 0.5, 1)

  // ── The bar ──
  const bar = word("bar along", 41)
  at(bar - 0.2, () => (w.bar.scale.t = BAR_BIG))
  push(bar - 0.1, bar + 1.2, () => {
    const D = 0.56
    return { cx: S.SW / 2, cy: S.SH - (S.SH * D) / 2, D }
  })
  const away = word("one key away", 45)
  sw(away - 0.45, 2)
  sw(away + 0.05, 3)
  sw(away + 0.6, 1)
  push(46.7, 47.6, home)

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
  const rule0 = word("Then I made a rule", 54)
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
  const chatter = word("Chatter opens", 55)
  type(chatter - 0.1, "Chatter", (s, done) => w.set("settings", { data: { rules: [{ app: s, icon: done ? "chatter" : null, action: "", ws: null, cursor: 0 }] } }))
  clickAt(word("on two", 56) - 0.05, "settings", () => cell(3), () => w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Open on", ws: null, cursor: 3 }] } }), 0.6)
  at(word("two", 56.3) + 0.12, () => {
    w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Open on", ws: 2, hl: 1 }] } })
    S.sfx("key", 0.3)
  })
  push(57.3, 58.2, home, null)
  clickAt(57.75, "settings", closeBtn, () => (w.close("settings"), S.sfx("close", 0.5)))
  // a new Chatter window: it opens, and goes straight to 2
  const goes = word("It just goes", 59)
  at(goes - 0.2, () => {
    const ww = S.SW * 0.3
    const hh = S.SH * 0.38
    w.open({ id: "dana", app: "chatter", kind: "chat", title: "Dana", float: true, at: [S.SW * 0.62 - ww / 2, S.SH * 0.42 - hh / 2, ww, hh], data: { who: "Dana", when: "Today 6:12 AM", msgs: [{ t: "are we still on for friday?" }, { t: "you can say no", note: "Read 6:14 AM" }] } })
    S.sfx("thunk", 0.35)
  })
  at(goes + 0.35, () => w.move("dana", 2))
  // "I did read both messages."
  const read = word("did read both", 61)
  sw(read - 0.05, 2)
  push(read + 0.15, read + 1.0, () => frameOf("dana", 1.3), "dana")
  // "I gave Mark his drill back."
  const gave = word("gave Mark", 63)
  sw(gave - 0.1, 1, "Sat Jun 6  10:05 AM")
  push(gave - 0.05, gave + 0.85, () => frameOf("drill", 1.5), "drill")
  clickAt(word("back", 63.9), "drill", (id) => [R(id).x.t + 34, R(id).y.t + BAR_H + 72], () => w.set("drill", { data: { check: 1, lines: [{ t: "to No. 10 (Mark)", done: 1 }, { t: "borrowed spring 2022", soft: 1, nobox: 1 }] } }))
  push(64.4, 68.4, () => ({ ...frameOf("drill", 1.36) }))
  push(68.9, 69.8, home, null)
  // "...the place was his.": the note goes, and the others close the gap
  clickAt(word("his", 70.2), "drill", closeBtn, () => (w.close("drill"), S.sfx("close", 0.6)))

  // ── 4 · The house ──
  const house = word("Four is the house", 71)
  sw(house, 4, "Tue Jun 9  9:30 PM")
  open(house + 0.25, 4, { id: "groceries", app: "sheets", kind: "sheet", title: "Groceries — Standing Order", data: { cols: ["Item", "Qty", "Day"], widths: [0.5, 0.25, 0.25], rows: [["Eggs", "12", "Sun"], ["Milk", "1 gal", "Sun"], ["Bread", "1", "Sun"], ["Coffee", "2 lb", "Sun"], ["Apples", "6", "Sun"], ["Paper towels", "2", "1st Sun"]].map((cells) => ({ cells })) } })
  open(house + 0.5, 4, { id: "laundry", app: "jot", kind: "jot", title: "Laundry — Rotation", data: { heading: "Laundry — Rotation", lines: ["Mon  whites", "Wed  darks", "Fri  towels", "Sun  sheets"] } })
  open(house + 0.75, 4, { id: "friday", app: "daybook", kind: "daybook", title: "Daybook — Friday", data: { date: "Friday, June 12", events: [{ t: "Dinner (4)", at: 18.5, len: 2.5, color: "#e9d5c4" }] } })

  // ── 5 · The street ──
  const street = word("Five is the street", 72)
  sw(street + 0.05, 5)
  open(street + 0.3, 5, { id: "minutes", app: "paper", kind: "paper", title: "HOA — Minutes", data: { heading: "Pinecrest HOA — Minutes", sub: "Meeting of June 9 · taken by No. 12", size: 14, gap: 23, lines: ["Present: 9 of 9", "1. Trash cans out of view by 8 PM", "    on collection days.", "2. Hedges and fences: six feet maximum.", "3. Minutes: No. 12."] } })
  open(street + 0.55, 5, { id: "trash", app: "sheets", kind: "sheet", title: "Trash Cans — Collection", data: { cols: ["House", "Out", "In by 8"], widths: [0.4, 0.3, 0.3], rows: [["No. 8", "✓", "✓"], ["No. 10", "✓", "8:40 PM"], ["No. 12", "✓", "✓"], ["No. 14", "✓", "—"], ["No. 16", "✓", "✓"]].map((cells) => ({ cells })) } })
  open(street + 0.8, 5, { id: "door", app: "doorbell", kind: "cam", title: "Doorbell — Front", dark: 1, data: { time: "9:31:02 PM", lit: 0 } })
  const mins = word("Nobody else wanted", 74)
  at(mins, () => w.setFocus("minutes"))
  push(mins + 0.1, mins + 1.1, () => frameOf("minutes", 1.2), "minutes")
  push(76.0, 76.85, home, null)

  // ── The hedge ──
  const hedge = word("Your hedge", 76)
  open(hedge, 5, { id: "hedge", app: "viewer", kind: "photos", title: "No. 14 — Hedge", data: { count: 1 } })
  at(hedge, () => (w.clock = dayClock(0)))
  push(hedge + 0.2, hedge + 1.1, () => frameOf("hedge", 1.04), "hedge")
  at(word("six foot two", 77) + 0.1, () => (w.set("hedge", { data: { measured: 1 } }), S.sfx("shutter", 0.35)))
  at(word("The limit", 78.5) + 0.2, () => w.set("hedge", { data: { limit: 1 } }))
  // "I've photographed it every morning since." A hundred and sixty mornings.
  const since = word("photographed", 83)
  let shots = 1
  span(since, since + 4.4, (u) => {
    const n = 1 + Math.floor(159 * u * u)
    if (n !== shots) {
      if (n <= 4 || n === 24 || n === 70) S.sfx("shutter", n <= 4 ? 0.35 : 0.25)
      shots = n
      w.set("hedge", { data: { count: n, measured: n <= 3 ? 1 : 0 } })
      w.clock = dayClock(n - 1)
    }
  })
  // "...find any one of them in a second."
  const find = word("find any one", 87)
  type(find + 0.1, "Aug 14", (s) => w.set("hedge", { data: { query: s, typing: 1 } }), 3)
  at(word("Any morning", 89), () => {
    w.set("hedge", { data: { pick: 64, typing: 0 } })
    S.sfx("key2", 0.25)
  })

  // ── The minutes, again ──
  const again = word("sent the minutes round", 90.5)
  push(again - 0.5, again + 0.4, home, null)
  at(again - 0.1, () => {
    w.clock = "Wed Nov 11  9:12 PM"
    w.setFocus("minutes")
    w.set("minutes", { title: "HOA — Minutes (awaiting corrections)", data: { banner: ["Sent to 8 households · Jun 10"] } })
  })
  at(again + 0.75, () => (w.set("minutes", { data: { banner: ["Sent to 8 households · Jun 10", "Sent again to 8 households · Nov 11"] } }), S.sfx("thunk", 0.2)))

  // ── The only reply ──
  const only = word("You're the only", 92.5)
  sw(only - 0.1, 2, "Fri Nov 20  7:41 PM")
  const reply = { from: "No. 14", subject: "Re: Minutes — corrections", when: "7:41 PM", unread: 1, clip: 1 }
  at(only + 0.25, () => {
    const d = R("postbox").data
    w.set("postbox", { data: { list: [reply, ...d.list], sel: -1, slide: 0 } })
    S.sfx("thunk", 0.25)
  })
  span(only + 0.25, only + 0.6, (u) => w.set("postbox", { data: { slide: smooth(u) } }))
  push(only + 0.1, only + 1.0, () => frameOf("postbox", 1.12), "postbox")
  const attached = word("photograph attached", 94.5)
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
  const took = word("It took me", 96.5)
  push(took, word("this side", 99.5) + 0.5, () => {
    const p = R("photo")
    const D = Math.max(p.w.t / S.SW, p.h.t / S.SH) * 0.62
    return { cx: p.x.t + p.w.t * 0.52, cy: p.y.t + BAR_H + (p.h.t - BAR_H) * 0.46, D }
  })

  // ── The last thing ──
  const place = word("Everything else has", 100.5)
  push(place - 0.5, place + 0.3, home, null)
  clickAt(place + 0.15, "photo", closeBtn, () => (w.close("photo"), S.sfx("close", 0.45)))
  sw(place + 0.55, 5, "Fri Nov 20  11:47 PM")
  at(place + 0.6, () => {
    w.setFocus("door")
    w.set("door", { data: { time: "11:47:02 PM", lit: 0 } })
  })
  for (let s = 1; s < 6; s++) at(place + 0.6 + s, () => w.set("door", { data: { time: `11:47:0${2 + s} PM` } }))
  push(place + 0.9, place + 2.1, () => frameOf("door", 1.02), "door")
  // "You're the last thing." His light comes on across the street.
  at(word("last thing", 103) + 0.05, () => w.set("door", { data: { lit: 1 } }))
  at(103.2, () => w.cursorTo(S.SW * 0.86, S.SH * 0.9, 104.6))

  // ── The study, empty; across the street; the kitchen ──
  span(104.3, 106.4, (u) => (S.scene.pull = u))
  at(104.3, () => {
    S.bed("room", 0.3, 0.8)
    S.bed("night", 0.22, 1.2)
  })
  span(106.6, 108.4, (u) => (S.scene.win = u))
  at(106.8, () => S.sfx("car", 0.35))
  span(108.0, 108.6, (u) => (S.scene.fade = smooth(u)))
  at(108.0, () => (S.bed("room", 0, 0.25), S.bed("night", 0, 0.25)))
  span(108.0, END, (u) => (S.scene.kpush = u))
  at(END, () => S.end())

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
