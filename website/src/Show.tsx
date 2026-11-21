import { useCallback, useEffect, useRef, useState } from "react";

export const RELEASE_VERSION = "0.2.0";
export const DOWNLOAD_URL = `https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v${RELEASE_VERSION}-macos-arm64.zip`;

type Phase = "still" | "loading" | "live" | "ended";
type Caption = { id: string; text: string } | null;
type ShowApi = {
  phase: Phase;
  start(o: { context: AudioContext | null; muted: boolean }): Promise<void>;
  setMuted(muted: boolean, context: AudioContext | null): void;
  key(n: number): boolean;
  tap(x: number, y: number): boolean;
  skip(): void;
  destroy(): void;
};

const MUTE_KEY = "spaceman-show-muted";

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
  const still = useRef<HTMLCanvasElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const show = useRef<ShowApi | null>(null);
  const context = useRef<AudioContext | null>(null);
  const [phase, setPhase] = useState<Phase>("still");
  const [card, setCard] = useState(false);
  const [ready, setReady] = useState(false);
  const [caption, setCaption] = useState<Caption>(null);
  const [hint, setHint] = useState(false);
  const [muted, setMuted] = useState(false);
  const [cc, setCc] = useState<boolean | null>(null);
  const [touch, setTouch] = useState(false);

  useEffect(() => {
    setMuted(localStorage.getItem(MUTE_KEY) === "1");
    setTouch(matchMedia("(hover: none) and (pointer: coarse)").matches);
    const params = new URLSearchParams(location.search);
    const capture = params.has("capture");
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches || params.has("reduced");
    let cancelled = false;
    const load = () =>
      import("./show/main.js").then(({ createShow }) => {
        if (cancelled || !still.current || !canvas.current) return;
        show.current = createShow({
          still: still.current,
          canvas: canvas.current,
          base: new URL("show/", document.baseURI).href,
          capture,
          reduced,
          on: {
            phase: (p: Phase) => {
              setPhase(p);
              if (p === "ended") window.setTimeout(() => setCard(true), capture ? 0 : 700);
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
    const isMuted = localStorage.getItem(MUTE_KEY) === "1";
    if (!isMuted && !context.current) context.current = newContext();
    setCard(false);
    void s.start({ context: context.current, muted: isMuted });
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

  const toggleSound = () => {
    const next = !muted;
    setMuted(next);
    localStorage.setItem(MUTE_KEY, next ? "1" : "0");
    if (!next && !context.current) context.current = newContext();
    show.current?.setMuted(next, context.current);
  };

  const onTap = (e: React.PointerEvent) => {
    const r = canvas.current?.getBoundingClientRect();
    if (r) show.current?.tap(e.clientX - r.left, e.clientY - r.top);
  };

  const captions = cc ?? muted;
  const live = phase === "live" || phase === "loading";

  return (
    <section
      ref={section}
      data-phase={phase}
      aria-label="Spaceman, a short film"
      className="relative h-[100svh] min-h-[480px] w-full overflow-hidden bg-[#08080a] select-none"
    >
      <canvas ref={still} aria-hidden="true" className={`absolute inset-0 size-full transition-opacity duration-300 ${phase === "live" || phase === "ended" ? "opacity-0" : "opacity-100"}`} />
      <canvas ref={canvas} aria-hidden="true" onPointerDown={onTap} className={`absolute inset-0 size-full ${phase === "live" ? "opacity-100" : "opacity-0"}`} />

      <div className={`pointer-events-none absolute inset-x-0 top-0 transition-opacity duration-500 ${live ? "opacity-0" : "opacity-100"}`}>
        <div className={live ? "" : "pointer-events-auto"}>{header}</div>
      </div>

      {phase === "still" && (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-4 bg-[radial-gradient(ellipse_at_center,rgba(8,8,10,.55),rgba(8,8,10,.15)_60%,transparent)]">
          <button
            type="button"
            onClick={start}
            disabled={!ready}
            aria-label={touch ? "Play the film" : "Press Option 1 to play the film"}
            className="group flex flex-col items-center gap-3 rounded-2xl p-4 text-[#f2f1ee] focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-[#cfcac0] disabled:cursor-wait"
          >
            <span className="flex items-center gap-1.5">
              {(touch ? ["Play"] : ["⌥", "1"]).map((k) => (
                <kbd key={k} className="grid h-16 min-w-16 place-items-center rounded-[14px] border border-white/15 bg-[#1b1b1e]/90 px-4 font-sans text-[26px] font-medium shadow-[0_2px_0_rgba(255,255,255,.08)_inset,0_10px_30px_rgba(0,0,0,.5)] transition-transform duration-100 group-active:translate-y-0.5">
                  {k}
                </kbd>
              ))}
            </span>
            <span className="text-[15px] text-[#d8d5ce]">{touch ? "Tap to play" : "Press ⌥1 to play"}</span>
          </button>
          <p className="text-[13px] text-[#8d8a84]">A short film. 1:41{muted ? ", sound off" : ", with sound"}.</p>
        </div>
      )}

      {(phase === "still" || live) && (
        <div className="absolute right-4 bottom-4 flex gap-2 sm:right-6 sm:bottom-6">
          {live && (
            <button type="button" onClick={() => show.current?.skip()} className="h-9 rounded-full border border-white/10 bg-black/45 px-4 text-[13px] text-[#d8d5ce] backdrop-blur hover:bg-black/60">
              Skip
            </button>
          )}
          <button type="button" onClick={() => setCc(!captions)} aria-pressed={captions} className={`h-9 rounded-full border border-white/10 px-3 text-[12px] font-semibold tracking-wide backdrop-blur ${captions ? "bg-[#ece9e3] text-[#151517]" : "bg-black/45 text-[#d8d5ce] hover:bg-black/60"}`}>
            CC
          </button>
          <button type="button" onClick={toggleSound} aria-pressed={!muted} className="h-9 rounded-full border border-white/10 bg-black/45 px-4 text-[13px] text-[#d8d5ce] backdrop-blur hover:bg-black/60">
            {muted ? "Sound off" : "Sound on"}
          </button>
        </div>
      )}

      {captions && caption && phase === "live" && (
        <p aria-live="polite" className="pointer-events-none absolute inset-x-0 bottom-[17%] mx-auto w-fit max-w-[min(90vw,46rem)] rounded-lg bg-black/70 px-4 py-2 text-center text-[clamp(15px,2.1vw,21px)] leading-snug text-[#f4f2ee]">
          {caption.text}
        </p>
      )}

      {hint && phase === "live" && (
        <p className="pointer-events-none absolute inset-x-0 top-[12%] mx-auto w-fit rounded-full bg-black/60 px-4 py-2 text-[14px] text-[#ece9e3] backdrop-blur">
          {touch ? "Your turn. Tap the bar to switch workspaces." : "Your turn. ⌥1, ⌥2 and ⌥3 switch workspaces."}
        </p>
      )}

      {phase === "ended" && (
        <div className={`absolute inset-0 flex flex-col items-center justify-center gap-7 bg-[#08080a] px-6 text-center transition-opacity duration-500 ${card ? "opacity-100" : "opacity-0"}`}>
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
