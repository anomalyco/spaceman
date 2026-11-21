// The film, as cues on a clock (seconds after the key press). Cues fire once, in order, as the clock
// passes them; spans run every frame between two times. Everything here only calls the world's
// verbs, so the same timeline drives WebGPU, Canvas 2D, capture and reduced motion.
import { easeInOut, smooth, clamp, mix } from "./springs.js"
import { BAR_H } from "./content.js"

export const HOLD = [24, 30] // the visitor's turn: Option+1/2/3 really switch
export const END = 101.4

const NOTE = ["call about the gutters", "drill → return", "passwords??", "organize notes", "hedge (ask)"]

export function pile(SW, SH) {
  const W = (f) => f * SW
  const H = (f) => f * SH
  const tall = SH > SW
  const F = (x, y, w, h) => (tall ? [W(y * 0.9), H(0.06 + x * 0.8), W(Math.min(0.92, h * 1.5)), H(w * 0.62)] : [W(x), H(y), W(w), H(h)])
  return [
    { id: "browsy", app: "browsy", kind: "browser", title: "Browsy — 38 tabs", at: F(0.38, 0.08, 0.5, 0.62), data: { tabs: 38, tabTitle: "How to", url: "how to organize", heading: "How to organize" } },
    { id: "postbox", app: "postbox", kind: "mail", title: "Postbox — 4,812 unread", at: F(0.04, 0.1, 0.48, 0.5), data: mail() },
    { id: "shot", app: "viewer", kind: "shot", title: "Screenshot 2026-03-02 at 11.48.07 PM", at: F(0.5, 0.42, 0.36, 0.4) },
    { id: "recipes", app: "paper", kind: "paper", title: "Recipes", at: F(0.62, 0.16, 0.3, 0.44), data: { heading: "Recipes", lines: ["Chili (Mom's)", "Banana bread", "Lasagna — the good one", "Pancakes"] } },
    { id: "daybook", app: "daybook", kind: "daybook", title: "Daybook", at: F(0.12, 0.36, 0.32, 0.5), data: { date: "Tuesday, June 2", events: [{ t: "Trash out", at: 19, len: 0.7, color: "#d9e3cf" }] } },
    { id: "untitled", app: "paper", kind: "paper", title: "Untitled 47", at: F(0.3, 0.3, 0.3, 0.36), data: {} },
    { id: "drill", app: "jot", kind: "jot", title: "return drill", at: F(0.68, 0.5, 0.27, 0.34), data: { heading: "return drill", lines: ["to No. 10", { t: "borrowed spring 2022", soft: 1 }] } },
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
    return { cx: x.x.t + x.w.t / 2, cy: x.y.t + x.h.t / 2, D: Math.max(x.w.t / S.SW, x.h.t / S.SH) * margin }
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

  // ── 0 · The key press: the windows on 1 tile ──
  at(0, () => {
    w.status = true
    w.ensureCell(3)
    w.tileNow()
    w.showBar()
    S.sfx("thunk", 0.8)
    S.room(0.5)
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

  // ── Moving: ⌥⇧2 sends the focused window to 2 ──
  at(13.0, () => to2(w.bar.scale, 1.7))
  clickAt(13.7, "postbox")
  at(13.7, () => w.setFocus("postbox"))
  key(14.2, ["⌥", "⇧", "2"], () => w.move("postbox", 2))
  clickAt(15.4, "browsy")
  at(15.4, () => w.setFocus("browsy"))
  key(15.9, ["⌥", "⇧", "3"], () => w.move("browsy", 3))
  S.say(16.4, "mail")
  // ── Switching: ⌥2 finds it there ──
  sw(20.8, 2)
  sw(21.9, 3)
  sw(23.0, 1)
  at(23.6, () => to2(w.bar.scale, 1))

  // ── The visitor's turn ──
  at(HOLD[0], () => S.hint(true))
  at(HOLD[1] - 0.4, () => {
    S.hint(false)
    if (w.active !== 1) w.switchTo(1)
  })

  // ── A rule: Chatter opens on 2 ──
  const settingsAt = () => {
    const sw = Math.min(S.SW * 0.86, 760)
    const sh = Math.min(S.SH * 0.6, 330)
    return [S.SW / 2 - sw / 2, S.SH * 0.42 - sh / 2, sw, sh]
  }
  at(30.2, () => {
    w.add({ id: "settings", app: "spaceman", kind: "settings", title: "Spaceman Settings", float: true, hidden: true, at: settingsAt(), data: { rules: [], add: 1 } })
    const s = R("settings")
    s.s.x = 0.96
    S.world.setFocus("settings")
    s.a.t = 1
    s.s.t = 1
    w.moveCursor(settingsAt()[0] + 240, settingsAt()[1] + 150)
  })
  S.say(30.6, "rule")
  at(31.0, () => w.set("settings", { data: { add: 0, rules: [{ app: "Chatter", icon: "chatter", action: "", ws: null, cursor: 0 }] } }))
  at(31.6, () => w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Route", ws: null }] } }))
  at(32.1, () => w.set("settings", { data: { rules: [{ app: "Chatter", icon: "chatter", action: "Route", ws: 2, hl: 1 }] } }))
  at(34.6, () => w.close("settings"))
  at(35.0, () => {
    const [x, y, ww, hh] = [S.SW * 0.55, S.SH * 0.3, S.SW * 0.3, S.SH * 0.4]
    w.open({ id: "dana", app: "chatter", kind: "chat", title: "Dana", ws: 1, at: [x, y, ww, hh], data: { msgs: [{ t: "are we still on for friday?" }, { t: "you can say no", note: "Delivered" }] } })
    S.sfx("thunk", 0.4)
  })
  at(35.9, () => w.move("dana", 2))
  S.say(36.6, "not-closed")

  // ── The drill ──
  at(40.4, () => {
    w.clock = "Wed Jun 3  6:02 AM"
    w.setFocus("drill")
  })
  at(41.0, () => w.set("drill", { data: { check: 1, lines: [{ t: "to No. 10", done: 1 }, { t: "borrowed spring 2022", soft: 1 }] } }))
  S.say(40.8, "drill")

  // ── 4 · The house ──
  sw(47.0, 4, "Sat Jun 6  9:30 AM")
  const open = (t, ws, def) => at(t, () => (w.open({ ws, ...def, at: def.at || [S.SW * 0.3, S.SH * 0.3, S.SW * 0.4, S.SH * 0.4] }), S.sfx("thunk", 0.35)))
  open(47.4, 4, { id: "groceries", app: "sheets", kind: "sheet", title: "Groceries — Standing Order", data: { cols: ["Item", "Qty", "Day"], widths: [0.5, 0.25, 0.25], rows: [["Eggs", "12", "Sun"], ["Milk", "1 gal", "Sun"], ["Bread", "1", "Sun"], ["Coffee", "2 lb", "Sun"], ["Apples", "6", "Sun"], ["Paper towels", "2", "1st Sun"]].map((cells) => ({ cells })) } })
  open(47.8, 4, { id: "laundry", app: "jot", kind: "jot", title: "Laundry — Rotation", data: { heading: "Laundry — Rotation", lines: ["Mon  whites", "Wed  darks", "Fri  towels", "Sun  sheets"] } })
  open(48.2, 4, { id: "friday", app: "daybook", kind: "daybook", title: "Friday dinner", data: { date: "Friday, June 12", events: [{ t: "Dinner (4)", at: 18.5, len: 2.5, color: "#e9d5c4" }] } })
  S.say(47.6, "house")

  // ── 5 · The neighborhood ──
  sw(50.6, 5, "Tue Jun 9  10:12 PM")
  open(50.9, 5, { id: "minutes", app: "paper", kind: "paper", title: "HOA — Minutes", data: { heading: "Pinecrest HOA — Minutes", sub: "Meeting of June 9", size: 14, gap: 23, lines: ["Present: 9 of 9", "1. Trash cans to be stored out of view", "    by 8 PM on collection days.", "2. Hedges and fences: six feet maximum.", "3. Minutes: No. 12."] } })
  open(51.3, 5, { id: "trash", app: "sheets", kind: "sheet", title: "Trash Cans — Collection", data: { cols: ["House", "Out", "In by 8"], widths: [0.4, 0.3, 0.3], rows: [["No. 8", "✓", "✓"], ["No. 10", "✓", "8:40 PM"], ["No. 12", "✓", "✓"], ["No. 14", "✓", "—"], ["No. 16", "✓", "✓"]].map((cells) => ({ cells })) } })
  open(51.7, 5, { id: "door", app: "doorbell", kind: "cam", title: "Doorbell — Front", dark: 1, data: { time: "10:12:31 PM", lit: 1 } })
  S.say(51.2, "minutes")

  // ── The hedge ──
  open(56.0, 5, { id: "hedge", app: "viewer", kind: "photos", title: "No. 14 — Hedge", data: { count: 0 } })
  let shots = 0
  span(56.2, 68, (u, t) => {
    // one photograph a morning; the mornings speed up
    const n = Math.floor(160 * Math.pow(clamp((t - 56.2) / 11.8), 2.2))
    if (n !== shots) {
      if (n < 14 || n % 9 === 0) S.sfx("shutter", n < 14 ? 0.45 : 0.25)
      shots = n
      w.set("hedge", { data: { count: n } })
      const day = Math.min(n, 140)
      w.clock = dayClock(day)
    }
  })
  push(56.4, 60.4, () => frameOf("hedge", 1.06))
  S.say(56.6, "hedge")
  S.say(63.6, "second")
  at(64.2, () => w.set("hedge", { data: { measured: 1, measuredFrom: shots } }))
  push(67.6, 69.0, home)

  // ── The minutes, again ──
  at(68.4, () => {
    w.set("minutes", { title: "HOA — Minutes (awaiting corrections)" })
    const m = R("minutes")
    m.kind = "sheet"
    m.app = "sheets"
    w.set("minutes", { data: { cols: ["Meeting", "Length", "Present"], widths: [0.4, 0.28, 0.32], rows: [] } })
    w.setFocus("minutes")
  })
  MINUTES_ROWS.forEach((r, i) => at(68.8 + i * 0.75, () => w.set("minutes", { data: { rows: MINUTES_ROWS.slice(0, i + 1).map((cells) => ({ cells })) } })))
  push(68.6, 70.4, () => frameOf("minutes", 1.25))
  S.say(69.0, "circulated")
  push(74.0, 74.9, home)

  // ── 6 · The call ──
  sw(75.0, 6, "Tue Nov 17  7:00 PM")
  const people = [{ name: "No. 12" }, { name: "No. 8", initial: "8", off: 1 }, { name: "No. 10", initial: "10", off: 1 }, { name: "No. 16", initial: "16", off: 1 }]
  people.forEach((p, i) => open(75.3 + i * 0.22, 6, { id: "call" + i, app: "call", kind: "call", title: "HOA — Board", dark: 1, data: { people: [p] } }))
  S.say(76.4, "space")

  // ── The months ──
  sw(80.5, 1, "Thu Jul 16  9:14 PM")
  sw(81.3, 4, "Fri Jul 17  8:55 PM")
  at(81.5, () => w.set("friday", { title: "Friday dinner — apologies received (3)" }))
  sw(82.2, 2, "Sat Aug 22  11:20 PM")
  at(83.0, () => {
    w.add({ id: "rules", app: "spaceman", kind: "settings", title: "Spaceman Settings", float: true, hidden: true, at: settingsAt(), data: { rules: [
      { app: "Chatter", icon: "chatter", title: "Dana", action: "Ignore", ws: null, hl: 1 },
      { app: "Chatter", icon: "chatter", action: "Route", ws: 2 },
      { app: "Doorbell", icon: "doorbell", action: "Route", ws: 5 },
      { app: "Call", icon: "call", action: "Route", ws: 6 },
    ] } })
    const s = R("rules")
    s.a.t = 1
    w.setFocus("rules")
  })
  at(85.0, () => w.close("rules"))
  sw(85.4, 3, "Mon Sep 28  6:02 AM")
  sw(86.2, 6, "Tue Nov 17  7:41 PM")
  sw(87.0, 5, "Fri Nov 20  11:46 PM")
  span(80, 88.5, (u) => {
    S.grade[0] = 0.42 * smooth(u)
    S.grade[1] = smooth(u)
    S.grade[2] = 0.3 * smooth(u)
  })
  at(80.5, () => S.room(0.28, 3))
  at(87.0, () => w.setFocus("door"))

  // ── Number fourteen ──
  push(87.4, 90.6, () => frameOf("door", 1.04))
  at(88.0, () => (w.clock = "Fri Nov 20  11:48 PM"))
  for (let s = 0; s < 14; s++) at(87.0 + s, () => w.set("door", { data: { time: `11:4${s < 6 ? 7 : 8}:${String((s * 1 + 54) % 60).padStart(2, "0")} PM` } }))
  at(88.4, () => {
    // the cursor drifts to the lit window and rests there
    const d = R("door")
    w.moveCursor(d.x.t + d.w.t * 0.36 * 1.0 + d.w.t * 0.28 * 0.66, d.y.t + BAR_H + (d.h.t - BAR_H) * 0.5, 4.6)
  })
  S.say(90.6, "light")

  // ── The pull-back ──
  span(94.2, 98.4, (u) => {
    S.pull.a = u
  })
  at(94.2, () => S.room(0.12, 1.5))
  at(94.8, () => S.sfx("car", 0.55))
  span(98.4, 100.4, (u) => {
    S.pull.b = u
  })
  at(98.4, () => S.room(0, 0.4))
  at(100.9, () => S.outerKey())
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
