#!/bin/sh
# Bring a capture round back from the worker it ran on and clear it there.
# usage: fetch.sh <round> [worker]
set -e
round=$1; worker=${2:-pro}
src="/Users/kit/mac-remote.noindex/src/site-show-2d4d9103/captures/$round"
dest="/private/tmp/opencode/spaceman-cap/$round"
rm -rf "$dest"; mkdir -p "$dest"
scp -qr "$worker:$src/*" "$dest/"
ssh "$worker" "rm -rf '$src'"
rm -rf "$(dirname "$0")/../../../captures/$round"
ls "$dest"
