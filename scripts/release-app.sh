#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
mode=${1:-prepare}
notary_profile=${SPACEMAN_NOTARY_KEYCHAIN_PROFILE:-spaceman-notary}
bucket=spaceman-releases
feed_url=$(/usr/bin/plutil -extract SUFeedURL raw "$root/app/Info.plist")
base_url=${feed_url%/appcast.xml}
version=$(/usr/bin/plutil -extract CFBundleShortVersionString raw "$root/app/Info.plist")
build_number=$(/usr/bin/plutil -extract CFBundleVersion raw "$root/app/Info.plist")
release_notes=${SPACEMAN_RELEASE_NOTES:-$root/changelog/$version.md}
expected_sparkle_key=$(/usr/bin/plutil -extract SUPublicEDKey raw "$root/app/Info.plist")
host_arch=$(uname -m)
arch=${SPACEMAN_RELEASE_ARCH:-$host_arch}
dist="$root/dist"
updates="$dist/updates"
artifact="Spaceman-v$version-macos-$arch.zip"
latest_artifact="Spaceman-latest-macos-$arch.zip"
checksum="$artifact.sha256"
submission="$dist/.Spaceman-v$version-macos-$arch-notarization.zip"

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
case "$arch" in
  arm64|x86_64) ;;
  *)
    echo "SPACEMAN_RELEASE_ARCH must be arm64 or x86_64, not '$arch'." >&2
    exit 1
    ;;
esac
[ "$arch" = "$host_arch" ] || {
  echo "Cross-architecture release builds are not supported: requested '$arch' on '$host_arch'." >&2
  exit 1
}
case "$build_number" in
  ''|*[!0-9]*|0)
    echo "CFBundleVersion must be a positive integer." >&2
    exit 1
    ;;
esac
[ -z "$(git -C "$root" status --porcelain)" ] || {
  echo "Release builds require a clean, reviewed Git worktree." >&2
  exit 1
}
upstream=$(git -C "$root" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null || true)
if [ -z "$upstream" ] || [ "$(git -C "$root" rev-parse HEAD)" != "$(git -C "$root" rev-parse "$upstream")" ]; then
  echo "Push the release commit to its upstream branch before releasing." >&2
  exit 1
fi

published_appcast=$(mktemp)
downloaded=$(mktemp)
trap 'rm -f "$published_appcast" "$downloaded" "$submission"' EXIT HUP INT TERM
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
xcrun notarytool history --keychain-profile "$notary_profile" >/dev/null || {
  echo "The notarytool keychain profile '$notary_profile' is missing or unusable." >&2
  exit 1
}
sparkle_dir=$("$root/scripts/setup-sparkle.sh")
actual_sparkle_key=$("$sparkle_dir/bin/generate_keys" --account ed25519 -p)
[ "$actual_sparkle_key" = "$expected_sparkle_key" ] || {
  echo "The Sparkle signing key does not match SUPublicEDKey in Info.plist." >&2
  exit 1
}

mkdir -p "$dist"
rm -rf "$updates"
rm -f "$dist/$artifact" "$dist/$checksum" "$submission"
mkdir -p "$updates"

bundle=$("$root/scripts/build-app.sh")
(cd "$(dirname "$bundle")" && /usr/bin/zip -qry --symlinks "$submission" "$(basename "$bundle")")
notary_result=$(xcrun notarytool submit "$submission" \
  --keychain-profile "$notary_profile" --wait --output-format plist)
notary_status=$(printf '%s' "$notary_result" | /usr/bin/plutil -extract status raw -)
[ "$notary_status" = Accepted ] || {
  echo "Apple notarization finished with status '$notary_status', not Accepted." >&2
  exit 1
}
xcrun stapler staple "$bundle"

(cd "$(dirname "$bundle")" && /usr/bin/zip -qry --symlinks "$dist/$artifact" "$(basename "$bundle")")
SPACEMAN_RELEASE_ARCH="$arch" "$root/scripts/validate-release.sh" "$bundle" "$dist/$artifact"
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

rm -f "$submission"
trap 'rm -f "$published_appcast" "$downloaded"' EXIT HUP INT TERM
echo "$dist/$artifact"
echo "$dist/$checksum"
echo "Prepared but not published. Run $0 publish after validation." >&2
