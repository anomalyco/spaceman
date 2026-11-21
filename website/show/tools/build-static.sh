#!/bin/zsh
# Build the site with relative paths (for the artifacts gallery and capture runs) into dist/.
set -euo pipefail
cd "${0:A:h}/../.."
bunx tsc -b
bunx vite build --base ./
bunx vite build --ssr src/prerender.tsx --outDir dist-ssr --base ./
bun scripts/prerender.mjs
