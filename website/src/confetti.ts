// The end card's confetti: two cannons at the bottom corners fire once, the paper tumbles and drifts
// down. Every piece is a closed-form function of the time since the burst (linear air drag, gravity,
// a sway and a flip), so a frame is the same whether it's played live or stepped by the export.

type Piece = { side: number; birth: number; vx: number; vy: number; w: number; h: number; color: string; a0: number; spin: number; flip: number; f0: number; sway: number; sw: number; s0: number; round: boolean };

const COLORS = ["#ff5f57", "#febc2e", "#28c840", "#0a84ff", "#bf5af2", "#ff9f0a", "#64d2ff", "#ff375f", "#f5f5f7"];
const K = 1.9; // air drag, 1/s: the burst slows fast and the paper sinks at g/K
export const CONFETTI_SECS = 7.5;

function rng(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let x = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    x = (x + Math.imul(x ^ (x >>> 7), 61 | x)) ^ x;
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
}

const PIECES: Piece[] = (() => {
  const r = rng(20261122);
  const out: Piece[] = [];
  for (let i = 0; i < 220; i++) {
    const side = i % 2 ? 1 : -1;
    const long = r() < 0.7;
    out.push({
      side,
      // the cannon fires over a tenth of a second, the first pieces first
      birth: r() * r() * 0.12,
      // speeds in units of the card's height (vy) and width (vx): up and inward
      vx: 0.3 + r() * 0.75,
      vy: 0.85 + r() * 0.85,
      w: long ? 7 + r() * 5 : 6 + r() * 3,
      h: long ? 3 + r() * 1.6 : 6 + r() * 3,
      color: COLORS[Math.floor(r() * COLORS.length)],
      a0: r() * Math.PI * 2,
      spin: (r() - 0.5) * 14,
      flip: 5 + r() * 9,
      f0: r() * Math.PI * 2,
      sway: 6 + r() * 16,
      sw: 2 + r() * 3.5,
      s0: r() * Math.PI * 2,
      round: r() < 0.12,
    });
  }
  return out;
})();

// draws the burst at t seconds after it fired; w, h in CSS pixels (the context is already scaled)
export function drawConfetti(ctx: CanvasRenderingContext2D, t: number, w: number, h: number) {
  ctx.clearRect(0, 0, w, h);
  if (t < 0 || t > CONFETTI_SECS) return;
  const g = 0.34 * h; // so the paper settles to about a third of the card's height a second
  const fade = Math.min(1, (CONFETTI_SECS - t) / 0.8);
  for (const p of PIECES) {
    const s = t - p.birth;
    if (s < 0) continue;
    const e = Math.exp(-K * s);
    const x0 = p.side < 0 ? w * 0.04 : w * 0.96;
    const y0 = h + 12;
    const vx = -p.side * p.vx * w;
    const vy = -p.vy * h;
    // x: the shot's drift plus a sway that grows as the paper slows; y: up, slowing, then sinking
    const x = x0 + (vx / K) * (1 - e) + p.sway * Math.sin(p.sw * s + p.s0) * (1 - e);
    const y = y0 + (vy / K) * (1 - e) + (g / K) * s - (g / (K * K)) * (1 - e);
    if (y > h + 30 || x < -30 || x > w + 30) continue;
    const flip = Math.cos(p.flip * s + p.f0);
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(p.a0 + p.spin * s * (0.4 + 0.6 * e));
    ctx.scale(1, Math.max(0.08, Math.abs(flip)));
    ctx.globalAlpha = fade;
    ctx.fillStyle = p.color;
    if (p.round) {
      ctx.beginPath();
      ctx.arc(0, 0, p.w * 0.45, 0, Math.PI * 2);
      ctx.fill();
    } else ctx.fillRect(-p.w / 2, -p.h / 2, p.w, p.h);
    // the back of the paper, turned away from the light
    if (flip < 0) {
      ctx.globalAlpha = fade * 0.35 * -flip;
      ctx.fillStyle = "#000";
      if (p.round) ctx.fill();
      else ctx.fillRect(-p.w / 2, -p.h / 2, p.w, p.h);
    }
    ctx.restore();
  }
}
