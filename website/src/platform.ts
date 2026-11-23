// Which download the visitor can use. The prerendered page assumes an Apple silicon Mac (most visitors);
// guess() runs before the first paint after hydration and refine() asks Chromium's client hints for the
// chip and the macOS version. Nothing else names the chip reliably (Safari reports every Mac as Intel),
// so without hints a Mac keeps the download. ?platform=mac|mac-intel|mac-old|windows|linux|mobile forces a state.
import { useEffect, useLayoutEffect, useState } from "react";

export type Platform =
  | { kind: "mac" }
  | { kind: "mac-old"; version: string }
  | { kind: "mac-intel" }
  | { kind: "windows" }
  | { kind: "linux" }
  | { kind: "mobile" };

export const MIN_MACOS = 14;

type HintData = {
  mobile?: boolean;
  platform?: string;
  getHighEntropyValues?: (hints: string[]) => Promise<{ architecture?: string; platformVersion?: string }>;
};

const hints = () => (navigator as Navigator & { userAgentData?: HintData }).userAgentData;

function forced(): Platform | undefined {
  const p = new URLSearchParams(location.search).get("platform");
  if (p === "mac-old") return { kind: "mac-old", version: "13" };
  if (p === "mac" || p === "mac-intel" || p === "windows" || p === "linux" || p === "mobile") return { kind: p };
  return undefined;
}

export function guess(): Platform {
  const f = forced();
  if (f) return f;
  const ua = navigator.userAgent;
  const hint = hints()?.platform ?? "";
  if (/iPhone|iPod|iPad|Android/.test(ua) || hints()?.mobile) return { kind: "mobile" };
  // iPadOS asks for the desktop site
  if (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1) return { kind: "mobile" };
  if (/Windows/.test(ua) || hint === "Windows") return { kind: "windows" };
  if (/Mac OS X|Macintosh/.test(ua) || hint === "macOS") return { kind: "mac" };
  return { kind: "linux" };
}

async function refine(p: Platform): Promise<Platform> {
  if (p.kind !== "mac" || forced()) return p;
  const data = hints();
  if (!data?.getHighEntropyValues) return p;
  try {
    const { architecture, platformVersion } = await data.getHighEntropyValues(["architecture", "platformVersion"]);
    if (architecture === "x86") return { kind: "mac-intel" };
    const major = Number(platformVersion?.split(".")[0]);
    if (major && major < MIN_MACOS) return { kind: "mac-old", version: String(major) };
  } catch {
    // no hints: keep the guess
  }
  return p;
}

const same = (a: Platform, b: Platform) => JSON.stringify(a) === JSON.stringify(b);
const useIsoLayoutEffect = typeof window === "undefined" ? useEffect : useLayoutEffect;

export function usePlatform(): Platform {
  const [p, setP] = useState<Platform>({ kind: "mac" });
  useIsoLayoutEffect(() => {
    const g = guess();
    setP((old) => (same(old, g) ? old : g));
    let live = true;
    void refine(g).then((r) => live && setP((old) => (same(old, r) ? old : r)));
    return () => {
      live = false;
    };
  }, []);
  return p;
}
