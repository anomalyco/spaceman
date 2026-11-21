import { motion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { DOWNLOAD_URL, RELEASE_VERSION, Show } from "./Show";

const CHECKSUM_URL = `${DOWNLOAD_URL}.sha256`;
const INSTALL = `curl -fL https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-latest-macos-arm64.zip -o /tmp/Spaceman.zip \\
  && ditto -x -k /tmp/Spaceman.zip /Applications && open /Applications/Spaceman.app`;

export function App() {
  return (
    <main className="bg-[#0b0c0e] text-[#f2f2f0] antialiased">
      <Show header={<Header />} />
      <Product />
    </main>
  );
}

function Header() {
  return (
    <header className="flex items-center justify-between px-5 py-4 sm:px-7 sm:py-5">
      <h1 className="flex items-center gap-3">
        <img src="mark.svg" alt="" width={28} height={28} className="size-7 rounded-[8px]" />
        <span className="sr-only">Spaceman</span>
        <Wordmark />
      </h1>
      <nav className="flex items-center gap-5 text-[13px] text-[#c9c6bf]">
        <a href="llms-full.txt" className="hidden hover:text-white sm:inline">Docs</a>
        <a href={DOWNLOAD_URL} className="rounded-full border border-white/15 bg-black/30 px-4 py-1.5 backdrop-blur hover:bg-black/50">Download</a>
      </nav>
    </header>
  );
}

function Product() {
  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-24 px-6 py-24 sm:py-32">
      <section className="flex flex-col gap-8">
        <h2 className="max-w-2xl text-[clamp(26px,3.6vw,38px)] leading-tight font-medium tracking-tight">
          Spaceman is a tiling window manager for macOS.
        </h2>
        <p className="max-w-2xl text-[17px] leading-relaxed text-[#a3a29d]">
          Windows open side by side instead of on top of each other, across ten workspaces you switch with the keyboard.
        </p>
        <Install />
      </section>

      <section aria-label="Features" className="grid gap-x-12 gap-y-20 md:grid-cols-2">
        <Feature title="Windows tile as they open" demo={<TilingDemo />}>
          Each new window takes a share of the screen and the others make room. Drag a window onto another to swap them, or drag an edge to resize the split.
        </Feature>
        <Feature title="Ten workspaces on the keyboard" demo={<WorkspacesDemo />}>
          <Kbd>⌥1</Kbd> to <Kbd>⌥0</Kbd> switch workspaces. Add <Kbd>⇧</Kbd> to send the focused window to one without following it. A small capsule shows each workspace's layout as you go.
        </Feature>
        <Feature title="Rules for where apps go" demo={<RulesStill />}>
          Send an app to a workspace when it opens, or leave a window alone by its title. Rules are checked in order and the first match wins.
        </Feature>
        <Feature title="Your windows come back" demo={<RestoreStill />}>
          Pausing, quitting or updating Spaceman brings every hidden window back on screen. It needs Accessibility access and nothing else, so System Integrity Protection stays on.
        </Feature>
      </section>

      <footer className="flex flex-col items-start gap-6 border-t border-white/8 pt-10 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-col gap-1.5">
          <a href={DOWNLOAD_URL} className="w-fit rounded-full bg-[#f0efec] px-6 py-2.5 text-[14px] font-medium text-[#111214] hover:opacity-85">
            Download for macOS
          </a>
          <p className="text-[13px] text-[#6f6e6a]">
            Spaceman {RELEASE_VERSION} · macOS 14 or later · Apple silicon preview ·{" "}
            <a href={CHECKSUM_URL} className="underline-offset-4 hover:underline">SHA-256</a>
          </p>
        </div>
        <a href="llms-full.txt" className="text-[14px] text-[#c9c6bf] underline-offset-4 hover:underline">
          Read the documentation
        </a>
      </footer>
      {import.meta.env.VITE_BUILD_STAMP && <p className="-mt-8 text-[11px] text-[#55544f]">Build: {import.meta.env.VITE_BUILD_STAMP}</p>}
    </div>
  );
}

