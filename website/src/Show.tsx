import { useCallback, useEffect, useRef, useState } from "react";

export const RELEASE_VERSION = "0.2.0";
export const DOWNLOAD_URL = `https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v${RELEASE_VERSION}-macos-arm64.zip`;

type Phase = "still" | "loading" | "live" | "ended";
type Caption = { text: string; high?: boolean } | null;
type ShowApi = {
  phase: Phase;
  time: number;
  duration: number;
  paused: boolean;
  start(o: { context: AudioContext | null; muted: boolean }): Promise<void>;
  setPaused(p: boolean): void;
  seek(t: number): void;
  setMuted(muted: boolean, context: AudioContext | null): void;
  key(n: number): boolean;
  tap(x: number, y: number): boolean;
  destroy(): void;
};

const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;

// The player's controls: a quiet strip along the bottom that shows while the pointer moves or the film
// is paused, and is gone otherwise. The fill and the time are written straight to the DOM each frame.
function Controls({ show, shown, paused, muted, captions, onCaptions, onMuted, onToggle }: {
  show: React.RefObject<ShowApi | null>;
  shown: boolean;
  paused: boolean;
  muted: boolean;
  captions: boolean;
  onCaptions: () => void;
  onMuted: () => void;
  onToggle: () => void;
}) {
  const track = useRef<HTMLDivElement>(null);
  const fill = useRef<HTMLDivElement>(null);
  const time = useRef<HTMLSpanElement>(null);
  const drag = useRef<number | null>(null);
  const [over, setOver] = useState(false);
  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const s = show.current;
      if (s && fill.current && time.current) {
        const t = drag.current ?? s.time;
        fill.current.style.transform = `scaleX(${Math.min(1, t / s.duration)})`;
        time.current.textContent = `${clock(t)} / ${clock(s.duration)}`;
      }
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  }, [show]);
  const at = (e: React.PointerEvent) => {
    const r = track.current!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * (show.current?.duration ?? 0);
  };
  const btn = "grid size-8 place-items-center rounded-full text-white/75 transition-colors hover:bg-white/10 hover:text-white focus-visible:outline-1 focus-visible:outline-white/50";
  return (
    <div
      data-ui="controls"
      onPointerDown={(e) => e.stopPropagation()}
      onPointerEnter={() => setOver(true)}
      onPointerLeave={() => setOver(false)}
      className={`absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/45 to-transparent px-4 pt-10 pb-3 transition-opacity duration-500 focus-within:opacity-100 sm:px-6 ${shown || over ? "opacity-100" : "pointer-events-none opacity-0"}`}
    >
      <div className="flex items-center gap-2 text-[12px] text-white/70">
        <button type="button" data-ui="play" aria-label={paused ? "Play" : "Pause"} onClick={onToggle} className={btn}>
          {paused ? (
            <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor"><path d="M3.5 1.8v10.4c0 .5.5.8.9.5l8-5.2a.6.6 0 0 0 0-1L4.4 1.3c-.4-.3-.9 0-.9.5Z" /></svg>
          ) : (
            <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor"><rect x="3" y="2" width="2.8" height="10" rx=".8" /><rect x="8.2" y="2" width="2.8" height="10" rx=".8" /></svg>
          )}
        </button>
        <button type="button" data-ui="restart" aria-label="Restart" onClick={() => show.current?.seek(0)} className={btn}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M2.5 7a4.5 4.5 0 1 0 1.4-3.3" /><path d="M2.3 1.8v2.6h2.6" /></svg>
        </button>
        <span ref={time} className="w-[84px] text-center tabular-nums" />
        <div
          ref={track}
          data-ui="scrub"
          role="slider"
          aria-label="Seek"
          aria-valuemin={0}
          aria-valuemax={100}
          tabIndex={-1}
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            drag.current = at(e);
          }}
          onPointerMove={(e) => {
            if (drag.current !== null) drag.current = at(e);
          }}
          onPointerUp={(e) => {
            if (drag.current === null) return;
            show.current?.seek(at(e));
            drag.current = null;
          }}
          className="group relative flex h-6 flex-1 cursor-pointer items-center"
        >
          <div className="relative h-[3px] w-full overflow-hidden rounded-full bg-white/20 transition-[height] group-hover:h-[5px]">
            <div ref={fill} className="absolute inset-0 origin-left bg-white/80" style={{ transform: "scaleX(0)" }} />
          </div>
        </div>
        <button type="button" data-ui="cc" aria-label="Captions" aria-pressed={captions} onClick={onCaptions} className={`${btn} w-auto px-2 text-[11px] font-semibold tracking-wide ${captions ? "text-white" : ""}`}>
          <span className={captions ? "border-b border-white/80" : ""}>CC</span>
        </button>
        <button type="button" data-ui="mute" aria-label={muted ? "Unmute" : "Mute"} onClick={onMuted} className={btn}>
          <svg width="16" height="14" viewBox="0 0 16 14" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
            <path d="M2 5h2.4L8 2v10L4.4 9H2z" fill="currentColor" stroke="none" />
            {muted ? <path d="M10.5 5l3.5 4M14 5l-3.5 4" /> : <path d="M10.6 4.6a3.4 3.4 0 0 1 0 4.8M12.4 2.8a6 6 0 0 1 0 8.4" />}
          </svg>
        </button>
      </div>
    </div>
  );
}

