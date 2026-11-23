#!/bin/sh
# release-app.sh [prepare|publish]: puts the stable release that the Release workflow
# (.github/workflows/release.yml) built, signed, notarized and published to
# https://anoma.ly/release/spaceman into Sparkle's appcast on R2. It never builds or signs
# the app: `prepare` downloads that exact zip, checks it against the release service's
# manifest and scripts/validate-release.sh, and signs a local appcast for it with the
# Sparkle key; `publish` uploads the zip, its checksum and the latest alias to R2, then
# the appcast. Run it from the released commit (docs/releasing.md).
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
mode=${1:-prepare}
service=https://anoma.ly/release
team_id=5NZ4Q7NXJ4
bucket=spaceman-releases
feed_url=$(/usr/bin/plutil -extract SUFeedURL raw "$root/app/Info.plist")
base_url=${feed_url%/appcast.xml}
version=$(/usr/bin/plutil -extract CFBundleShortVersionString raw "$root/app/Info.plist")
build_number=$(/usr/bin/plutil -extract CFBundleVersion raw "$root/app/Info.plist")
release_notes=${SPACEMAN_RELEASE_NOTES:-$root/changelog/$version.md}
expected_sparkle_key=$(/usr/bin/plutil -extract SUPublicEDKey raw "$root/app/Info.plist")
arch=arm64
dist="$root/dist"
updates="$dist/updates"
artifact="Spaceman-v$version-macos-$arch.zip"
latest_artifact="Spaceman-latest-macos-$arch.zip"
checksum="$artifact.sha256"

verify_prepared_appcast() {
  grep -Fq "$base_url/releases/$artifact" "$updates/appcast.xml" || {
    echo "The prepared appcast does not reference $artifact." >&2
    exit 1
  }
  grep -Fq 'sparkle:edSignature=' "$updates/appcast.xml" || {
    echo "The prepared appcast has no signed update archive." >&2
    exit 1
  }
  grep -Fq '<!-- sparkle-signatures:' "$updates/appcast.xml" || {
    echo "The prepared appcast feed is not signed." >&2
    exit 1
  }
}

if [ "$mode" != prepare ] && [ "$mode" != publish ]; then
  echo "usage: $0 [prepare|publish]" >&2
  exit 2
fi
case "$build_number" in
  ''|*[!0-9]*|0)
    echo "CFBundleVersion must be a positive integer." >&2
    exit 1
    ;;
esac
[ -z "$(git -C "$root" status --porcelain)" ] || {
  echo "Release from a clean worktree of the released commit." >&2
  exit 1
}
upstream=$(git -C "$root" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null || true)
if [ -z "$upstream" ] || [ "$(git -C "$root" rev-parse HEAD)" != "$(git -C "$root" rev-parse "$upstream")" ]; then
  echo "Push the release commit to its upstream branch before releasing." >&2
  exit 1
fi
command -v jq >/dev/null 2>&1 || {
  echo "Releasing needs jq." >&2
  exit 1
}

published_appcast=$(mktemp)
downloaded=$(mktemp)
manifest=$(mktemp)
extracted=$(mktemp -d)
trap 'rm -f "$published_appcast" "$downloaded" "$manifest"; rm -rf "$extracted"' EXIT HUP INT TERM
if ! feed_status=$(curl --silent --show-error --location --output "$published_appcast" \
  --write-out '%{http_code}' "$feed_url"); then
  echo "Could not fetch the published appcast; refusing to replace release history." >&2
  exit 1
fi
case "$feed_status" in
  200)
    published_build=$(sed -n 's|.*<sparkle:version>\([0-9][0-9]*\)</sparkle:version>.*|\1|p' "$published_appcast" | head -1)
    if [ -z "$published_build" ] || [ "$build_number" -le "$published_build" ]; then
      echo "Build $build_number must be newer than published build ${published_build:-unknown}." >&2
      exit 1
    fi
    ;;
  404)
    [ "${SPACEMAN_FIRST_RELEASE:-}" = 1 ] || {
      echo "No appcast exists. Set SPACEMAN_FIRST_RELEASE=1 only for the initial release." >&2
      exit 1
    }
    rm -f "$published_appcast"
    ;;
  *)
    echo "Published appcast returned HTTP $feed_status; refusing to replace release history." >&2
    exit 1
    ;;
esac

