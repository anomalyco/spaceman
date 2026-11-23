// The download button, for the visitor's platform: the zip on an Apple silicon Mac, and otherwise a
// way to get the link to one. Every state has the same height and one line under it, so switching
// after hydration doesn't move the page.
import { useState } from "react";
import { usePlatform, type Platform } from "./platform";
import { DOWNLOAD_URL, MIN_MACOS_LABEL, RELEASE_VERSION, SITE_URL } from "./release";

const BUTTON =
  "inline-flex h-11 items-center justify-center gap-2 rounded-full px-6 text-[15px] font-medium tracking-[-0.01em] whitespace-nowrap transition-[background-color,transform,opacity] duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/70";
const LIGHT = "bg-[#f3f4f6] text-[#0d0e11] shadow-[0_1px_1px_rgba(0,0,0,.35),0_10px_24px_-10px_rgba(255,255,255,.25)] hover:bg-white active:scale-[0.98]";

export function downloadNote(p: Platform): string {
  switch (p.kind) {
    case "mac":
      return `Version ${RELEASE_VERSION} · ${MIN_MACOS_LABEL} · Apple silicon`;
    case "mac-old":
      return `Needs ${MIN_MACOS_LABEL}. This Mac has macOS ${p.version}.`;
    case "mac-intel":
      return "Needs a Mac with Apple silicon.";
    case "mobile":
      return "Spaceman runs on your Mac.";
    default:
      return "Spaceman is for Macs with Apple silicon.";
  }
}

export function DownloadGlyph() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M7 1.75v7.5M3.75 6.25 7 9.5l3.25-3.25M2.25 12.25h9.5" />
    </svg>
  );
}

export function Download({ align = "start", note = true }: { align?: "start" | "center"; note?: boolean }) {
  const p = usePlatform();
  const [sent, setSent] = useState(false);
  const share = async () => {
    const data = { title: "Spaceman", text: "Spaceman, a tiling window manager for macOS", url: SITE_URL };
    try {
      if (p.kind === "mobile" && navigator.share) await navigator.share(data);
      else await navigator.clipboard.writeText(SITE_URL);
      setSent(true);
      window.setTimeout(() => setSent(false), 1800);
    } catch {
      // the share sheet was dismissed
    }
  };
  let button: React.ReactNode;
  if (p.kind === "mac" || p.kind === "mac-old") {
    button = (
      <a href={DOWNLOAD_URL} className={`${BUTTON} ${LIGHT}`}>
        <DownloadGlyph />
        Download for macOS
      </a>
    );
  } else if (p.kind === "mac-intel") {
    button = (
      <button type="button" disabled className={`${BUTTON} cursor-not-allowed bg-white/[0.06] text-white/60 shadow-[inset_0_0_0_1px_rgba(255,255,255,.1)]`}>
        <DownloadGlyph />
        Download for macOS
      </button>
    );
  } else {
    button = (
      <button type="button" onClick={share} className={`${BUTTON} ${LIGHT}`}>
        {sent ? "Link copied" : p.kind === "mobile" ? "Send yourself the link" : "Copy the link for your Mac"}
      </button>
    );
  }
  return (
    <div className={`flex flex-col gap-3 ${align === "center" ? "items-center text-center" : "items-start"}`}>
      {button}
      {note && <p className="h-5 text-[13px] leading-5 text-[#9a9ca3] tabular-nums">{downloadNote(p)}</p>}
    </div>
  );
}

// the header's small button: the zip on a Mac, the download section anywhere else
export function HeaderDownload() {
  const p = usePlatform();
  const mac = p.kind === "mac" || p.kind === "mac-old";
  return (
    <a
      href={mac ? DOWNLOAD_URL : "#download"}
      className="inline-flex h-8 items-center gap-1.5 rounded-full bg-white/[0.1] px-3.5 text-[13px] font-medium text-[#f3f4f6] shadow-[inset_0_0_0_1px_rgba(255,255,255,.1)] backdrop-blur-md backdrop-saturate-150 transition-colors hover:bg-white/[0.16] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/70"
    >
      Download
    </a>
  );
}
