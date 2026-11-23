import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { render } from "../dist-ssr/prerender.js";

const root = resolve(import.meta.dirname, "..");
const indexPath = join(root, "dist", "index.html");
let html = readFileSync(indexPath, "utf8");
const marker = '<div id="root"></div>';

if (!html.includes(marker)) {
  throw new Error(`dist/index.html is missing ${marker}`);
}

// the one small stylesheet goes inline, so the first paint needs only the page
const sheet = /<link rel="stylesheet"[^>]*href="[^"]*assets\/([^"]+\.css)"[^>]*>/;
const css = sheet.exec(html);
if (!css) throw new Error("dist/index.html has no stylesheet to inline");
const style = readFileSync(join(root, "dist", "assets", css[1]), "utf8").replaceAll("</style", "<\\/style");
html = html.replace(css[0], () => `<style>${style}</style>`);

writeFileSync(indexPath, html.replace(marker, () => `<div id="root">${render()}</div>`));
rmSync(join(root, "dist-ssr"), { recursive: true, force: true });
