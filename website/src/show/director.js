// The film, as cues on a clock (seconds after the key press). Cues fire once, in order, as the clock
// passes them; spans run every frame between two times. Everything here only calls the world's
// verbs, so the same timeline drives WebGPU, Canvas 2D, capture and reduced motion.
import { easeInOut, smooth, clamp, mix } from "./springs.js"
import { BAR_H } from "./content.js"

export const HOLD = [24, 30] // the visitor's turn: Option+1/2/3 really switch
export const END = 100.66

const NOTE = ["call about the gutters", "drill → return", "passwords??", "organize notes", "hedge (ask)"]

export function pile(SW, SH) {
  const W = (f) => f * SW
  const H = (f) => f * SH
  const tall = SH > SW
  const F = (x, y, w, h) => (tall ? [W(y * 0.9), H(0.06 + x * 0.8), W(Math.min(0.92, h * 1.5)), H(w * 0.62)] : [W(x), H(y), W(w), H(h)])
  return [
    { id: "browsy", app: "browsy", kind: "browser", title: "Browsy — 38 tabs", at: F(0.38, 0.08, 0.5, 0.62), data: { tabs: 38, tabTitle: "Gutter guards", url: "gutter guards which kind", heading: "Gutter guards: which kind?" } },
    { id: "postbox", app: "postbox", kind: "mail", title: "Postbox — 4,812 unread", at: F(0.04, 0.1, 0.48, 0.5), data: mail() },
    { id: "shot", app: "viewer", kind: "shot", title: "Screenshot 2026-03-02 at 11.48.07 PM", at: F(0.5, 0.42, 0.36, 0.4) },
    { id: "recipes", app: "paper", kind: "paper", title: "Recipes", at: F(0.62, 0.16, 0.3, 0.44), data: { heading: "Recipes", lines: ["Chili (Mom's)", "Banana bread", "Lasagna — the good one", "Pancakes"] } },
    { id: "daybook", app: "daybook", kind: "daybook", title: "Daybook", at: F(0.12, 0.36, 0.32, 0.5), data: { date: "Tuesday, June 2", events: [{ t: "Trash out", at: 19, len: 0.7, color: "#d9e3cf" }] } },
    { id: "untitled", app: "paper", kind: "paper", title: "Untitled 47", at: F(0.3, 0.3, 0.3, 0.36), data: {} },
    { id: "drill", app: "jot", kind: "jot", title: "return drill", at: F(0.68, 0.5, 0.27, 0.34), data: { heading: "return drill", check: 1, lines: ["to No. 10", { t: "borrowed spring 2022", soft: 1, nobox: 1 }] } },
    { id: "notes2", app: "jot", kind: "jot", title: "notes — real", at: F(0.2, 0.2, 0.3, 0.42), data: { heading: "notes — real", lines: NOTE } },
    { id: "notes1", app: "jot", kind: "jot", title: "notes (1)", at: F(0.1, 0.46, 0.3, 0.42), data: { heading: "notes", lines: NOTE } },
    { id: "notes", app: "jot", kind: "jot", title: "notes", at: F(0.44, 0.24, 0.3, 0.42), data: { heading: "notes", lines: NOTE } },
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
  return { list, sel: 0, open: { subject: "Reminder: hedge and fence heights", from: "Pinecrest HOA", body: ["Hedges and fences along the front", "may not exceed six feet.", "", "Thank you for keeping Pinecrest", "looking its best."] } }
}

const MINUTES_ROWS = [
  ["Jun 9", "0:45", "9 of 9"],
  ["Jul 14", "0:40", "7 of 9"],
  ["Aug 11", "0:35", "5 of 9"],
  ["Sep 8", "0:20", "3 of 9"],
  ["Oct 13", "0:12", "2 of 9"],
  ["Nov 10", "0:04", "1 of 9"],
]

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
    return [x.x.t + x.w.t * 0.5, x.y.t + BAR_H / 2]
  }
  const clickAt = (t, id, where = titleOf) => {
    at(t - 0.55, () => w.moveCursor(...where(id)))
    at(t, () => w.click())
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
  const frameOf = (id, margin = 1.1) => {
    const x = R(id)
    const D = Math.min(1, Math.max(x.w.t / S.SW, x.h.t / S.SH) * margin)
    // keep the view inside the screen: never show what's past its edge
    const hx = (S.SW * D) / 2
    const hy = (S.SH * D) / 2
    return { cx: clamp(x.x.t + x.w.t / 2, hx, S.SW - hx), cy: clamp(x.y.t + x.h.t / 2, hy, S.SH - hy), D }
  }
  // Camera move from wherever it is to a framing, over [t0, t1].
  const push = (t0, t1, target) => {
    let from = null
    let to = null
    span(t0, t1, (u) => {
      if (!from) {
        from = { ...S.cam }
        to = typeof target === "function" ? target() : target
      }
      const e = easeInOut(u)
      S.cam.cx = mix(from.cx, to.cx, e)
      S.cam.cy = mix(from.cy, to.cy, e)
      S.cam.D = Math.exp(mix(Math.log(from.D), Math.log(to.D), e))
    })
  }
  const home = () => ({ cx: S.SW / 2, cy: S.SH / 2, D: 1 })

  const BAR_BASE = 1.5
  const BAR_BIG = 2.2

  // ── 0 · The key press: Spaceman starts, then the windows on 1 tile ──
  at(0, () => {
    w.status = true
    w.bar.scale.x = w.bar.scale.t = BAR_BASE
    w.ensureCell(3)
    w.showBar()
    S.sfx("key", 0.5)
    S.room(0.5)
  })
  at(0.7, () => {
    w.tileNow()
    S.sfx("thunk", 0.8)
  })
  S.say(2.4, "march")
  clickAt(6.3, "notes1", closeBtn)
  at(6.3, () => (w.close("notes1"), S.sfx("close", 0.7)))
  clickAt(7.4, "notes2", closeBtn)
  at(7.4, () => (w.close("notes2"), S.sfx("close", 0.7)))
  clickAt(8.5, "untitled", closeBtn)
  at(8.5, () => (w.close("untitled"), S.sfx("close", 0.7)))
  S.say(8.9, "same-note")
  clickAt(12.2, "sticky", (id) => [R(id).x.t + R(id).w.t * 0.5, R(id).y.t + BAR_H + 30])
  at(12.2, () => (w.set("sticky", { data: { done: 1 } }), w.setFocus("sticky")))

  // ── Moving, then switching: one whole example first ──
  at(13.0, () => to2(w.bar.scale, BAR_BIG))
  clickAt(13.4, "postbox")
  at(13.4, () => w.setFocus("postbox"))
  key(13.9, ["⌥", "⇧", "2"], () => w.move("postbox", 2))
  S.say(14.3, "mail")
  sw(15.9, 2)
  sw(17.0, 1)
  clickAt(18.0, "browsy")
  at(18.0, () => w.setFocus("browsy"))
  key(18.6, ["⌥", "⇧", "3"], () => w.move("browsy", 3))
  S.say(19.0, "internet")
  sw(20.4, 3)
  sw(21.8, 1)

  // ── The visitor's turn ──
  at(HOLD[0], () => S.hint(true))
  at(HOLD[1] - 0.4, () => {
    S.hint(false)
    if (w.active !== 1) w.switchTo(1)
    to2(w.bar.scale, BAR_BASE)
  })

  // ── A rule: Chatter opens on 2 ──
  const settingsAt = () => {
    const sw = Math.min(S.SW * 0.86, 860)
    const sh = Math.min(S.SH * 0.6, 380)
    return [S.SW / 2 - sw / 2, S.SH * 0.44 - sh / 2, sw, sh]
  }
  const settingsFrame = () => {
    const [x, y, ww, hh] = settingsAt()
    return { cx: x + ww / 2, cy: y + hh / 2, D: Math.max(ww / S.SW, hh / S.SH) * 1.14 }
  }
  at(30.2, () => {
    w.add({ id: "settings", app: "spaceman", kind: "settings", title: "Spaceman Settings", float: true, hidden: true, at: settingsAt(), data: { rules: [], add: 1 } })
    const s = R("settings")
    s.s.x = 0.96
    S.world.setFocus("settings")
    s.a.t = 1
    s.s.t = 1
    w.moveCursor(settingsAt()[0] + 200, settingsAt()[1] + 132)
  })
  push(30.3, 31.2, settingsFrame)
  S.say(30.6, "rule")
  clickAt(31.3, "settings", () => [settingsAt()[0] + 200, settingsAt()[1] + 132])
  at(31.3, () => w.set("settings", { data: { add: 0, rules: [{ app: "Chatter", icon: "chatter", action: "", ws: null, cursor: 0 }] } }))
  at(31.6, () => w.moveCursor(settingsAt()[0] + 172 + (settingsAt()[2] - 192) * 0.62, settingsAt()[1] + 132))
  at(32.2, () => (w.click(), w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Route", ws: null }] } })))
  at(32.5, () => w.moveCursor(settingsAt()[0] + 172 + (settingsAt()[2] - 192) * 0.82, settingsAt()[1] + 132))
  at(33.1, () => (w.click(), w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Route", ws: 2, hl: 1 }] } })))
  push(34.3, 35.0, home)
  at(34.7, () => w.close("settings"))
  at(35.2, () => {
    const [x, y, ww, hh] = [S.SW * 0.55, S.SH * 0.3, S.SW * 0.3, S.SH * 0.4]
    w.open({ id: "dana", app: "chatter", kind: "chat", title: "Dana", ws: 1, at: [x, y, ww, hh], data: { msgs: [{ t: "are we still on for friday?" }, { t: "you can say no", note: "Delivered" }] } })
    S.sfx("thunk", 0.4)
  })
  at(35.5, () => w.move("dana", 2))
  S.say(36.8, "not-closed")
  sw(37.6, 2)
  // stay with her message, Postbox's edge still in frame
  push(37.8, 38.6, () => {
    const f = frameOf("dana", 1.25)
    return { ...f, cx: clamp(f.cx - S.SW * 0.06, (S.SW * f.D) / 2, S.SW - (S.SW * f.D) / 2) }
  })
  sw(40.1, 1, "Wed Jun 3  6:02 AM")

  // ── The drill: he ticks it after the neighbour says he needn't have ──
  at(40.15, () => w.setFocus("drill"))
  push(40.15, 40.9, () => frameOf("drill", 1.5))
  S.say(40.8, "drill")
  clickAt(46.3, "drill", (id) => [R(id).x.t + 34, R(id).y.t + BAR_H + 64])
  at(46.3, () => w.set("drill", { data: { check: 1, lines: [{ t: "to No. 10", done: 1 }, { t: "borrowed spring 2022", soft: 1, nobox: 1 }] } }))
  push(46.9, 47.6, home)

  // ── 4 · The house ──
  sw(47.6, 4, "Sat Jun 6  9:30 AM")
  const open = (t, ws, def) => at(t, () => (w.open({ ws, ...def, at: def.at || [S.SW * 0.3, S.SH * 0.3, S.SW * 0.4, S.SH * 0.4] }), S.sfx("thunk", 0.35)))
  open(47.9, 4, { id: "groceries", app: "sheets", kind: "sheet", title: "Groceries — Standing Order", data: { cols: ["Item", "Qty", "Day"], widths: [0.5, 0.25, 0.25], rows: [["Eggs", "12", "Sun"], ["Milk", "1 gal", "Sun"], ["Bread", "1", "Sun"], ["Coffee", "2 lb", "Sun"], ["Apples", "6", "Sun"], ["Paper towels", "2", "1st Sun"]].map((cells) => ({ cells })) } })
  open(48.2, 4, { id: "laundry", app: "jot", kind: "jot", title: "Laundry — Rotation", data: { heading: "Laundry — Rotation", lines: ["Mon  whites", "Wed  darks", "Fri  towels", "Sun  sheets"] } })
  open(48.5, 4, { id: "friday", app: "daybook", kind: "daybook", title: "Friday dinner", data: { date: "Friday, June 12", events: [{ t: "Dinner (4)", at: 18.5, len: 2.5, color: "#e9d5c4" }] } })
  S.say(48.1, "house")

  // ── 5 · The neighborhood ──
  sw(50.5, 5, "Tue Jun 9  10:12 PM")
  open(50.8, 5, { id: "minutes", app: "paper", kind: "paper", title: "HOA — Minutes", data: { heading: "Pinecrest HOA — Minutes", sub: "Meeting of June 9", size: 14, gap: 23, lines: ["Present: 9 of 9", "1. Trash cans to be stored out of view", "    by 8 PM on collection days.", "2. Hedges and fences: six feet maximum.", "3. Minutes: No. 12."] } })
  open(51.2, 5, { id: "trash", app: "sheets", kind: "sheet", title: "Trash Cans — Collection", data: { cols: ["House", "Out", "In by 8"], widths: [0.4, 0.3, 0.3], rows: [["No. 8", "✓", "✓"], ["No. 10", "✓", "8:40 PM"], ["No. 12", "✓", "✓"], ["No. 14", "✓", "—"], ["No. 16", "✓", "✓"]].map((cells) => ({ cells })) } })
  open(51.6, 5, { id: "door", app: "doorbell", kind: "cam", title: "Doorbell — Front", dark: 1, data: { time: "10:12:31 PM", lit: 0 } })
  S.say(51.4, "minutes")

  // ── The hedge: three mornings you can read, then all of them ──
  open(56.0, 5, { id: "hedge", app: "viewer", kind: "photos", title: "No. 14 — Hedge", data: { count: 1 } })
  let shots = 1
  span(56.0, 65.4, (u, t) => {
    const n = t < 58.2 ? 1 : t < 59.6 ? 2 : t < 60.6 ? 3 : 3 + Math.floor(157 * Math.pow(clamp((t - 60.6) / 3.8), 2))
    if (n !== shots) {
      if (n <= 3 || n === 24 || n === 70) S.sfx("shutter", n <= 3 ? 0.45 : 0.3)
      shots = n
      w.set("hedge", { data: { count: n } })
      w.clock = dayClock(Math.min(n, 140))
    }
  })
  push(56.2, 57.4, () => frameOf("hedge", 1.04))
  S.say(56.6, "hedge")
  S.say(62.6, "second")
  at(63.6, () => w.set("hedge", { data: { measured: 1, measuredFrom: shots } }))
  push(65.6, 66.3, home)

  // ── The minutes, again ──
  at(66.0, () => {
    w.set("minutes", { title: "HOA — Minutes (awaiting corrections)" })
    const m = R("minutes")
    m.kind = "sheet"
    m.app = "sheets"
    w.set("minutes", { data: { cols: ["Meeting", "Length", "Present"], widths: [0.4, 0.28, 0.32], rows: [] } })
    w.setFocus("minutes")
  })
  MINUTES_ROWS.forEach((r, i) => at(66.4 + i * 0.75, () => w.set("minutes", { data: { rows: MINUTES_ROWS.slice(0, i + 1).map((cells) => ({ cells })) } })))
  push(66.2, 68.0, () => frameOf("minutes", 1.25))
  S.say(67.0, "circulated")
  at(67.6, () => w.set("minutes", { data: { foot: ["Sent to 8 households, Jun 10"] } }))
  at(70.4, () => w.set("minutes", { data: { foot: ["Sent to 8 households, Jun 10", "Sent again to 8 households, Nov 11"] } }))
  push(71.6, 72.4, home)

  // ── 6 · The call ──
  sw(72.6, 6, "Tue Nov 17  7:00 PM")
  // the board meeting: him, and No. 16, who leaves
  const people = [{ name: "No. 12" }, { name: "No. 16", initial: "16", off: 1 }]
  people.forEach((p, i) => open(72.9 + i * 0.22, 6, { id: "call" + i, app: "call", kind: "call", title: "HOA — Board", dark: 1, data: { people: [p] } }))
  at(75.3, () => (w.close("call1"), w.set("call0", { data: { toast: "No. 16 left" } }), S.sfx("close", 0.35)))
  at(76.9, () => w.set("call0", { data: { toast: null } }))

  // ── The months ──
  sw(77.6, 1, "Wed Nov 18  6:02 AM")
  at(78.2, () => w.set("friday", { data: { date: "Friday, November 20", events: [{ t: "Dinner (1)", at: 18.5, len: 2.5, color: "#e9d5c4" }] } }))
  sw(78.4, 4, "Wed Nov 18  9:14 PM")
  sw(79.3, 2, "Thu Nov 19  6:02 AM")
  at(80.0, () => {
    w.add({ id: "rules", app: "spaceman", kind: "settings", title: "Spaceman Settings", float: true, hidden: true, at: settingsAt(), data: { rules: [
      { app: "Chatter", icon: "chatter", action: "Route", ws: 2 },
      { app: "Doorbell", icon: "doorbell", action: "Route", ws: 5 },
      { app: "Call", icon: "call", action: "Route", ws: 6 },
      { app: "Viewer", icon: "viewer", title: "No. 14 — Hedge", action: "Route", ws: 5, hl: 1 },
    ] } })
    const s = R("rules")
    s.a.t = 1
    w.setFocus("rules")
  })
  push(80.1, 80.6, settingsFrame)
  push(81.5, 82.0, home)
  at(81.9, () => w.close("rules"))
  sw(82.4, 3, "Thu Nov 19  11:20 PM")
  sw(83.3, 6, "Fri Nov 20  7:41 PM")
  sw(84.3, 5, "Fri Nov 20  11:46 PM")
  span(76.8, 86, (u) => {
    S.grade[0] = 0.26 * smooth(u)
    S.grade[1] = 0.65 * smooth(u)
    S.grade[2] = 0.18 * smooth(u)
  })
  at(77.6, () => S.room(0.28, 3))
  at(84.3, () => w.setFocus("door"))

  // ── Number fourteen ──
  push(84.8, 88.0, () => frameOf("door", 1.04))
  at(85.4, () => (w.clock = "Fri Nov 20  11:48 PM"))
  for (let s = 0; s < 14; s++) at(84.3 + s, () => w.set("door", { data: { time: `11:4${s < 6 ? 7 : 8}:${String((s * 1 + 54) % 60).padStart(2, "0")} PM` } }))
  at(85.8, () => {
    // the cursor drifts onto the feed and stops
    const d = R("door")
    w.moveCursor(d.x.t + d.w.t * 0.62, d.y.t + BAR_H + (d.h.t - BAR_H) * 0.8, 4.6)
  })
  // his light comes on across the street
  at(87.3, () => w.set("door", { data: { lit: 1 } }))
  S.say(88.0, "light")

  // ── The pull-back, then a long look at the other desktop ──
  span(91.8, 95.2, (u) => {
    S.pull.a = u
  })
  at(91.8, () => S.room(0.12, 1.5))
  at(92.4, () => S.sfx("car", 0.55))
  span(95.2, 96.4, (u) => {
    S.pull.b = u
  })
  at(95.2, () => S.room(0, 0.4))
  at(100.55, () => S.outerKey())
  at(END, () => S.end())

  cues.sort((a, b) => a.t - b.t)
  return { cues, spans }
}

function to2(s, v) {
  s.t = v
}

function dayClock(i) {
  const months = [["Jun", 30], ["Jul", 31], ["Aug", 31], ["Sep", 30], ["Oct", 31], ["Nov", 30]]
  const wd = ["Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Mon"]
  let d = i + 3
  for (const [m, n] of months) {
    if (d <= n) return `${wd[(i + 1) % 7]} ${m} ${d}  6:${String(2 + (i % 3)).padStart(2, "0")} AM`
    d -= n
  }
  return "Fri Nov 20  6:02 AM"
}