function Install() {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex max-w-3xl flex-col gap-2.5">
      <div className="relative rounded-xl border border-white/10 bg-[#121316]">
        <pre className="mr-20 overflow-x-auto py-4 pl-5 font-mono text-[13px] leading-relaxed text-[#d9d7d1]">
          <code>{INSTALL}</code>
        </pre>
        <button
          type="button"
          onClick={() => {
            void navigator.clipboard?.writeText(INSTALL.replace(/\\\n\s*/g, "")).then(() => {
              setCopied(true);
              window.setTimeout(() => setCopied(false), 1500);
            });
          }}
          className="absolute top-3 right-3 rounded-md border border-white/10 bg-white/5 px-2.5 py-1 text-[12px] text-[#c9c6bf] hover:bg-white/10"
        >
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <p className="text-[13px] text-[#76756f]">Turn off any other window manager first. The setup tour asks for Accessibility access.</p>
    </div>
  );
}

function Feature({ title, demo, children }: { title: string; demo: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-5">
      <div aria-hidden="true" inert className="aspect-[16/10] overflow-hidden rounded-xl border border-white/8 bg-[#141518]">
        {demo}
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="text-[18px] font-medium">{title}</h3>
        <p className="text-[15px] leading-relaxed text-[#a3a29d]">{children}</p>
      </div>
    </div>
  );
}

function Kbd({ children }: { children: React.ReactNode }) {
  return <kbd className="rounded-[5px] border border-white/12 bg-white/6 px-1.5 py-px font-sans text-[13px] text-[#e4e2dc]">{children}</kbd>;
}

// ── Demos ───────────────────────────────────────────────────────────────

type R = { x: number; y: number; w: number; h: number };
const APP_COLORS = ["#e9b93f", "#4f8ad6", "#4fb3a7", "#8a74d1", "#d9803a", "#5cb86e"];

function tiles(n: number): R[] {
  if (n === 1) return [{ x: 0, y: 0, w: 1, h: 1 }];
  if (n === 2) return [{ x: 0, y: 0, w: 0.5, h: 1 }, { x: 0.5, y: 0, w: 0.5, h: 1 }];
  if (n === 3) return [{ x: 0, y: 0, w: 0.5, h: 1 }, { x: 0.5, y: 0, w: 0.5, h: 0.5 }, { x: 0.5, y: 0.5, w: 0.5, h: 0.5 }];
  return [{ x: 0, y: 0, w: 0.5, h: 0.5 }, { x: 0.5, y: 0, w: 0.5, h: 0.5 }, { x: 0, y: 0.5, w: 0.5, h: 0.5 }, { x: 0.5, y: 0.5, w: 0.5, h: 0.5 }];
}

// Steps through a cycle while the demo is on screen.
function useCycle(length: number, ms: number) {
  const ref = useRef<HTMLDivElement>(null);
  const [i, setI] = useState(0);
  useEffect(() => {
    let timer = 0;
    const io = new IntersectionObserver(([e]) => {
      window.clearInterval(timer);
      if (e.isIntersecting) timer = window.setInterval(() => setI((x) => (x + 1) % length), ms);
    });
    if (ref.current) io.observe(ref.current);
    return () => {
      io.disconnect();
      window.clearInterval(timer);
    };
  }, [length, ms]);
  return [ref, i] as const;
}

