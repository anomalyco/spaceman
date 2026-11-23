#!/bin/zsh
# The og:image and site stills on a HiveMind browser worker: build, shoot, leave results in captures/<name>.
# usage (from the worktree root): hivemind run --kind browser -- website/show/tools/run-stills.sh <name>
set -euo pipefail
name=${1:-stills}
root="${0:A:h}/../.."
out="$root/../captures/$name"
cd $root
bun install --silent
bun run build >/dev/null
cd show/tools
bun install --silent
bunx playwright install chromium >/dev/null
mkdir -p $out
bun stills.ts $out 2>&1 | tee $out/stills.log | grep -v '^\[pid='
ls -la $out
