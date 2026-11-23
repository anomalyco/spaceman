import { readFileSync } from "node:fs";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import { DOWNLOAD_URL, RELEASE_VERSION, SITE_URL } from "./src/release.ts";

// The agent files that come from elsewhere: the JSON-LD, from the release the page offers, and
// /skill.md, the repository's setup skill as it is.
function forAgents(): Plugin {
  const skill = () => readFileSync(new URL("../skills/spaceman-setup/SKILL.md", import.meta.url), "utf8");
  const jsonLd = JSON.stringify({
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "Spaceman",
    description: "A tiling window manager for macOS. Every window side by side, nothing on top of anything else.",
    url: SITE_URL,
    image: `${SITE_URL}og-image.png`,
    applicationCategory: "UtilitiesApplication",
    operatingSystem: "macOS 14 or later (Apple silicon)",
    softwareVersion: RELEASE_VERSION,
    downloadUrl: DOWNLOAD_URL,
    offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
    author: { "@type": "Person", name: "Kit Langton" },
    publisher: { "@type": "Organization", name: "Anomaly", url: "https://anoma.ly" },
  });
  return {
    name: "agent-files",
    transformIndexHtml: () => [{ tag: "script", attrs: { type: "application/ld+json" }, children: jsonLd, injectTo: "head" }],
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (!req.url?.split("?")[0].endsWith("/skill.md")) return next();
        res.setHeader("Content-Type", "text/markdown; charset=utf-8");
        res.end(skill());
      });
    },
    generateBundle() {
      if (this.environment?.config.build.ssr) return;
      this.emitFile({ type: "asset", fileName: "skill.md", source: skill() });
    },
  };
}

// Production lives at https://anoma.ly/spaceman/ (worker.ts). The gallery and capture builds pass
// --base ./ (show/tools/build-static.sh); every runtime asset URL is relative, so both work.
export default defineConfig({
  base: "/spaceman/",
  plugins: [react(), tailwindcss(), forAgents()],
});
