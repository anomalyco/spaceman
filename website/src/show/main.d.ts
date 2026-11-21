export function createShow(opts: {
  still: HTMLCanvasElement;
  canvas: HTMLCanvasElement;
  base: string;
  capture?: boolean;
  reduced?: boolean;
  on?: Record<string, (v: any) => void>;
}): unknown;