function MiniWindow({ r, color, pad = 0.012 }: { r: R; color: string; pad?: number }) {
  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.94 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.94 }}
      transition={{ type: "spring", duration: 0.42, bounce: 0 }}
      className="absolute overflow-hidden rounded-[6px] bg-[#ebe8e2] shadow-[0_6px_18px_rgba(0,0,0,.35)]"
      style={{ left: `${(r.x + pad) * 100}%`, top: `${(r.y + pad * 1.6) * 100}%`, width: `${(r.w - pad * 2) * 100}%`, height: `${(r.h - pad * 3.2) * 100}%` }}
    >
      <div className="flex h-[14%] max-h-4 min-h-2.5 items-center gap-[3px] bg-[#dedad3] px-1.5">
        {[0, 1, 2].map((k) => <span key={k} className="size-[5px] rounded-full bg-[#c4bfb7]" />)}
      </div>
      <div className="flex flex-col gap-[6%] p-[6%]">
        <span className="h-1.5 w-1/2 rounded-full" style={{ background: color }} />
        <span className="h-1 w-4/5 rounded-full bg-[#d3cfc7]" />
        <span className="h-1 w-3/5 rounded-full bg-[#d3cfc7]" />
      </div>
    </motion.div>
  );
}

function TilingDemo() {
  const [ref, i] = useCycle(5, 1300);
  const n = i === 0 ? 1 : i;
  const rs = tiles(Math.min(n, 4));
  return (
    <div ref={ref} className="relative size-full bg-[linear-gradient(135deg,#465363,#262c35)] p-[3%]">
      <div className="relative size-full">
        {rs.map((r, k) => <MiniWindow key={k} r={r} color={APP_COLORS[k]} />)}
      </div>
    </div>
  );
}

const SPACES = [
  [{ c: 0 }, { c: 1 }, { c: 2 }],
  [{ c: 3 }, { c: 4 }],
  [{ c: 5 }],
];

function WorkspacesDemo() {
  const [ref, i] = useCycle(3, 2000);
  return (
    <div ref={ref} className="relative size-full bg-[linear-gradient(135deg,#465363,#262c35)] p-[3%]">
      <div className="relative h-[78%] w-full">
        {tiles(SPACES[i].length).map((r, k) => <MiniWindow key={`${i}-${k}`} r={r} color={APP_COLORS[SPACES[i][k].c]} />)}
      </div>
      <div className="absolute inset-x-0 bottom-[5%] flex justify-center">
        <div className="relative flex items-center gap-2 rounded-full border border-white/10 bg-[#161618] px-3 py-1.5">
          {SPACES.map((s, k) => (
            <div key={k} className="relative">
              {k === i && <motion.span layoutId="ws-marker" transition={{ type: "spring", duration: 0.34, bounce: 0 }} className="absolute -inset-[3px] rounded-[7px] border border-white/20 bg-white/10" />}
              <motion.div animate={{ width: k === i ? 46 : 29, height: k === i ? 29 : 18, opacity: k === i ? 1 : 0.55 }} transition={{ type: "spring", duration: 0.34, bounce: 0 }} className="relative rounded-[4px] border border-white/20 bg-white/5">
                {tiles(s.length).map((r, j) => (
                  <span key={j} className="absolute rounded-[2px]" style={{ left: `${r.x * 100 + 6}%`, top: `${r.y * 100 + 8}%`, width: `${r.w * 100 - 12}%`, height: `${r.h * 100 - 16}%`, background: APP_COLORS[s[j].c], opacity: 0.8 }} />
                ))}
              </motion.div>
            </div>
          ))}
        </div>
      </div>
      <div className="absolute top-[6%] right-[4%] rounded-md bg-black/50 px-2 py-0.5 font-sans text-[12px] text-[#ece9e3]">⌥{i + 1}</div>
    </div>
  );
}

const RULES = [
  { app: "Chat", title: "Standup", action: "Ignore", ws: "" },
  { app: "Chat", title: "Any", action: "Route", ws: "2" },
  { app: "Mail", title: "Any", action: "Route", ws: "2" },
  { app: "Browser", title: "Any", action: "Route", ws: "3" },
];

