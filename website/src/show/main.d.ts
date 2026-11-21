export function createShow(opts: {
  still?: HTMLCanvasElement | null;
  canvas: HTMLCanvasElement;
  base: string;
  capture?: boolean;
  reduced?: boolean;
  on?: Record<string, (v: any) => void>;
}): unknown;
