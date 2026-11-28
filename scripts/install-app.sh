#!/bin/sh
set -eu

# Usage: scripts/install-app.sh [--bundle PATH] [--wait-idle SECS]
#   --bundle     install an already built Spaceman.app instead of building one
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
resolve_path() {
  _rp_target=$1
  _rp_suffix=
  while [ ! -e "$_rp_target" ]; do
    _rp_parent=$(dirname -- "$_rp_target")
    [ "$_rp_parent" = "$_rp_target" ] && break
    _rp_suffix="/$(basename -- "$_rp_target")$_rp_suffix"
    _rp_target=$_rp_parent
  done
  if [ -e "$_rp_target" ]; then
    _rp_resolved=$(realpath -- "$_rp_target" 2>/dev/null || {
      if [ -d "$_rp_target" ]; then
        (CDPATH= cd -P -- "$_rp_target" && pwd -P)
      else
        _rp_dir=$(CDPATH= cd -P -- "$(dirname -- "$_rp_target")" && pwd -P)
        printf '%s/%s\n' "${_rp_dir%/}" "$(basename -- "$_rp_target")"
      fi
    })
    if [ -z "$_rp_suffix" ]; then
      printf '%s\n' "$_rp_resolved"
    else
      printf '%s%s\n' "${_rp_resolved%/}" "$_rp_suffix"
    fi
  else
    printf '%s%s\n' "$_rp_target" "$_rp_suffix"
  fi
}

paths_overlap() {
  _po_a=${1%/}
  _po_b=${2%/}
  [ "$_po_a" = "$_po_b" ] && return 0
  case "$_po_a" in "$_po_b"/*) return 0 ;; esac
  case "$_po_b" in "$_po_a"/*) return 0 ;; esac
  return 1
}

# build-app.sh prints the built bundle path on its last line.
[ -n "$source_bundle" ] || source_bundle=$("$root/scripts/build-app.sh" | tail -n 1)
[ -d "$source_bundle/Contents" ] || { echo "no Spaceman.app at $source_bundle" >&2; exit 1; }
install_root="/Applications"
installed_bundle="$install_root/Spaceman.app"
legacy_bundle="$install_root/Pane.app"
cli_root="$HOME/.local/bin"
was_automatic=false

open_installed() {
  opened=false
  open_error=
  for _ in $(seq 1 10); do
    if open_error=$(open "$installed_bundle" 2>&1 >/dev/null); then
      opened=true
      break
    fi
    sleep 0.2
  done
  if ! $opened; then
    echo "failed to open $installed_bundle${open_error:+: $open_error}" >&2
    return 1
  fi
  if $was_automatic; then
    for _ in $(seq 1 75); do
      if "$installed_bundle/Contents/MacOS/spaceman" ctl resume >/dev/null 2>&1; then
        break
      fi
      sleep 0.2
    done
  fi
}

source_real=$(resolve_path "$source_bundle")
installed_real=$(resolve_path "$installed_bundle")
legacy_real=$(resolve_path "$legacy_bundle")
if paths_overlap "$source_real" "$installed_real"; then
  echo "refusing to install $source_bundle over itself ($installed_bundle)" >&2
  exit 1
fi
if paths_overlap "$source_real" "$legacy_real"; then
  echo "refusing to install $source_bundle because installation removes $legacy_bundle" >&2
  exit 1
fi

if [ -n "$wait_idle" ]; then
  idle_seconds() { ioreg -c IOHIDSystem | awk '/HIDIdleTime/ { print int($NF / 1000000000); exit }'; }
  while [ "$(idle_seconds)" -lt "$wait_idle" ]; do sleep 5; done
fi

mkdir -p "$install_root"
restoring_pids=$(pgrep -x spaceman-agent || true)
if [ -n "$restoring_pids" ]; then
  status=$("$source_bundle/Contents/MacOS/spaceman" ctl status 2>/dev/null || SPACEMAN_APPLICATION_SUPPORT="$HOME/Library/Application Support/Pane" "$source_bundle/Contents/MacOS/spaceman" ctl status 2>/dev/null || true)
  case "$status" in
    *'"mode":"automatic"'*) was_automatic=true ;;
  esac
  if ! "$source_bundle/Contents/MacOS/spaceman" ctl restart >/dev/null 2>&1 &&
     ! SPACEMAN_APPLICATION_SUPPORT="$HOME/Library/Application Support/Pane" "$source_bundle/Contents/MacOS/spaceman" ctl restart >/dev/null 2>&1; then
    pkill -TERM -x spaceman-agent
  fi
fi
# Several parked windows can take longer than 15 seconds to restore through AX.
for _ in $(seq 1 300); do
  pgrep -x spaceman-agent >/dev/null 2>&1 || break
  sleep 0.2
done
if pgrep -x spaceman-agent >/dev/null 2>&1; then
  echo "Spaceman did not finish restoring windows; installation aborted" >&2
  # A restart request cannot be cancelled: the old process may still exit after we
  # return failure. Leave the installed bundle intact and reopen it if those exact
  # processes finish within the next 5 minutes (bounded so a permanently stuck
  # process won't later reopen after an intentional quit). Detach stdio and ignore
  # HUP so an updater exiting does not lose the recovery.
  (
    trap '' HUP
    exited=false
    for _ in $(seq 1 1500); do
      still_running=false
      for pid in $restoring_pids; do
        if kill -0 "$pid" 2>/dev/null; then
          still_running=true
          break
        fi
      done
      if ! $still_running; then
        exited=true
        break
      fi
      sleep 0.2
    done
    # Another installer or the user may already have relaunched it.
    if $exited && [ -d "$installed_bundle" ] && ! pgrep -x spaceman-agent >/dev/null 2>&1; then
      open_installed
    fi
  ) </dev/null >/dev/null 2>&1 &
  exit 1
fi
rm -rf "$installed_bundle" "$install_root/Pane.app"
ditto "$source_bundle" "$installed_bundle"
codesign --verify --deep --strict "$installed_bundle"
mkdir -p "$cli_root"
rm -f "$cli_root/pane"
ln -sf "$installed_bundle/Contents/MacOS/spaceman" "$cli_root/spaceman"
open_installed

printf '%s\n' "$installed_bundle"