if [ "$mode" = publish ]; then
  [ -f "$dist/$artifact" ] && [ -f "$dist/$checksum" ] && [ -f "$updates/appcast.xml" ] || {
    echo "Prepare and validate version $version before publishing it." >&2
    exit 1
  }
  verify_prepared_appcast
  (cd "$dist" && /usr/bin/shasum -a 256 -c "$checksum")
  wrangler r2 object put "$bucket/releases/$artifact" --remote \
    --file "$dist/$artifact" --content-type application/zip
  wrangler r2 object put "$bucket/releases/$checksum" --remote \
    --file "$dist/$checksum" --content-type text/plain
  wrangler r2 object put "$bucket/releases/$latest_artifact" --remote \
    --file "$dist/$artifact" --content-type application/zip
  curl --fail --silent --show-error --location "$base_url/releases/$artifact" -o "$downloaded"
  /usr/bin/cmp "$dist/$artifact" "$downloaded"
  curl --fail --silent --show-error --location "$base_url/releases/$checksum" -o "$downloaded"
  /usr/bin/cmp "$dist/$checksum" "$downloaded"
  curl --fail --silent --show-error --location "$base_url/releases/$latest_artifact" -o "$downloaded"
  /usr/bin/cmp "$dist/$artifact" "$downloaded"
  wrangler r2 object put "$bucket/appcast.xml" --remote \
    --file "$updates/appcast.xml" --content-type application/xml
  curl --fail --silent --show-error --location "$feed_url" -o "$downloaded"
  /usr/bin/cmp "$updates/appcast.xml" "$downloaded"
  echo "$base_url/releases/$artifact"
  exit 0
fi

[ -f "$release_notes" ] || {
  echo "Add release notes at $release_notes or set SPACEMAN_RELEASE_NOTES." >&2
  exit 1
}
sparkle_dir=$("$root/scripts/setup-sparkle.sh")
actual_sparkle_key=$("$sparkle_dir/bin/generate_keys" --account ed25519 -p)
[ "$actual_sparkle_key" = "$expected_sparkle_key" ] || {
  echo "The Sparkle signing key does not match SUPublicEDKey in Info.plist." >&2
  exit 1
}

# The stable channel's current release must be this version; take its macOS zip by the
# size and SHA-256 the manifest names.
curl --fail --silent --show-error --location "$service/spaceman/desktop/stable.json" -o "$manifest"
jq -e --arg version "$version" \
  '.app == "spaceman" and .artifact == "desktop" and .channel == "stable" and .release.version == $version' \
  "$manifest" >/dev/null || {
  echo "The release service's stable Spaceman is not $version; run the Release workflow (stable) first." >&2
  exit 1
}
url=$(jq -er '.release.targets["macos-arm64"].url' "$manifest")
size=$(jq -er '.release.targets["macos-arm64"].size' "$manifest")
sha256=$(jq -er '.release.targets["macos-arm64"].sha256' "$manifest")
case "$url" in
  "/release/spaceman/files/$sha256/Spaceman.zip") ;;
  *)
    echo "The release service names an unexpected file: $url" >&2
    exit 1
    ;;
esac

mkdir -p "$dist"
rm -rf "$updates"
rm -f "$dist/$artifact" "$dist/$checksum"
mkdir -p "$updates"
curl --fail --silent --show-error --location --retry 3 "${service%/release}$url" -o "$dist/$artifact"
[ "$(wc -c <"$dist/$artifact" | tr -d ' ')" = "$size" ] || {
  echo "The downloaded zip is not $size bytes." >&2
  exit 1
}
printf '%s  %s\n' "$sha256" "$dist/$artifact" | /usr/bin/shasum -a 256 -c - >/dev/null || {
  echo "The downloaded zip does not have SHA-256 $sha256." >&2
  exit 1
}

/usr/bin/ditto -x -k "$dist/$artifact" "$extracted"
SPACEMAN_RELEASE_ARCH="$arch" "$root/scripts/validate-release.sh" "$extracted/Spaceman.app" "$dist/$artifact"
/usr/bin/codesign --display --verbose=4 "$extracted/Spaceman.app" 2>&1 | grep -qx "TeamIdentifier=$team_id" || {
  echo "Spaceman.app is not signed by Anomaly's team $team_id." >&2
  exit 1
}
(cd "$dist" && /usr/bin/shasum -a 256 "$artifact" >"$checksum")
(cd "$dist" && /usr/bin/shasum -a 256 -c "$checksum")

cp "$dist/$artifact" "$updates/$artifact"
# The update prompt shows the changelog entry without its frontmatter or icon and link lines.
{
  printf '# Spaceman %s\n\n' "$version"
  awk 'NR == 1 && $0 == "---" { front = 1; next } front && $0 == "---" { front = 0; next } !front' \
    "$release_notes" | grep -v -E '^(Icon|Media|Links):'
} >"$updates/${artifact%.zip}.md"
if [ -f "$published_appcast" ]; then
  cp "$published_appcast" "$updates/appcast.xml"
fi
security find-generic-password -s 'https://sparkle-project.org' -a ed25519 -w \
  | "$sparkle_dir/bin/generate_appcast" \
    --ed-key-file - \
    --download-url-prefix "$base_url/releases/" \
    --embed-release-notes \
    --maximum-deltas 0 \
    --maximum-versions 5 \
    "$updates"
verify_prepared_appcast

echo "$dist/$artifact"
echo "$dist/$checksum"
echo "Prepared but not published. Run $0 publish after testing the zip." >&2
