#!/bin/sh
set -eu

# Usage: scripts/install-app.sh [--bundle PATH] [--wait-idle SECS]
#   --bundle     install an already built Pane.app instead of building one
#   --wait-idle  hold the restart until the keyboard and mouse have been idle for SECS seconds,
#                because restarting restores and re-tiles every window
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source_bundle=
wait_idle=
while [ $# -gt 0 ]; do
  case "$1" in
    --bundle) source_bundle=$2; shift 2 ;;
    --wait-idle) wait_idle=$2; shift 2 ;;
    *) echo "usage: $0 [--bundle PATH] [--wait-idle SECS]" >&2; exit 2 ;;
  esac
done
# The HiveMind cargo wrapper prints build output on stdout; the bundle path is the last line.
[ -n "$source_bundle" ] || source_bundle=$("$root/scripts/build-app.sh" | tail -n 1)
[ -d "$source_bundle/Contents" ] || { echo "no Pane.app at $source_bundle" >&2; exit 1; }
if [ -n "$wait_idle" ]; then
  idle_seconds() { ioreg -c IOHIDSystem | awk '/HIDIdleTime/ { print int($NF / 1000000000); exit }'; }
  while [ "$(idle_seconds)" -lt "$wait_idle" ]; do sleep 5; done
fi
install_root="/Applications"
installed_bundle="$install_root/Pane.app"
legacy_bundle="$install_root/Spaceman.app"
legacy_support="$HOME/Library/Application Support/Spaceman"
cli_root="$HOME/.local/bin"
was_automatic=false

mkdir -p "$install_root"
if pgrep -x spaceman-agent >/dev/null 2>&1; then
  cli="$source_bundle/Contents/MacOS/spaceman"
  ctl_support=
  status=$("$cli" ctl status 2>/dev/null || true)
  if [ -z "$status" ] && [ -S "$legacy_support/control.sock" ]; then
    ctl_support="$legacy_support"
    status=$(PANE_APPLICATION_SUPPORT="$ctl_support" "$cli" ctl status 2>/dev/null || true)
  fi
  case "$status" in
    *'"mode":"automatic"'*) was_automatic=true ;;
  esac
  if [ -n "$ctl_support" ]; then
    if ! PANE_APPLICATION_SUPPORT="$ctl_support" "$cli" ctl restart >/dev/null 2>&1; then
      pkill -TERM -x spaceman-agent
    fi
  elif ! "$cli" ctl restart >/dev/null 2>&1; then
    pkill -TERM -x spaceman-agent
  fi
fi
for _ in $(seq 1 75); do
  pgrep -x spaceman-agent >/dev/null 2>&1 || break
  sleep 0.2
done
if pgrep -x spaceman-agent >/dev/null 2>&1; then
  echo "Pane did not finish restoring windows; installation aborted" >&2
  exit 1
fi
rm -rf "$installed_bundle" "$legacy_bundle"
ditto "$source_bundle" "$installed_bundle"
codesign --verify --deep --strict "$installed_bundle"
mkdir -p "$cli_root"
ln -sf "$installed_bundle/Contents/MacOS/spaceman" "$cli_root/pane"
ln -sf "$installed_bundle/Contents/MacOS/spaceman" "$cli_root/spaceman"
for _ in $(seq 1 10); do
  open "$installed_bundle" 2>/dev/null && break
  sleep 0.2
done
if $was_automatic; then
  for _ in $(seq 1 75); do
    if "$installed_bundle/Contents/MacOS/spaceman" ctl resume >/dev/null 2>&1; then
      break
    fi
    sleep 0.2
  done
fi

printf '%s\n' "$installed_bundle"
