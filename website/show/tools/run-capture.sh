#!/bin/zsh
# One capture round on a HiveMind browser worker: build, capture, leave results in $LEASE_OUT.
# usage (from the worktree root): hivemind run --kind browser --fetch -- website/show/tools/run-capture.sh <round> [--quick]
set -euo pipefail
round=${1:-r0}
shift || true
root="${0:A:h}/../.."
# browser jobs fetch new files under <worktree>/captures/ back by themselves
out="$root/../captures/$round"
cd $root
bun install --silent
show/tools/build-static.sh >/dev/null
cd show/tools
bun install --silent
bunx playwright install chromium >/dev/null
mkdir -p $out
bun capture.ts ../../dist $out "$@" 2>&1 | tee $out/capture.log
du -sh $out/*