// The still is the study at night with the screen asleep, a Cycles render (show/scenes/study/hero.py)
// that is also the opening plate's first frame. A click anywhere on it starts the film. It is
// 2400 x 1500; the view covers the section, centred on the monitor as far as the frame allows (as the
// plate is, so the cut to the film doesn't move).
const EX = { w: 2400, h: 1500, focus: [0.401, 0.431] };

function Still({ onStart, live }: { onStart: () => void; live: boolean }) {
  const ref = useRef<SVGSVGElement>(null);
  const [vb, setVb] = useState(`0 0 ${EX.w} ${EX.h}`);
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
  return (
    <svg
      ref={ref}
      viewBox={vb}
      preserveAspectRatio="xMidYMid slice"
      role="button"
      tabIndex={0}
      aria-label="Play"
      onClick={onStart}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onStart())}
      className={`absolute inset-0 size-full cursor-pointer outline-none transition-opacity duration-700 ${live ? "pointer-events-none opacity-0" : "opacity-100"}`}
    >
      <image href="show/renders/intro.jpg" width={EX.w} height={EX.h} />
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
  const cardTimer = useRef(0);
  const [caption, setCaption] = useState<Caption>(null);
  const [hint, setHint] = useState(false);
  const [cc, setCc] = useState<boolean | null>(null);
  const [touch, setTouch] = useState(false);
  // the CC button shows while the pointer moves (or a key is pressed), then gets out of the picture
  const [awake, setAwake] = useState(true);
  const [paused, setPaused] = useState(false);
  const [muted, setMuted] = useState(false);
  // a click before the engine has loaded starts the film as soon as it has
  const pending = useRef(false);

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
              // a hard cut: the last image, a moment of black and silence, then the card, with no fades
              clearTimeout(cardTimer.current);
              if (p === "ended") cardTimer.current = window.setTimeout(() => setCard(true), capture ? 0 : 650);
              if (p === "live") setCard(false);
            },
            caption: (c: Caption) => setCaption(c),
            paused: (p: boolean) => setPaused(p),
            hint: (h: boolean) => setHint(h),
          },
        }) as ShowApi;
        if (pending.current) {
          pending.current = false;
          setCard(false);
          void show.current.start({ context: context.current, muted: false });
        }
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
    // the AudioContext is made inside the click, whenever the film itself gets going
    if (!context.current) context.current = newContext();
    const s = show.current;
    if (!s) {
      pending.current = true;
      return;
    }
    if (s.phase !== "still" && s.phase !== "ended") return;
    setCard(false);
    setPaused(false);
    void s.start({ context: context.current, muted });
  }, [muted]);

  const toggle = useCallback(() => {
    const s = show.current;
    if (s?.phase === "live") s.setPaused(!s.paused);
  }, []);
  const toggleMute = useCallback(() => {
    const m = !muted;
    setMuted(m);
    show.current?.setMuted(m, context.current);
  }, [muted]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) return;
      const s0 = show.current;
      if (s0?.phase === "live" && !e.altKey) {
        const onButton = e.target instanceof HTMLElement && e.target.closest("button, a, input");
        if (e.code === "Space" && !onButton) {
          e.preventDefault();
          s0.setPaused(!s0.paused);
          return;
        }
        if (e.code === "ArrowLeft" || e.code === "ArrowRight") {
          e.preventDefault();
          s0.seek(s0.time + (e.code === "ArrowLeft" ? -5 : 5));
          return;
        }
        if (e.code === "KeyM") {
          toggleMute();
          return;
        }
      }
      const m = /^Digit(\d)$/.exec(e.code);
      if (!m) return;
      const n = Number(m[1]);
      const s = show.current;
      if (!s) {
        if (n === 1 && e.altKey) start();
        return;
      }
      if (s.phase === "still" && n === 1) {
        e.preventDefault();
        start();
      } else if (s.phase === "live" && e.altKey && s.key(n)) e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [start, toggleMute]);

  useEffect(() => {
    if (phase !== "live") return;
    let timer = 0;
    const wake = () => {
      setAwake(true);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setAwake(false), 2500);
    };
    wake();
    const events = ["pointermove", "pointerdown", "keydown"] as const;
    for (const e of events) window.addEventListener(e, wake);
    return () => {
      window.clearTimeout(timer);
      for (const e of events) window.removeEventListener(e, wake);
    };
  }, [phase]);

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
      aria-label="Spaceman"
      className="relative h-[100svh] min-h-[480px] w-full overflow-hidden bg-[#08080a] select-none"
    >
      {phase !== "ended" && <Still onStart={start} live={phase === "live"} />}
      <canvas ref={canvas} aria-hidden="true" onPointerDown={onTap} className={`absolute inset-0 size-full ${phase === "live" ? "opacity-100" : "pointer-events-none opacity-0"}`} />

      <div className={`pointer-events-none absolute inset-x-0 top-0 transition-opacity duration-500 ${live || phase === "ended" ? "opacity-0" : "opacity-100"}`}>
        <div className={live || phase === "ended" ? "" : "pointer-events-auto"}>{header}</div>
      </div>

      {live && (
        <Controls show={show} shown={awake || paused} paused={paused} muted={muted} captions={captions} onCaptions={() => setCc(!captions)} onMuted={toggleMute} onToggle={toggle} />
      )}

      {captions && caption && phase === "live" && (
        <p aria-live="polite" className={`pointer-events-none absolute inset-x-0 ${caption.high ? "bottom-[64%]" : "bottom-[17%]"} mx-auto w-fit max-w-[min(90vw,46rem)] rounded-lg bg-black/70 px-4 py-2 text-center text-[clamp(15px,2.1vw,21px)] leading-snug text-[#f4f2ee]`}>
          {caption.text}
        </p>
      )}

      {hint && phase === "live" && (
        <p data-ui="hint" className="pointer-events-none absolute inset-x-0 bottom-[26%] mx-auto w-fit rounded-full bg-black/70 px-4 py-2 text-[15px] text-[#ece9e3] backdrop-blur">
          {touch ? "Your turn. Tap a workspace in the bar." : "Your turn. Try ⌥2, then ⌥3."}
        </p>
      )}

      {phase === "ended" && (
        // black and silence for a breath, then daylight: the plain page, the way it looks on any other day
        <div className={`absolute inset-0 ${card ? "bg-[#fbf9f4]" : "bg-black"}`}>
          <div
            data-ui="card"
            className={`relative flex size-full flex-col items-center justify-center gap-8 px-6 text-center ${card ? "" : "invisible"}`}
            style={{ background: "radial-gradient(120% 80% at 50% -10%, #fff6dc 0%, #fdf8ec 38%, #fbf9f4 70%), #fbf9f4" }}
          >
            <img src="icon-512.png" alt="" width={104} height={104} className="size-[clamp(80px,9vw,104px)] rounded-[23%] shadow-[0_1px_2px_rgba(60,45,10,.12),0_14px_34px_-10px_rgba(60,45,10,.32)]" />
            <div className="flex flex-col items-center gap-1.5">
              <p className="text-[clamp(30px,4vw,44px)] leading-none font-semibold tracking-[-0.02em] text-[#1d1d1f]">Spaceman</p>
              <p className="text-[clamp(16px,1.7vw,19px)] text-[#6e6b64]">For your windows.</p>
            </div>
            <div className="flex flex-col items-center gap-3.5">
              <a
                href={DOWNLOAD_URL}
                className="rounded-full bg-[#1d1d1f] px-7 py-3 text-[15px] font-medium text-white shadow-[0_1px_1px_rgba(0,0,0,.08),0_6px_16px_-6px_rgba(0,0,0,.35)] transition-[background-color,transform] duration-150 hover:bg-[#2c2c2f] active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#1d1d1f]"
              >
                Download for macOS
              </a>
              <p className="rounded-full bg-[#1d1d1f]/[0.05] px-3 py-1 text-[12.5px] text-[#6e6b64] tabular-nums">
                Version {RELEASE_VERSION} · macOS 14 or later · Apple silicon
              </p>
            </div>
            <button type="button" data-ui="again" onClick={start} className="text-[13px] text-[#8a867d] underline-offset-4 transition-colors hover:text-[#1d1d1f] hover:underline">
              Watch again
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
