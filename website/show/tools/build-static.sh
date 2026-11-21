#!/bin/zsh
# Build the site with relative paths (for the artifacts gallery and capture runs) into dist/.
set -euo pipefail
cd "${0:A:h}/../.."
# a one-line stamp in the footer while the film is in review: build-static.sh "what's in this build"
export VITE_BUILD_STAMP="${1:+$(TZ=America/Los_Angeles date '+%b %-d, %-I:%M %p') · $1}"
bunx tsc -b
bunx vite build --base ./
bunx vite build --ssr src/prerender.tsx --outDir dist-ssr --base ./
bun scripts/prerender.mjs
