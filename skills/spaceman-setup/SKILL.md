---
name: spaceman-setup
description: Install, configure, verify, troubleshoot, or uninstall the Spaceman tiling window manager on macOS. Use when a user asks to set up Spaceman, configure window rules or workspaces, transition from another window manager, recover off-screen windows, or remove Spaceman.
---

# Spaceman Setup

Spaceman is an experimental native macOS tiling window manager for Apple silicon. The preview
declares macOS 14.0 as its minimum; compatibility is not yet broadly qualified.

## Install

Disable any other window manager first — two managers fight over the same windows:

```sh
aerospace enable off   # AeroSpace
yabai --stop-service   # yabai
```

Download, verify, and open:

```sh
cd "$(mktemp -d)"
if [[ -e "/Applications/Spaceman.app" ]]; then
  echo "Spaceman is already installed; upgrade through Spaceman Settings." >&2
  exit 1
fi
curl -fLO "https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v0.2.0-macos-arm64.zip"
curl -fLO "https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v0.2.0-macos-arm64.zip.sha256"
shasum -a 256 -c Spaceman-v0.2.0-macos-arm64.zip.sha256
ditto -x -k Spaceman-v0.2.0-macos-arm64.zip .
mv Spaceman.app "/Applications/"
open "/Applications/Spaceman.app"
```

Spaceman opens Settings and walks through granting Accessibility access
(**System Settings > Privacy & Security > Accessibility**). Optionally enable `Launch at Login`.
Tiling starts automatically once Accessibility permission is granted and hotkeys register.

## Verify

The CLI ships inside the bundle:

```sh
mkdir -p "$HOME/.local/bin"
ln -sf "/Applications/Spaceman.app/Contents/MacOS/spaceman" "$HOME/.local/bin/spaceman"
"$HOME/.local/bin/spaceman" ctl status   # healthy: "mode":"automatic" and "busy":false
```

## Controls

| Keys | Action |
| --- | --- |
| `Option+1` … `Option+0` | Switch to workspace 1 … 10 |
| `Option+Shift+1` … `Option+Shift+0` | Move focused window to workspace 1 … 10 |
| `Option+A` … `Option+Z` | Switch to a configured named workspace |
| `Option+Shift+A` … `Option+Shift+Z` | Move focused window to a configured named workspace |
| `Option+Tab` | Previous workspace on the focused display |
| `Option+backtick` | Focus the next tiled window in workspace order |
| `Option+H/J/K/L` | Focus left/down/up/right |
| `Option+Shift+H/J/K/L` | Move the focused tile |
| `Option+/` | Toggle the root split axis |
| `Option+-`, `Option+=` | Shrink/grow the focused tile |
| `Option+Shift+F` | Toggle workspace-wide Fullscreen Layout |

CLI: `spaceman ctl switch 2`, `spaceman ctl move 7`, `spaceman ctl back-and-forth`,
`spaceman ctl layout`, `spaceman ctl balance`, `spaceman ctl pause`, `spaceman ctl resume`.

## Configure

Config lives at `~/Library/Application Support/Spaceman/config.toml`, read at startup
(`SPACEMAN_CONFIG` overrides the path). Workspaces 1–5 prefer the main display, 6–10 the
secondary; assignments accept `main`, `secondary`, a display UUID, or a display name. Schema
version is `1`; rules are ordered and first-match. A rule accepts exact `bundle_id` and/or `title`
matching, with an `action` of `float`, `route`, or `ignore`; `route` also requires `workspace`.

```sh
spaceman config-check "$HOME/Library/Application Support/Spaceman/config.toml"
spaceman displays
```

## Troubleshoot

- **Hotkeys do nothing**: another app owns the `Option` shortcuts, or Accessibility permission is
  missing — the menu bar item reports which.
- **Windows off-screen after a crash**: reopen the same app, relaunch Spaceman, and let it restore.
  Keep `parked-windows.json` — it records where those windows return. Do not remove recovery state
  until pause or quit has restored every window.
- **Windows keep moving on their own**: another window manager is running; quit it, then
  `spaceman ctl resume`.
- **Updates**: signed and delivered through Sparkle in Spaceman Settings.

## Uninstall

Turn off `Launch at Login` in Spaceman Settings first, then restore and quit:

```sh
spaceman ctl pause   # wait for "mode":"paused" and "busy":false
spaceman ctl quit    # wait for spaceman-agent to exit
```

Confirm windows returned, then:

```sh
rm -rf "/Applications/Spaceman.app"
rm -f "$HOME/.local/bin/spaceman"
rm -rf "$HOME/Library/Application Support/Spaceman"
```
