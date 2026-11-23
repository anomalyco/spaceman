#!/bin/sh
# Renders the Homebrew cask anomalyco/tap/spaceman (anomalyco/homebrew-tap,
# Casks/spaceman.rb) from packaging/homebrew/spaceman.rb, without secrets. The
# Release workflow's Homebrew job pushes it with
# .github/release/push-homebrew.sh.
#
#   release-homebrew.sh render <version> <zip-url> <sha256> <out>
#
# The zip URL is the release service's macos-arm64 file.
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
usage() {
  echo "Usage: $0 render <version> <zip-url> <sha256> <out>" >&2
  exit 1
}

case "${1:-}" in
  render)
    [ $# -eq 5 ] || usage
    version=$2 url=$3 sha256=$4 out=$5
    printf '%s\n' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-(beta|dev)\.[0-9]+)?$'
    printf '%s\n' "$sha256" | grep -Eq '^[0-9a-f]{64}$'
    printf '%s\n' "$url" | grep -Eq '^https://anoma\.ly/release/spaceman/files/[0-9a-f]{64}/Spaceman\.zip$'
    # The file is Spaceman.zip under its sha256, so the URL names no version.
    sed -e "s|@VERSION@|$version|" -e "s|@SHA256@|$sha256|" -e "s|@URL@|$url|" \
      "$root/packaging/homebrew/spaceman.rb" > "$out"
    if grep -q '@[A-Z0-9_]*@' "$out"; then
      echo "The cask has an unfilled placeholder." >&2
      exit 1
    fi
    echo "$out"
    ;;
  *) usage ;;
esac
