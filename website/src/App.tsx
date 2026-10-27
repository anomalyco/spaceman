import {
  AnimatePresence,
  LayoutGroup,
  motion,
  useAnimationFrame,
  useReducedMotion,
  type Variants,
} from "motion/react";
import { useEffect, useId, useMemo, useRef, useState } from "react";

const RELEASE_VERSION = "0.1.5";
const DOWNLOAD_URL = `https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v${RELEASE_VERSION}-macos-arm64.zip`;

type Rect = { x: number; y: number; w: number; h: number };

const layouts: Array<Record<string, Rect>> = [
  {
    a: { x: 2.5, y: 4, w: 60, h: 92 },
    b: { x: 64.5, y: 4, w: 33, h: 44 },
    c: { x: 64.5, y: 52, w: 33, h: 44 },
  },
  {
    b: { x: 2.5, y: 4, w: 66, h: 92 },
    d: { x: 70.5, y: 4, w: 27, h: 92 },
  },
  {
    e: { x: 2.5, y: 4, w: 46.75, h: 92 },
    f: { x: 51.25, y: 4, w: 46.75, h: 92 },
  },
];

export function App() {
  const [[workspace, direction], setState] = useState<[number, number]>([0, 1]);

  const select = (next: number) =>
    setState((prev) =>
      next === prev[0] ? prev : [next, next > prev[0] ? 1 : -1],
    );

  useEffect(() => {
    const interval = window.setInterval(() => {
      setState(([current]) => [(current + 1) % layouts.length, 1]);
    }, 3200);
    return () => window.clearInterval(interval);
  }, [workspace]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      const digit = Number(event.key);
      if (Number.isInteger(digit) && digit >= 1 && digit <= layouts.length) {
        select(digit - 1);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <main className="relative flex min-h-dvh flex-col items-center justify-center gap-10 overflow-hidden bg-[#0b0c0e] px-6 py-16 text-[#f2f2f0] antialiased">
      <Starfield />

      <header className="relative flex flex-col items-center gap-5">
        <img src="/mark.svg" alt="" className="size-14 rounded-[14px] shadow-lg" />
        <div className="flex flex-col items-center gap-3 text-center">
          <h1>
            <Wordmark />
          </h1>
          <p className="text-base text-[#87888e]">
            Another fucking tiling window manager for macOS.
          </p>
        </div>
      </header>

      <WorkspaceDemo workspace={workspace} direction={direction} onSelect={select} />

      <footer className="relative flex flex-col items-center gap-3">
        <a
          href={DOWNLOAD_URL}
          className="rounded-full bg-[#f0f0ee] px-6 py-2.5 text-sm font-medium text-[#101114] transition-opacity duration-150 hover:opacity-85 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#7dd3a8]"
        >
          Download
        </a>
        <p className="text-[13px] text-[#5c5d63]">
          v{RELEASE_VERSION} · declares macOS 14+ · Apple silicon preview
        </p>
      </footer>
    </main>
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
      role="img"
      aria-label="Spaceman"
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

/* ------------------------------------------------------------------ */
/* Warp starfield: one shared 3D star state drawn to every registered  */
/* canvas (page background + the refracted copy inside the glass).     */
/* ------------------------------------------------------------------ */

type WarpStar = { x: number; y: number; z: number; r: number };

const WARP_STARS: WarpStar[] = Array.from({ length: 110 }, () => {
  const z = 0.12 + Math.random() * 0.88;
  return {
    x: (Math.random() * 2 - 1) * z,
    y: (Math.random() * 2 - 1) * z,
    z,
    r: 0.4 + Math.random() * 0.9,
  };
});

const WARP_SPEED = 0.035;
const starCanvases = new Set<HTMLCanvasElement>();

function registerStarCanvas(canvas: HTMLCanvasElement) {
  starCanvases.add(canvas);
  return () => {
    starCanvases.delete(canvas);
  };
}

function advanceWarp(dt: number) {
  for (const star of WARP_STARS) {
    star.z -= dt * WARP_SPEED;
    if (star.z <= 0.05 || Math.abs(star.x / star.z) > 1.2 || Math.abs(star.y / star.z) > 1.2) {
      star.x = Math.random() * 2 - 1;
      star.y = Math.random() * 2 - 1;
      star.z = 1;
      star.r = 0.4 + Math.random() * 0.9;
    }
  }
}

function prepareCanvas(
  canvas: HTMLCanvasElement,
  w: number,
  h: number,
): CanvasRenderingContext2D | null {
  const dpr = Math.min(window.devicePixelRatio || 1, 3);
  const width = Math.round(w * dpr);
  const height = Math.round(h * dpr);
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  return ctx;
}

function drawWarp() {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  for (const canvas of starCanvases) {
    const bounds = canvas.getBoundingClientRect();
    if (bounds.width === 0 || bounds.height === 0) continue;
    const ctx = prepareCanvas(canvas, bounds.width, bounds.height);
    if (!ctx) continue;
    const cx = vw / 2 - bounds.left;
    const cy = vh / 2 - bounds.top;
    for (const star of WARP_STARS) {
      const k = 1 / star.z;
      const px = cx + star.x * (vw / 2) * k * 0.9;
      const py = cy + star.y * (vh / 2) * k * 0.9;
      const depth = 1 - star.z;
      const size = star.r * (0.3 + depth * 1.3);
      const alpha = Math.min(1, depth * 1.5) * 0.65 + 0.05;
      if (size > 1.1) {
        // Brightest stars get a slight softening, not a halo.
        const bloom = size * 1.8;
        const glow = ctx.createRadialGradient(px, py, 0, px, py, bloom);
        glow.addColorStop(0, `rgba(236,238,244,${alpha * 0.9})`);
        glow.addColorStop(0.6, `rgba(236,238,244,${alpha * 0.2})`);
        glow.addColorStop(1, "rgba(236,238,244,0)");
        ctx.fillStyle = glow;
        ctx.beginPath();
        ctx.arc(px, py, bloom, 0, Math.PI * 2);
        ctx.fill();
      } else {
        ctx.beginPath();
        ctx.arc(px, py, size, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(232,234,240,${alpha * 0.75})`;
        ctx.fill();
      }
    }
  }
}

function Starfield() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const reduced = useReducedMotion();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    return registerStarCanvas(canvas);
  }, []);

  useAnimationFrame((_, delta) => {
    if (!reduced) advanceWarp(Math.min(delta, 64) / 1000);
    drawWarp();
  });

  return (
    <div aria-hidden="true" className="pointer-events-none absolute inset-0">
      <div className="nebula absolute inset-0" />
      <canvas ref={canvasRef} className="absolute inset-0 size-full" />
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Wall-clock ticker: app state derives from performance.now(), so     */
/* every app resumes exactly where it left off after workspace slides. */
/* ------------------------------------------------------------------ */

function useTicker(stepMs: number) {
  const [, force] = useState(0);
  useEffect(() => {
    const interval = window.setInterval(() => force((value) => value + 1), stepMs);
    return () => window.clearInterval(interval);
  }, [stepMs]);
  return Math.floor(performance.now() / stepMs);
}

/* ------------------------------------------------------------------ */
/* Workspace demo frame with liquid-glass refraction                   */
/* ------------------------------------------------------------------ */

const panelVariants: Variants = {
  enter: (direction: number) => ({ x: `${direction * 106}%` }),
  center: { x: "0%" },
  exit: (direction: number) => ({ x: `${direction * -106}%` }),
};

type Frame = { x: number; y: number; w: number; h: number; vw: number; vh: number };

function WorkspaceDemo({
  workspace,
  direction,
  onSelect,
}: {
  workspace: number;
  direction: number;
  onSelect: (workspace: number) => void;
}) {
  const frameRef = useRef<HTMLDivElement>(null);
  const copyCanvasRef = useRef<HTMLCanvasElement>(null);
  const [frame, setFrame] = useState<Frame | null>(null);
  const frameW = frame?.w ?? 0;
  const frameH = frame?.h ?? 0;
  const mapUri = useMemo(
    () => (frameW > 0 && frameH > 0 ? buildDisplacementMap(frameW, frameH) : null),
    [frameW, frameH],
  );

  useEffect(() => {
    const canvas = copyCanvasRef.current;
    if (!canvas) return;
    return registerStarCanvas(canvas);
  }, []);

  useEffect(() => {
    const element = frameRef.current;
    if (!element) return;
    const update = () => {
      const bounds = element.getBoundingClientRect();
      if (bounds.width === 0) return;
      setFrame({
        x: Math.round(bounds.left),
        y: Math.round(bounds.top),
        w: Math.round(bounds.width),
        h: Math.round(bounds.height),
        vw: window.innerWidth,
        vh: window.innerHeight,
      });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, []);

  return (
    <div className="relative flex w-full max-w-[520px] flex-col items-center gap-4">
      {frame && mapUri && <LiquidGlassFilter href={mapUri} width={frame.w} height={frame.h} />}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -inset-10 top-[-2.5rem] bg-[radial-gradient(ellipse,rgba(235,238,242,0.05),transparent_70%)] blur-2xl"
      />
      <div
        ref={frameRef}
        className="relative aspect-[16/10] w-full overflow-hidden rounded-2xl border border-white/[0.14] bg-[#0e0f12] shadow-[inset_0_1px_0_rgba(255,255,255,0.12),inset_0_-1px_0_rgba(255,255,255,0.04),0_16px_48px_rgba(0,0,0,0.5),0_0_60px_rgba(235,238,242,0.05)]"
      >
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-0"
          style={
            mapUri
              ? { filter: "url(#liquid-glass) blur(0.75px) brightness(1.2)" }
              : undefined
          }
        >
          <div
            className="nebula absolute"
            style={{
              left: frame ? -frame.x : 0,
              top: frame ? -frame.y : 0,
              width: frame?.vw ?? "100%",
              height: frame?.vh ?? "100%",
            }}
          />
          <canvas ref={copyCanvasRef} className="absolute inset-0 size-full" />
        </div>
        <AnimatePresence initial={false} custom={direction}>
          <motion.div
            key={workspace}
            custom={direction}
            variants={panelVariants}
            initial="enter"
            animate="center"
            exit="exit"
            transition={{ type: "spring", visualDuration: 0.3, bounce: 0.05 }}
            className="absolute inset-0"
          >
            {Object.entries(layouts[workspace]).map(([id, rect]) => (
              <FakeWindow key={id} id={id} rect={rect} />
            ))}
          </motion.div>
        </AnimatePresence>

        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 bg-[linear-gradient(115deg,rgba(255,255,255,0.05),transparent_40%)]"
        />
      </div>

      <div className="flex items-center gap-1" role="tablist" aria-label="Workspaces">
        {layouts.map((_, index) => (
          <button
            key={index}
            type="button"
            role="tab"
            aria-selected={index === workspace}
            aria-label={`Workspace ${index + 1}`}
            onClick={() => onSelect(index)}
            className="relative grid size-8 place-items-center rounded-md font-mono text-[13px] tabular-nums focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#7dd3a8]"
          >
            {index === workspace && (
              <motion.span
                layoutId="active-workspace"
                className="absolute inset-0 rounded-md bg-white/10"
                transition={{ type: "spring", visualDuration: 0.3, bounce: 0.1 }}
              />
            )}
            <motion.span
              animate={{ opacity: index === workspace ? 1 : 0.35 }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="relative text-white"
            >
              {index + 1}
            </motion.span>
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * Frame-sized displacement map: red encodes horizontal displacement, green
 * vertical, with a soft neutral center so only a lens band at the edge warps.
 */
function buildDisplacementMap(w: number, h: number): string {
  const canvas = document.createElement("canvas");
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  canvas.width = w * dpr;
  canvas.height = h * dpr;
  const ctx = canvas.getContext("2d");
  if (!ctx) return "";
  ctx.scale(dpr, dpr);

  ctx.fillStyle = "rgb(0,0,0)";
  ctx.fillRect(0, 0, w, h);
  ctx.globalCompositeOperation = "lighter";
  const gradientX = ctx.createLinearGradient(0, 0, w, 0);
  gradientX.addColorStop(0, "rgb(0,0,0)");
  gradientX.addColorStop(1, "rgb(255,0,0)");
  ctx.fillStyle = gradientX;
  ctx.fillRect(0, 0, w, h);
  const gradientY = ctx.createLinearGradient(0, 0, 0, h);
  gradientY.addColorStop(0, "rgb(0,0,0)");
  gradientY.addColorStop(1, "rgb(0,255,0)");
  ctx.fillStyle = gradientY;
  ctx.fillRect(0, 0, w, h);
  ctx.globalCompositeOperation = "source-over";

  const bandStart = 8;
  const bandEnd = 36;
  const steps = 16;
  ctx.fillStyle = "rgb(128,128,0)";
  for (let step = 0; step < steps; step += 1) {
    const inset = bandStart + ((bandEnd - bandStart) * step) / (steps - 1);
    ctx.globalAlpha = 0.28;
    ctx.beginPath();
    ctx.roundRect(inset, inset, w - inset * 2, h - inset * 2, 12);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
  ctx.beginPath();
  ctx.roundRect(bandEnd, bandEnd, w - bandEnd * 2, h - bandEnd * 2, 10);
  ctx.fill();

  return canvas.toDataURL("image/png");
}

function LiquidGlassFilter({
  href,
  width,
  height,
}: {
  href: string;
  width: number;
  height: number;
}) {
  return (
    <svg width="0" height="0" aria-hidden="true" className="absolute">
      <filter
        id="liquid-glass"
        x="0"
        y="0"
        width={width}
        height={height}
        filterUnits="userSpaceOnUse"
        primitiveUnits="userSpaceOnUse"
        colorInterpolationFilters="sRGB"
      >
        <feImage
          href={href}
          x="0"
          y="0"
          width={width}
          height={height}
          preserveAspectRatio="none"
          result="map"
        />
        <feDisplacementMap
          in="SourceGraphic"
          in2="map"
          scale={-72}
          xChannelSelector="R"
          yChannelSelector="G"
        />
      </filter>
    </svg>
  );
}

/* ------------------------------------------------------------------ */
/* Fake windows                                                        */
/* ------------------------------------------------------------------ */

const appNames: Record<string, string> = {
  a: "Star Map",
  b: "Mission Control",
  c: "Space Tunes",
  d: "Alien Chat",
  e: "Ship Status",
  f: "Radar",
};

function FakeWindow({ id, rect }: { id: string; rect: Rect }) {
  return (
    <div
      className="absolute flex flex-col overflow-hidden rounded-lg border border-white/[0.16] bg-gradient-to-b from-[#1a1d25]/95 to-[#14161c]/95 shadow-[0_0_0_0.5px_rgba(0,0,0,0.6),inset_0_0.5px_0_rgba(255,255,255,0.14),0_10px_30px_rgba(0,0,0,0.5)]"
      style={{
        left: `${rect.x}%`,
        top: `${rect.y}%`,
        width: `${rect.w}%`,
        height: `${rect.h}%`,
      }}
    >
      <div className="relative flex h-[26px] shrink-0 items-center border-b border-black/40 bg-white/[0.045] px-2.5 shadow-[inset_0_-0.5px_0_rgba(255,255,255,0.05)]">
        <div className="flex gap-1.5">
          <span className="size-[8px] rounded-full bg-white/20 shadow-[inset_0_0_1px_rgba(0,0,0,0.4)]" />
          <span className="size-[8px] rounded-full bg-white/20 shadow-[inset_0_0_1px_rgba(0,0,0,0.4)]" />
          <span className="size-[8px] rounded-full bg-white/20 shadow-[inset_0_0_1px_rgba(0,0,0,0.4)]" />
        </div>
        <span className="pointer-events-none absolute inset-x-0 truncate px-14 text-center text-[10px] font-medium text-white/40">
          {appNames[id]}
        </span>
      </div>
      <AppBody id={id} />
    </div>
  );
}

function AppBody({ id }: { id: string }) {
  if (id === "a") return <StarMapApp />;
  if (id === "b") return <TerminalApp />;
  if (id === "c") return <MusicApp />;
  if (id === "d") return <ChatApp />;
  if (id === "f") return <RadarApp />;
  return <StatusApp />;
}

const RADAR_PERIOD_S = 4;
const RADAR_EPOCH_S = 16;

function RadarApp() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const reduced = useReducedMotion();

  useAnimationFrame(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const bounds = canvas.getBoundingClientRect();
    if (bounds.width === 0 || bounds.height === 0) return;
    const w = bounds.width;
    const h = bounds.height;
    const ctx = prepareCanvas(canvas, w, h);
    if (!ctx) return;
    const cx = w / 2;
    const cy = h / 2;
    const radius = Math.min(w, h) * 0.38;
    const t = reduced ? 0 : performance.now() / 1000;

    // Rings and cross hairs
    ctx.lineWidth = 1;
    ctx.strokeStyle = "rgba(255,255,255,0.08)";
    for (const fraction of [1 / 3, 2 / 3, 1]) {
      ctx.beginPath();
      ctx.arc(cx, cy, radius * fraction, 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.strokeStyle = "rgba(255,255,255,0.05)";
    ctx.beginPath();
    ctx.moveTo(cx - radius, cy);
    ctx.lineTo(cx + radius, cy);
    ctx.moveTo(cx, cy - radius);
    ctx.lineTo(cx, cy + radius);
    ctx.stroke();

    // Sweep with decaying trail
    const sweep = (t / RADAR_PERIOD_S) * Math.PI * 2;
    for (let segment = 27; segment >= 0; segment -= 1) {
      const angle = sweep - segment * 0.035;
      const alpha = 0.22 * (1 - segment / 28);
      ctx.strokeStyle = `rgba(232,238,244,${alpha})`;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius);
      ctx.stroke();
    }

    // Contacts light up as the sweep passes, then fade
    const epoch = Math.floor(t / RADAR_EPOCH_S);
    for (let index = 0; index < 5; index += 1) {
      const angle = hash01(epoch * 13.7 + index * 3.1) * Math.PI * 2;
      const distance = (0.25 + hash01(epoch * 7.9 + index * 5.3) * 0.68) * radius;
      const tau = Math.PI * 2;
      const delta = (((sweep - angle) % tau) + tau) % tau;
      const alpha = Math.max(0, 1 - delta / (Math.PI * 1.2));
      if (alpha > 0.02) {
        ctx.fillStyle = `rgba(236,240,246,${alpha * 0.9})`;
        ctx.beginPath();
        ctx.arc(
          cx + Math.cos(angle) * distance,
          cy + Math.sin(angle) * distance,
          1.2 + alpha * 1.3,
          0,
          Math.PI * 2,
        );
        ctx.fill();
      }
    }
  });

  return (
    <div className="relative min-h-0 flex-1 overflow-hidden">
      <canvas ref={canvasRef} className="absolute inset-0 size-full" />
      <p className="absolute bottom-2 left-2.5 font-mono text-[8px] tracking-wider text-white/30">
        SCANNING
      </p>
      <p className="absolute bottom-2 right-2.5 font-mono text-[9px] text-white/25">
        no threats
      </p>
    </div>
  );
}

/**
 * Top-down procedural scanner: the ship holds position while parallax star
 * layers stream past. Everything derives from wall-clock time, so the motion
 * is constant and survives workspace switches.
 */
function hash01(n: number) {
  const s = Math.sin(n * 127.1) * 43758.5453;
  return s - Math.floor(s);
}

const MAP_LAYERS = [
  { count: 26, speed: 0.016, size: 0.7, alpha: 0.28, seed: 11 },
  { count: 16, speed: 0.038, size: 1.1, alpha: 0.5, seed: 57 },
  { count: 8, speed: 0.08, size: 1.5, alpha: 0.75, seed: 91 },
];
const PLANET_PERIOD_S = 16;
const MAP_HOME = { x: 0.5, y: 0.56 };
const HYPERSPACE_AFTER_S = 6;

/** Shared scroll clock so the field never resets across mounts. */
let mapScroll = 0;
/** Guards against double-advancing when two map instances overlap mid-slide. */
let mapScrollStamp = 0;

type ShipSim = {
  initialized: boolean;
  x: number;
  y: number;
  vx: number;
  vy: number;
  heading: number;
  flyTime: number;
  mode: "fly" | "align" | "jump";
  modeTime: number;
  boost: number;
};

function StarMapApp() {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const shipRef = useRef<HTMLDivElement>(null);
  const cursor = useRef<{ x: number; y: number; at: number } | null>(null);
  const sim = useRef<ShipSim>({
    initialized: false,
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    heading: -90,
    flyTime: 0,
    mode: "fly",
    modeTime: 0,
    boost: 0,
  });
  const reduced = useReducedMotion();

  useAnimationFrame((_, delta) => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    const ship = shipRef.current;
    if (!canvas || !container || !ship) return;
    const bounds = container.getBoundingClientRect();
    if (bounds.width === 0 || bounds.height === 0) return;
    const dt = Math.min(delta, 64) / 1000;
    const w = bounds.width;
    const h = bounds.height;
    const now = performance.now();
    const t = now / 1000;
    const s = sim.current;

    if (!s.initialized) {
      s.x = MAP_HOME.x * w;
      s.y = MAP_HOME.y * h;
      s.initialized = true;
    }

    const fresh = cursor.current !== null && now - cursor.current.at < 2500;

    /* --- state machine: fly → align → jump → fly --- */
    if (reduced) {
      s.mode = "fly";
      s.boost = 0;
      s.flyTime = 0;
    } else if (s.mode === "fly") {
      if (fresh) s.flyTime += dt;
      if (s.flyTime > HYPERSPACE_AFTER_S) {
        s.mode = "align";
        s.modeTime = 0;
      }
    } else if (s.mode === "align") {
      s.modeTime += dt;
      if (s.modeTime > 0.9) {
        s.mode = "jump";
        s.modeTime = 0;
      }
    } else {
      s.modeTime += dt;
      s.boost =
        s.modeTime < 1.3
          ? Math.min(1, s.modeTime / 0.5)
          : Math.max(0, 1 - (s.modeTime - 1.3) / 0.5);
      if (s.modeTime > 1.9) {
        s.mode = "fly";
        s.flyTime = 0;
        s.boost = 0;
        cursor.current = null;
      }
    }

    /* --- steering --- */
    let targetX = MAP_HOME.x * w + Math.sin(t * 0.5) * 10;
    let targetY = MAP_HOME.y * h + Math.sin(t * 0.8 + 2) * 6;
    if (s.mode !== "fly") {
      targetX = MAP_HOME.x * w;
      targetY = MAP_HOME.y * h;
    } else if (fresh && cursor.current) {
      targetX = cursor.current.x;
      targetY = cursor.current.y;
    }

    if (!reduced) {
      const maxSpeed = s.mode === "fly" && fresh ? 95 : 55;
      const dx = targetX - s.x;
      const dy = targetY - s.y;
      const dist = Math.hypot(dx, dy) || 1;
      const speedTarget = maxSpeed * Math.min(1, dist / 70);
      const blend = Math.min(1, dt * 3);
      s.vx += ((dx / dist) * speedTarget - s.vx) * blend;
      s.vy += ((dy / dist) * speedTarget - s.vy) * blend;
      s.x = Math.max(10, Math.min(w - 10, s.x + s.vx * dt));
      s.y = Math.max(12, Math.min(h - 12, s.y + s.vy * dt));

      let desired = s.heading;
      if (s.mode !== "fly") desired = -90;
      else if (Math.hypot(s.vx, s.vy) > 6) {
        desired = (Math.atan2(s.vy, s.vx) * 180) / Math.PI;
      }
      let diff = ((desired - s.heading + 540) % 360) - 180;
      s.heading += diff * Math.min(1, dt * 6);
    }

    const jitter = s.mode === "align" ? Math.min(1.6, s.modeTime * 2) : 0;
    const jitterX = (Math.random() - 0.5) * 2 * jitter;
    const jitterY = (Math.random() - 0.5) * 2 * jitter;
    const lift = s.mode === "jump" ? s.boost * 8 : 0;
    ship.style.transform = `translate(${s.x + jitterX}px, ${s.y + jitterY - lift}px) translate(-50%, -50%) rotate(${s.heading + 90}deg)`;

    /* --- procedural field --- */
    if (!reduced && now - mapScrollStamp > 4) {
      mapScroll += dt * (1 + s.boost * 45);
      mapScrollStamp = now;
    }
    const ctx = prepareCanvas(canvas, w, h);
    if (!ctx) return;

    for (const layer of MAP_LAYERS) {
      const streak = s.boost * layer.speed * 900;
      for (let index = 0; index < layer.count; index += 1) {
        const x = hash01(layer.seed + index * 71.7) * w;
        const y =
          ((hash01(layer.seed + index * 133.3 + 1) + mapScroll * layer.speed) % 1) * h;
        if (streak > 2) {
          ctx.strokeStyle = `rgba(232,234,240,${Math.min(0.9, layer.alpha + s.boost * 0.25)})`;
          ctx.lineWidth = layer.size;
          ctx.beginPath();
          ctx.moveTo(x, y);
          ctx.lineTo(x, y - streak);
          ctx.stroke();
        } else {
          ctx.fillStyle = `rgba(232,234,240,${layer.alpha})`;
          ctx.beginPath();
          ctx.arc(x, y, layer.size, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }

    // A ringed planet drifts through on the shared scroll clock.
    const epoch = Math.floor(mapScroll / PLANET_PERIOD_S);
    const phase = (mapScroll / PLANET_PERIOD_S) % 1;
    const px = (0.18 + hash01(epoch * 7.31) * 0.64) * w;
    const py = (phase * 1.5 - 0.25) * h;
    const pr = 7 + hash01(epoch * 3.17) * 9;
    if (py > -pr * 3 && py < h + pr * 3 && s.boost < 0.4) {
      ctx.lineWidth = 1.1;
      ctx.fillStyle = "rgba(235,238,244,0.05)";
      ctx.strokeStyle = "rgba(235,238,244,0.55)";
      ctx.beginPath();
      ctx.arc(px, py, pr, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
      ctx.strokeStyle = "rgba(235,238,244,0.28)";
      ctx.beginPath();
      ctx.ellipse(px, py, pr * 1.75, pr * 0.5, -0.3, 0, Math.PI * 2);
      ctx.stroke();
    }

    if (s.boost > 0) {
      ctx.fillStyle = `rgba(240,244,255,${s.boost * 0.05})`;
      ctx.fillRect(0, 0, w, h);
    }
  });

  return (
    <div
      ref={containerRef}
      className="relative min-h-0 flex-1 cursor-crosshair overflow-hidden"
      onPointerMove={(event) => {
        const bounds = containerRef.current?.getBoundingClientRect();
        if (!bounds) return;
        cursor.current = {
          x: event.clientX - bounds.left,
          y: event.clientY - bounds.top,
          at: performance.now(),
        };
      }}
    >
      <svg
        className="absolute inset-0 size-full"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        <path
          d="M 25 0 V 100 M 50 0 V 100 M 75 0 V 100 M 0 25 H 100 M 0 50 H 100 M 0 75 H 100"
          stroke="rgba(255,255,255,0.045)"
          strokeWidth="0.5"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <canvas ref={canvasRef} className="absolute inset-0 size-full" />
      <div
        ref={shipRef}
        className="absolute left-0 top-0 will-change-transform"
        style={{ transform: "translate(-9999px, -9999px)" }}
        aria-hidden="true"
      >
        <svg width="14" height="18" viewBox="0 0 14 18">
          <path
            d="M7 1 L12.5 16 L7 12.6 L1.5 16 Z"
            fill="none"
            stroke="rgba(240,242,248,0.9)"
            strokeWidth="1.2"
            strokeLinejoin="round"
          />
        </svg>
      </div>
      <p className="absolute bottom-2 left-2.5 font-mono text-[8px] tracking-wider text-white/30">
        YOU ARE HERE
      </p>
      <p className="absolute bottom-2 right-2.5 font-mono text-[9px] text-white/25">
        4.2 ly to Kepler-442b
      </p>
    </div>
  );
}

const termScript = [
  { cmd: "spaceman ctl focus right", out: "focus queued" },
  { cmd: "spaceman ctl move 4", out: "move to workspace 4 queued" },
  { cmd: "open pod-bay-doors", out: "I'm afraid I can't do that" },
  { cmd: "spaceman ctl balance", out: "balance queued" },
];
const CMD_PAUSE = 8;
const OUT_PAUSE = 20;
const termDurations = termScript.map((item) => item.cmd.length + CMD_PAUSE + OUT_PAUSE);
const termTotal = termDurations.reduce((sum, ticks) => sum + ticks, 0);

function TerminalApp() {
  const tick = useTicker(70);

  let remaining = tick % termTotal;
  let index = 0;
  while (remaining >= termDurations[index]) {
    remaining -= termDurations[index];
    index += 1;
  }
  const current = termScript[index];
  const typed = current.cmd.slice(0, Math.min(remaining, current.cmd.length));
  const showOut = remaining >= current.cmd.length + CMD_PAUSE;
  const history = termScript.slice(Math.max(0, index - 2), index);

  return (
    <div className="flex-1 space-y-1 overflow-hidden p-2.5 font-mono text-[10px] leading-relaxed">
      {history.map((item) => (
        <div key={item.cmd} className="space-y-1">
          <p>
            <span className="text-[#7dd3a8]/80">❯</span>{" "}
            <span className="text-white/55">{item.cmd}</span>
          </p>
          <p className="text-white/25">{item.out}</p>
        </div>
      ))}
      <p>
        <span className="text-[#7dd3a8]/80">❯</span>{" "}
        <span className="text-white/55">{typed}</span>
        <span className="terminal-cursor ml-px inline-block h-2.5 w-[7px] translate-y-px bg-white/50" />
      </p>
      {showOut && <p className="text-white/25">{current.out}</p>}
    </div>
  );
}

const songs = [
  { title: "An Ending (Ascent)", artist: "Brian Eno" },
  { title: "Olson", artist: "Boards of Canada" },
  { title: "Avril 14th", artist: "Aphex Twin" },
];
const SONG_MS = 12000;

function MusicApp() {
  const tick = useTicker(250);
  const nowMs = tick * 250;
  const songIndex = Math.floor(nowMs / SONG_MS);
  const track = songs[songIndex % songs.length];
  const progress = ((nowMs % SONG_MS) / SONG_MS) * 100;

  return (
    <div className="flex flex-1 items-center gap-3 overflow-hidden p-3">
      <div className="relative size-12 shrink-0">
        <motion.div
          className="size-12 rounded-full bg-[radial-gradient(circle_at_35%_30%,rgba(214,220,235,0.7),rgba(140,148,175,0.45)_55%,rgba(72,78,98,0.4)_85%)]"
          animate={{ rotate: 360 }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
        />
        <div className="absolute left-1/2 top-1/2 h-5 w-[70px] -translate-x-1/2 -translate-y-1/2 -rotate-[20deg] rounded-[50%] border border-white/20" />
      </div>
      <div className="min-w-0 flex-1 space-y-1">
        <motion.div
          key={songIndex}
          initial={{ opacity: 0, y: 4 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3 }}
        >
          <p className="truncate text-[10px] font-medium text-white/60">{track.title}</p>
          <p className="truncate text-[9px] text-white/30">{track.artist}</p>
        </motion.div>
        <div className="mt-1.5 flex items-center gap-2">
          <div className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-white/10">
            <div
              className="h-full rounded-full bg-white/40 transition-[width] duration-300 ease-linear"
              style={{ width: `${progress}%` }}
            />
          </div>
          <div className="flex h-3 shrink-0 items-end gap-[2px]" aria-hidden="true">
            {[0.5, 0.9, 0.65, 1, 0.75].map((peak, bar) => (
              <motion.span
                key={bar}
                className="w-[2px] origin-bottom rounded-sm bg-white/40"
                style={{ height: 12 }}
                animate={{ scaleY: [0.25, peak, 0.4, peak * 0.8, 0.25] }}
                transition={{
                  duration: 0.9 + bar * 0.17,
                  repeat: Infinity,
                  ease: "easeInOut",
                }}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

const chatScript = [
  { self: false, text: "approaching kepler-442b" },
  { self: true, text: "eta?" },
  { self: false, text: "eventually" },
  { self: true, text: "cool" },
  { self: false, text: "windows still tiled?" },
  { self: true, text: "always" },
];

const chatSpring = {
  type: "spring",
  visualDuration: 0.35,
  bounce: 0.15,
} as const;

function ChatApp() {
  const groupId = useId();
  const tick = useTicker(1100);
  const delivered = Math.floor(tick / 2);
  const typing = tick % 2 === 1;
  const next = chatScript[delivered % chatScript.length];

  // Messages that already existed when this instance mounted render settled;
  // only genuinely new arrivals animate in.
  const mountDelivered = useRef<number | null>(null);
  if (mountDelivered.current === null) mountDelivered.current = delivered;
  const isNew = (key: number) => key >= (mountDelivered.current ?? 0);

  // Keep extra history above the clip so drop-off never shifts the layout.
  const visible = [];
  for (let index = Math.max(0, delivered - 5); index < delivered; index += 1) {
    visible.push({ key: index, ...chatScript[index % chatScript.length] });
  }

  return (
    <LayoutGroup id={groupId}>
      <div className="flex flex-1 flex-col justify-end gap-1.5 overflow-hidden p-2.5 text-[10px] leading-tight">
        {visible.map((message) => (
          <motion.p
            layout
            layoutId={`chat-${message.key}`}
            key={message.key}
            initial={
              isNew(message.key) ? { opacity: 0, y: 8, scale: 0.95 } : false
            }
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ ...chatSpring, opacity: { duration: 0.25 } }}
            className={`max-w-[85%] rounded-lg px-2 py-1 ${
              message.self
                ? "self-end rounded-br-sm bg-white/[0.15] text-white/75"
                : "self-start rounded-bl-sm bg-white/[0.07] text-white/55"
            }`}
          >
            {message.text}
          </motion.p>
        ))}
        {typing && (
          <motion.div
            layout
            layoutId={`chat-${delivered}`}
            key={`typing-${delivered}`}
            initial={
              isNew(delivered) ? { opacity: 0, y: 8, scale: 0.9 } : false
            }
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ ...chatSpring, opacity: { duration: 0.2 } }}
            className={`flex gap-[3px] rounded-lg px-2 py-1.5 ${
              next.self
                ? "self-end rounded-br-sm bg-white/[0.15]"
                : "self-start rounded-bl-sm bg-white/[0.07]"
            }`}
          >
            {[0, 1, 2].map((dot) => (
              <span
                key={dot}
                className="size-1 animate-pulse rounded-full bg-white/50"
                style={{ animationDelay: `${dot * 0.2}s` }}
              />
            ))}
          </motion.div>
        )}
      </div>
    </LayoutGroup>
  );
}

function StatusApp() {
  const tick = useTicker(1500);
  const snacks = 99 - (tick % 100);

  const gauges = [
    ["OXYGEN", 98, "bg-white/35"],
    ["FUEL", 61, "bg-white/35"],
    ["GRAVITY", 100, "bg-white/35"],
    ["SNACKS", snacks, snacks < 15 ? "bg-white/50 animate-pulse" : "bg-white/35"],
  ] as const;

  return (
    <div className="flex min-h-0 flex-1 flex-col justify-center gap-2 overflow-hidden p-3 font-mono text-[9px]">
      {gauges.map(([label, value, tone]) => (
        <div key={label} className="space-y-1">
          <div className="flex justify-between text-white/35">
            <span>{label}</span>
            <span>{value}%</span>
          </div>
          <div className="h-1.5 rounded-full bg-white/[0.07]">
            <div
              className={`h-full rounded-full transition-[width] duration-1000 ease-linear ${tone}`}
              style={{ width: `${value}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}
