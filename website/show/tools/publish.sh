#!/bin/zsh
# Publish dist/ to the artifacts gallery and check it is served.
set -euo pipefail
cd "${0:A:h}/../.."
out=~/code/home/artifacts/2026-11-21-spaceman-site
mkdir -p $out
rsync -a --delete --exclude .opencode-session --exclude review dist/ $out/
cp ~/code/home/artifacts/2026-11-20-spaceman-hud-track/.opencode-session $out/.opencode-session
chmod -R a+rX $out
url=https://studio.taila8e7c7.ts.net:8444/2026-11-21-spaceman-site
for p in index.html show/audio/manifest.json show/audio/sfx-room.mp3; do
  print -- "$(curl -s -o /dev/null -w '%{http_code} %{size_download}' $url/$p) $p"
done
