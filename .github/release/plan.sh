#!/usr/bin/env bash
# plan.sh: decides what this run builds and where it publishes, from the
# event, the ref, Cargo.toml and the v* tags alone, and writes the plan to
# $GITHUB_OUTPUT.
#
#   dev    a push to dev (or a dispatch from dev): published on the release
#          service's dev channel as 0.0.0-dev-<run number>, the way Hex and
#          OpenCode number dev builds.
#   check  a dispatch from dev: build, sign, notarize and verify; publish
#          nothing. The last job pushes and deletes a throwaway
#          v0.0.0-test-<run> tag to prove it can tag.
#   stable a dispatch from dev: Cargo.toml's version on the stable channel. Needs
#          changelog/<version>.md and no v<version> tag yet; the workflow tags
#          v<version> only after it published.
#
# The manifest's version is display text: installs follow the channel and never
# compare versions. Spaceman.app itself always carries Cargo.toml's version
# (app/Info.plist), which sign-macos.sh checks as bundle-version.
#
# A run releases the commit it runs on: SHA (the checked-out commit) must be
# GITHUB_SHA, so every job can check out github.sha.
#
# Environment: EVENT, REF_NAME, CHANNEL, SHA, GITHUB_SHA, GITHUB_RUN_NUMBER,
# GITHUB_REPOSITORY, GH_TOKEN, and Cargo.toml and changelog/ of the commit.
. "$(dirname "$0")/lib.sh"

[ -n "$SHA" ] && [ "$SHA" = "${GITHUB_SHA:-}" ] || die "The checked-out commit $SHA is not this run's commit ${GITHUB_SHA:-}."
[ "$REF_NAME" = dev ] || die "Releases run from dev, not $REF_NAME."

crate=$(awk -F ' *= *' '
  /^\[package\]$/ { package = 1; next }
  /^\[/ { package = 0 }
  package && $1 == "version" { gsub(/"/, "", $2); print $2; exit }
' Cargo.toml)
[[ "$crate" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "Cargo.toml's version must look like 1.2.3, not '$crate'."

tag=""
notes=""
case "$CHANNEL" in
  stable)
    [ "$EVENT" = workflow_dispatch ] || die "Stable releases are dispatched."
    version=$crate
    tag="v$version"
    refs=$(gh api "repos/$GITHUB_REPOSITORY/git/matching-refs/tags/$tag" --jq '.[].ref')
    if grep -Fxq "refs/tags/$tag" <<< "$refs"; then die "Spaceman $version is already tagged $tag; bump Cargo.toml and app/Info.plist."; fi
    [ -s "changelog/$version.md" ] || die "Add release notes at changelog/$version.md first."
    release_channel=stable
    ;;
  dev | check)
    [ "$CHANNEL" = dev ] || [ "$EVENT" = workflow_dispatch ] || die "Check runs are dispatched."
    version="0.0.0-dev-$GITHUB_RUN_NUMBER"
    release_channel=dev
    if [ "$CHANNEL" = check ]; then tag="v0.0.0-test-$GITHUB_RUN_NUMBER"; fi
    ;;
  *) die "Unknown channel $CHANNEL." ;;
esac
{
  echo "version=$version"
  echo "bundle-version=$crate"
  echo "channel=$CHANNEL"
  echo "release-channel=$release_channel"
  echo "service=https://anoma.ly/release"
  echo "tag=$tag"
  echo "notes-url=$notes"
} >> "$GITHUB_OUTPUT"
echo "Releasing Spaceman $version ($CHANNEL, bundle $crate) from $SHA." >> "$GITHUB_STEP_SUMMARY"
