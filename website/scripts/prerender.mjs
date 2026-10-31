import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { render } from "../dist-ssr/prerender.js";

const root = resolve(import.meta.dirname, "..");
const indexPath = join(root, "dist", "index.html");
const html = readFileSync(indexPath, "utf8");
const marker = '<div id="root"></div>';

if (!html.includes(marker)) {
  throw new Error(`dist/index.html is missing ${marker}`);
}

writeFileSync(indexPath, html.replace(marker, `<div id="root">${render()}</div>`));
rmSync(join(root, "dist-ssr"), { recursive: true, force: true });