function RulesStill() {
  return (
    <div className="flex size-full flex-col justify-center gap-1.5 bg-[#f2f0ec] px-[6%] text-[#2b2a28]">
      <div className="grid grid-cols-[1.1fr_1.2fr_1fr_0.8fr] px-2 pb-1 text-[11px] font-semibold text-[#7b776f]">
        <span>App</span><span>Window title</span><span>Action</span><span>Workspace</span>
      </div>
      {RULES.map((r, k) => (
        <div key={k} className="grid grid-cols-[1.1fr_1.2fr_1fr_0.8fr] items-center rounded-md bg-white px-2 py-1.5 text-[13px] shadow-[0_1px_0_rgba(0,0,0,.05)]">
          <span className="flex items-center gap-1.5"><span className="size-3 rounded-[3px]" style={{ background: APP_COLORS[k + 1] }} />{r.app}</span>
          <span className={r.title === "Any" ? "text-[#9a968e]" : ""}>{r.title === "Any" ? "Any" : `“${r.title}”`}</span>
          <span>{r.action}</span>
          <span>{r.ws || "—"}</span>
        </div>
      ))}
    </div>
  );
}

function RestoreStill() {
  return (
    <div className="relative size-full bg-[linear-gradient(135deg,#465363,#262c35)]">
      <div className="absolute top-0 inset-x-0 flex h-[9%] items-center justify-end gap-3 bg-[#ece9e3]/85 px-[3%] text-[11px] text-[#2b2a28]">
        <span className="rounded-[4px] bg-[#2b2a28] px-1.5 py-px text-[10px] font-bold text-[#ece9e3]">1</span>
        <span>Disable Tiling</span>
      </div>
      {[{ x: 0.07, y: 0.2, w: 0.42, h: 0.5 }, { x: 0.38, y: 0.3, w: 0.44, h: 0.48 }, { x: 0.55, y: 0.16, w: 0.36, h: 0.42 }].map((r, k) => (
        <MiniWindow key={k} r={r} color={APP_COLORS[k + 2]} pad={0} />
      ))}
    </div>
  );
}

/**
 * Hand-drawn monoline wordmark: thin geometric caps with a crossbar-less A.
 * Glyphs live on an 80-unit cap height; strokes stay unfilled monolines.
 */
const GLYPHS: Record<string, { width: number; d: string }> = {
  S: {
    width: 52,
    d: "M47 13 C44 4 36 0 26 0 C13 0 5 6 5 16 C5 26 12 31 26 34 C41 37 49 43 49 55 C49 70 40 80 26 80 C14 80 6 75 3 67",
  },
  P: {
    width: 55,
    d: "M7 80 L7 0 L31 0 C44 0 51 8 51 21 C51 34 44 42 31 42 L7 42",
  },
  A: { width: 60, d: "M4 80 L30 0 L56 80" },
  C: {
    width: 54,
    d: "M49 14 C45 4 37 0 27 0 C11 0 5 13 5 40 C5 67 11 80 27 80 C37 80 45 76 49 66",
  },
  E: { width: 49, d: "M45 0 L9 0 L9 80 L45 80 M9 40 L35 40" },
  M: { width: 60, d: "M6 80 L6 0 L30 44 L54 0 L54 80" },
  N: { width: 56, d: "M7 80 L7 0 L49 80 L49 0" },
};

const WORDMARK_GAP = 24;

function Wordmark() {
  let x = 0;
  const placed = "SPACEMAN".split("").map((letter, index) => {
    const glyph = GLYPHS[letter];
    const entry = { key: `${letter}-${index}`, glyph, x };
    x += glyph.width + WORDMARK_GAP;
    return entry;
  });
  const total = x - WORDMARK_GAP;

  return (
    <svg
      viewBox={`-4 -5 ${total + 8} 90`}
      className="h-6"
      aria-hidden="true"
    >
      {placed.map((entry) => (
        <path
          key={entry.key}
          d={entry.glyph.d}
          transform={`translate(${entry.x},0)`}
          fill="none"
          stroke="rgba(255,255,255,0.92)"
          strokeWidth="5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      ))}
    </svg>
  );
}
