import { useCallback, useEffect, useRef, useState } from "react";

export const RELEASE_VERSION = "0.2.0";
export const DOWNLOAD_URL = `https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v${RELEASE_VERSION}-macos-arm64.zip`;

type Phase = "still" | "loading" | "live" | "ended";
type Caption = { text: string; high?: boolean } | null;
type ShowApi = {
  phase: Phase;
  start(o: { context: AudioContext | null; muted: boolean }): Promise<void>;
  key(n: number): boolean;
  tap(x: number, y: number): boolean;
  destroy(): void;
};

// The still is the exhibit: the recorder, years later, on a table in a records room. Its plate is
// 1280 x 856; the view covers the section and keeps the recorder in frame on narrow screens.
const EX = { w: 1280, h: 856, focus: [0.43, 0.6] };
const RECORDER = { x: 350, y: 395, w: 362, h: 333 };

function Exhibit({ onStart, ready, live }: { onStart: () => void; ready: boolean; live: boolean }) {
  const ref = useRef<SVGSVGElement>(null);
  const [vb, setVb] = useState(`0 0 ${EX.w} ${EX.h}`);
  const [href, setHref] = useState("show/plates/exhibit-s.jpg");
  // a sharper plate once the small one is up, for screens with more pixels than it has
  useEffect(() => {
    if (window.innerWidth * devicePixelRatio <= 1600) return;
    const img = new Image();
    img.onload = () => setHref(img.src);
    const t = window.setTimeout(() => (img.src = "show/plates/exhibit-l.jpg"), 400);
    return () => window.clearTimeout(t);
  }, []);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height) return;
      const A = r.width / r.height;
      const w = A > EX.w / EX.h ? EX.w : EX.h * A;
      const h = A > EX.w / EX.h ? EX.w / A : EX.h;
      const x = Math.min(EX.w - w, Math.max(0, EX.focus[0] * EX.w - w / 2));
      const y = Math.min(EX.h - h, Math.max(0, EX.focus[1] * EX.h - h / 2));
      setVb(`${x.toFixed(1)} ${y.toFixed(1)} ${w.toFixed(1)} ${h.toFixed(1)}`);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const tag = ["Cassette recorder", "Kitchen, No. 14", "Found running.", "Side A"];
  return (
    <svg ref={ref} viewBox={vb} preserveAspectRatio="xMidYMid slice" className={`absolute inset-0 size-full transition-opacity duration-700 ${live ? "opacity-0" : "opacity-100"}`}>
      <title>Exhibit 31: a cassette recorder found running in the kitchen of No. 14</title>
      <image href={href} width={EX.w} height={EX.h} />
      <g transform="rotate(-30 604 572)" fill="#2b2119" fillOpacity={0.86} style={{ fontFamily: '"Courier New", Courier, ui-monospace, monospace' }}>
        <text x={604} y={522} textAnchor="middle" fontSize={15} fontWeight={700} letterSpacing={1.5}>EXHIBIT 31</text>
        <rect x={562} y={528} width={84} height={0.8} />
        {tag.map((l, i) => (
          <text key={l} x={604} y={546 + i * 15} textAnchor="middle" fontSize={10.5}>{l}</text>
        ))}
      </g>
      <rect
        role="button"
        tabIndex={0}
        aria-label="Play the recording"
        aria-disabled={!ready}
        onClick={onStart}
        onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onStart())}
        x={RECORDER.x}
        y={RECORDER.y}
        width={RECORDER.w}
        height={RECORDER.h}
        rx={14}
        className={`fill-white/0 outline-none transition-[fill] duration-300 hover:fill-white/[.035] focus-visible:fill-white/[.05] ${ready ? "cursor-pointer" : "cursor-wait"}`}
      />
    </svg>
  );
}

function newContext(): AudioContext | null {
  try {
    const C = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    return C ? new C() : null;
  } catch {
    return null;
  }
}

