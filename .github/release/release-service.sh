#!/usr/bin/env bash
# release-service.sh publish <dir> <notes-url>: publishes Spaceman's signed
# macOS zip to the Anomaly release service (anomalyco/release
# docs/publishing.md) through its shared client, vendored as
# release-client.sh, so nothing fetched at run time executes here. The client
# trades the job's GitHub OIDC token for a one-hour publish session, uploads,
# publishes and re-downloads the file to check it. Runs only in the Release
# workflow's publish job, which has `id-token: write` and the release
# environment that anomalyco/release registry/spaceman.json requires.
#
# <dir> holds Spaceman-$SPACEMAN_VERSION-macos-arm64.zip, published as target
# macos-arm64 under the name Spaceman.zip. RELEASE is https://anoma.ly/release,
# SPACEMAN_RELEASE_CHANNEL the channel. Writes <dir>/manifest.json. A rerun of
# the channel's current version succeeds when it is exactly these files.
. "$(dirname "$0")/lib.sh"

: "${RELEASE:?Set RELEASE to the release service origin}"
: "${SPACEMAN_VERSION:?Set SPACEMAN_VERSION}"
: "${SPACEMAN_RELEASE_CHANNEL:?Set SPACEMAN_RELEASE_CHANNEL}"
[ "$RELEASE" = https://anoma.ly/release ] || die "RELEASE must be https://anoma.ly/release."
[ "${1:-}" = publish ] && [ $# -eq 3 ] || die "Usage: release-service.sh publish <dir> <notes-url>"

dir=$2
notes_url=$3
zip="$dir/Spaceman-$SPACEMAN_VERSION-macos-arm64.zip"
[ -f "$zip" ] || die "Missing $zip."
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT

jq -n --arg channel "$SPACEMAN_RELEASE_CHANNEL" --arg version "$SPACEMAN_VERSION" --arg notes_url "$notes_url" \
  '{channel: $channel, version: $version, distributions: [{kind: "direct"}]}
    + (if $notes_url == "" then {} else {notes_url: $notes_url} end)' > "$temporary/release.json"
RELEASE=$RELEASE bash "$(dirname "$0")/release-client.sh" publish --app spaceman --artifact desktop \
  --release "$temporary/release.json" --out "$dir/manifest.json" --verify-downloads \
  --target "macos-arm64=$zip=Spaceman.zip"
