#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bundle="$root/target/Spaceman.app"
icon_output="$root/target/AppIcon.assets"
icon_info="$root/target/AppIcon-info.plist"
sparkle_dir=$("$root/scripts/setup-sparkle.sh")
identity=${SPACEMAN_CODESIGN_IDENTITY:-}
deployment_target=$(/usr/bin/plutil -extract LSMinimumSystemVersion raw "$root/app/Info.plist")
sdk_version=$(/usr/bin/xcrun --sdk macosx --show-sdk-version)
cargo_bin=${CARGO:-}

if [ -z "$cargo_bin" ]; then
  cargo_bin=$(command -v cargo 2>/dev/null || true)
fi
if [ -z "$cargo_bin" ] && [ -x "$HOME/.cargo/bin/cargo" ]; then
  cargo_bin="$HOME/.cargo/bin/cargo"
fi
if [ -z "$cargo_bin" ] || ! command -v "$cargo_bin" >/dev/null 2>&1; then
  echo "cargo was not found; install Rust 1.100.0 or set CARGO to its absolute path." >&2
  exit 1
fi

if [ -z "$identity" ]; then
  identity=$(security find-identity -v -p codesigning \
    | sed -n 's/.*"\(Developer ID Application:[^"]*\)".*/\1/p' \
    | head -1)
fi
if [ -z "$identity" ]; then
  echo "No Developer ID Application identity found; set SPACEMAN_CODESIGN_IDENTITY." >&2
  exit 1
fi
case "$identity" in
  "Developer ID Application:"*) ;;
  *)
    echo "SPACEMAN_CODESIGN_IDENTITY must be a Developer ID Application identity, not '$identity'." >&2
    exit 1
    ;;
esac

CARGO_TARGET_DIR="$root/target" \
  "$cargo_bin" build --manifest-path "$root/Cargo.toml" --release --locked --workspace --bins
# A global MACOSX_DEPLOYMENT_TARGET also applies to host-side proc-macro
# dylibs, which must remain loadable by the build host. Set the deployment
# command only on the final app executables before they are signed.
for executable in spaceman-agent spaceman; do
  path="$root/target/release/$executable"
  patched="$path.deployment-target"
  /usr/bin/vtool -set-build-version macos "$deployment_target" "$sdk_version" \
    -replace -output "$patched" "$path"
  /bin/chmod 755 "$patched"
  /bin/mv "$patched" "$path"
done
rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources" "$bundle/Contents/Frameworks"
cp "$root/target/release/spaceman-agent" "$bundle/Contents/MacOS/spaceman-agent"
cp "$root/target/release/spaceman" "$bundle/Contents/MacOS/spaceman"
cp "$root/app/Info.plist" "$bundle/Contents/Info.plist"
rm -rf "$icon_output"
mkdir -p "$icon_output"
xcrun actool "$root/app/AppIcon.icon" \
  --compile "$icon_output" \
  --platform macosx \
  --minimum-deployment-target "$deployment_target" \
  --app-icon AppIcon \
  --standalone-icon-behavior all \
  --output-partial-info-plist "$icon_info" \
  --output-format human-readable-text \
  --warnings \
  --notices >&2
cp "$icon_output/AppIcon.icns" "$icon_output/Assets.car" "$bundle/Contents/Resources/"
/usr/bin/ditto "$sparkle_dir/Sparkle.framework" "$bundle/Contents/Frameworks/Sparkle.framework"
plutil -lint "$bundle/Contents/Info.plist" >/dev/null
sparkle="$bundle/Contents/Frameworks/Sparkle.framework/Versions/B"
codesign --force --timestamp --options runtime --sign "$identity" \
  "$sparkle/XPCServices/Installer.xpc"
codesign --force --timestamp --options runtime --preserve-metadata=entitlements --sign "$identity" \
  "$sparkle/XPCServices/Downloader.xpc"
codesign --force --timestamp --options runtime --sign "$identity" "$sparkle/Autoupdate"
codesign --force --timestamp --options runtime --sign "$identity" "$sparkle/Updater.app"
codesign --force --timestamp --options runtime --sign "$identity" \
  "$bundle/Contents/Frameworks/Sparkle.framework"
codesign --force --timestamp --options runtime --sign "$identity" \
  "$bundle/Contents/MacOS/spaceman-agent"
codesign --force --timestamp --options runtime --sign "$identity" \
  "$bundle/Contents/MacOS/spaceman"
codesign --force --timestamp --options runtime --sign "$identity" \
  --identifier com.kitlangton.spaceman \
  "$bundle"
codesign --verify --deep --strict "$bundle"

printf '%s\n' "$bundle"