export function Show({ header }: { header: React.ReactNode }) {
  const section = useRef<HTMLElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const show = useRef<ShowApi | null>(null);
  const context = useRef<AudioContext | null>(null);
  const [phase, setPhase] = useState<Phase>("still");
  const [card, setCard] = useState(false);
  const [ready, setReady] = useState(false);
  const [caption, setCaption] = useState<Caption>(null);
  const [hint, setHint] = useState(false);
  const [cc, setCc] = useState<boolean | null>(null);
  const [touch, setTouch] = useState(false);

  useEffect(() => {
    setTouch(matchMedia("(hover: none) and (pointer: coarse)").matches);
    const params = new URLSearchParams(location.search);
    const capture = params.has("capture");
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches || params.has("reduced");
    let cancelled = false;
    const load = () =>
      import("./show/main.js").then(({ createShow }) => {
        if (cancelled || !canvas.current) return;
        show.current = createShow({
          canvas: canvas.current,
          base: new URL("show/", document.baseURI).href,
          capture,
          reduced,
          on: {
            phase: (p: Phase) => {
              setPhase(p);
              // a hard cut: the last image, then the card, nothing between
              if (p === "ended") setCard(true);
              if (p === "live") setCard(false);
            },
            caption: (c: Caption) => setCaption(c),
            hint: (h: boolean) => setHint(h),
          },
        }) as ShowApi;
        setReady(true);
      });
    // The engine is small; the voice bank and WebGPU wait for the key press.
    const idle = (window as unknown as { requestIdleCallback?: (f: () => void, o?: object) => number }).requestIdleCallback;
    if (capture) void load();
    else if (idle) idle(() => void load(), { timeout: 1200 });
    else window.setTimeout(() => void load(), 300);
    return () => {
      cancelled = true;
      show.current?.destroy();
      show.current = null;
    };
  }, []);

  const start = useCallback(() => {
    const s = show.current;
    if (!s || (s.phase !== "still" && s.phase !== "ended")) return;
    if (!context.current) context.current = newContext();
    setCard(false);
    void s.start({ context: context.current, muted: false });
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) return;
      const m = /^Digit(\d)$/.exec(e.code);
      if (!m) return;
      const n = Number(m[1]);
      const s = show.current;
      if (!s) return;
      if (s.phase === "still" && n === 1) {
        e.preventDefault();
        start();
      } else if (s.phase === "live" && e.altKey && s.key(n)) e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [start]);

  const onTap = (e: React.PointerEvent) => {
    const r = canvas.current?.getBoundingClientRect();
    if (r) show.current?.tap(e.clientX - r.left, e.clientY - r.top);
  };

  const captions = cc ?? false;
  const live = phase === "live" || phase === "loading";

  return (
    <section
      ref={section}
      data-phase={phase}
      aria-label="Spaceman, a short film"
      className="relative h-[100svh] min-h-[480px] w-full overflow-hidden bg-[#08080a] select-none"
    >
      {phase !== "ended" && <Exhibit onStart={start} ready={ready} live={phase === "live"} />}
      <canvas ref={canvas} aria-hidden="true" onPointerDown={onTap} className={`absolute inset-0 size-full ${phase === "live" ? "opacity-100" : "opacity-0"}`} />

      <div className={`pointer-events-none absolute inset-x-0 top-0 transition-opacity duration-500 ${live ? "opacity-0" : "opacity-100"}`}>
        <div className={live ? "" : "pointer-events-auto"}>{header}</div>
      </div>

      {(phase === "still" || phase === "loading") && (
        <div className="pointer-events-none absolute inset-x-0 bottom-[7%] flex flex-col items-center gap-1.5 px-6 text-center">
          <p className="text-[15px] text-[#e4e1da] [text-shadow:0_1px_12px_rgba(0,0,0,.9)]">
            {phase === "loading" ? "…" : touch ? "Tap the recorder." : "Press ⌥1, or click the recorder."}
          </p>
          <p className="text-[13px] text-[#9d9991] [text-shadow:0_1px_10px_rgba(0,0,0,.9)]">A short film about Spaceman, a tiling window manager for macOS · 2:30 · sound on</p>
        </div>
      )}

      {live && (
        <div className="absolute top-3 left-3 flex gap-2 sm:top-auto sm:left-auto sm:right-6 sm:bottom-6">
          <button type="button" onClick={() => setCc(!captions)} aria-pressed={captions} className={`h-9 rounded-full border border-white/10 px-3 text-[12px] font-semibold tracking-wide backdrop-blur ${captions ? "bg-[#ece9e3] text-[#151517]" : "bg-black/45 text-[#d8d5ce] hover:bg-black/60"}`}>
            CC
          </button>
        </div>
      )}

      {captions && caption && phase === "live" && (
        <p aria-live="polite" className={`pointer-events-none absolute inset-x-0 ${caption.high ? "bottom-[64%]" : "bottom-[17%]"} mx-auto w-fit max-w-[min(90vw,46rem)] rounded-lg bg-black/70 px-4 py-2 text-center text-[clamp(15px,2.1vw,21px)] leading-snug text-[#f4f2ee]`}>
          {caption.text}
        </p>
      )}

      {hint && phase === "live" && (
        <p className="pointer-events-none absolute inset-x-0 bottom-[26%] mx-auto w-fit rounded-full bg-black/70 px-4 py-2 text-[15px] text-[#ece9e3] backdrop-blur">
          {touch ? "Your turn. Tap a workspace in the bar." : "Your turn. Try ⌥2, then ⌥3."}
        </p>
      )}

      {phase === "ended" && (
        <div className={`absolute inset-0 flex flex-col items-center justify-center gap-7 bg-[#08080a] px-6 text-center ${card ? "opacity-100" : "opacity-0"}`}>
          <p className="text-[clamp(28px,4.4vw,46px)] font-medium tracking-tight text-[#f2f1ee]">For your windows.</p>
          <div className="flex flex-col items-center gap-3">
            <a href={DOWNLOAD_URL} className="rounded-full bg-[#f0efec] px-7 py-3 text-[15px] font-medium text-[#111214] transition-opacity hover:opacity-85 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#cfcac0]">
              Download for macOS
            </a>
            <p className="text-[13px] text-[#7d7a74]">Spaceman {RELEASE_VERSION} · macOS 14 or later · Apple silicon</p>
          </div>
          <button type="button" onClick={start} className="text-[13px] text-[#9a968f] underline-offset-4 hover:underline">
            Watch again
          </button>
        </div>
      )}
    </section>
  );
}
