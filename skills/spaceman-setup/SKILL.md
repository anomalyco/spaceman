---
name: spaceman-setup
description: Install, configure, verify, troubleshoot, or uninstall the Pane (formerly Spaceman) tiling window manager on macOS. Use when a user asks to set up Pane or Spaceman, configure window rules or workspaces, transition from another window manager, recover off-screen windows, or remove Pane.
---

# Pane Setup

Pane (formerly Spaceman) is an experimental native macOS tiling window manager for Apple silicon
(`https://anoma.ly/pane/`). The preview declares macOS 14.0 as its minimum; compatibility is not yet
broadly qualified.

## Install

Disable any other window manager first — two managers fight over the same windows:

```sh
aerospace enable off   # AeroSpace
yabai --stop-service   # yabai
```

If upgrading from Spaceman, pause and quit it first (`spaceman ctl pause && spaceman ctl quit`)
so parked windows return before opening Pane. Pane automatically copies legacy Spaceman config and
state on first launch without deleting the originals.

Download, verify, and open:

```sh
cd "$(mktemp -d)"
if [[ -e "/Applications/Pane.app" ]]; then
  echo "Pane is already installed; upgrade through Pane Settings." >&2
  exit 1
fi
curl -fLO "https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Pane-v0.3.0-macos-arm64.zip"
curl -fLO "https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Pane-v0.3.0-macos-arm64.zip.sha256"
shasum -a 256 -c Pane-v0.3.0-macos-arm64.zip.sha256
ditto -x -k Pane-v0.3.0-macos-arm64.zip .
mv Pane.app "/Applications/"
open "/Applications/Pane.app"
```

Pane opens its setup tour and walks through granting Accessibility access
(**System Settings > Privacy & Security > Accessibility**). Optionally enable `Launch at Login`.
Tiling starts automatically once Accessibility permission is granted and hotkeys register.

## Verify

The CLI ships inside the bundle:

```sh
mkdir -p "$HOME/.local/bin"
ln -sf "/Applications/Pane.app/Contents/MacOS/spaceman" "$HOME/.local/bin/pane"
"$HOME/.local/bin/pane" ctl status   # healthy: "mode":"automatic" and "busy":false
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

CLI: `pane ctl switch 2`, `pane ctl move 7`, `pane ctl back-and-forth`,
`pane ctl layout`, `pane ctl balance`, `pane ctl pause`, `pane ctl resume`.

## Configure

Config lives at `~/.config/pane/config.toml`, read at startup and reloaded automatically on save
(`PANE_CONFIG` or legacy `SPACEMAN_CONFIG` overrides the path). Workspaces 1–5 prefer the main
display, 6–10 the secondary; assignments accept `main`, `secondary`, a display UUID, or a display
name. Schema version is `1`; rules are ordered and first-match. A rule accepts exact `bundle_id`
and/or `title` matching, with an `action` of `float`, `route`, or `ignore`; `route` also requires
`workspace`. The Workspace HUD defaults to `hud_style = "icons"` (numbers and app icons); set
`hud_style = "layouts"` for layout thumbnails.

```sh
pane config-check "$HOME/.config/pane/config.toml"
pane displays
```

## Troubleshoot

- **Hotkeys do nothing**: another app owns the `Option` shortcuts, or Accessibility permission is
  missing — the menu bar item reports which.
- **Windows off-screen after a crash**: reopen the same app, relaunch Pane, and let it restore.
  Keep `parked-windows.json` — it records where those windows return. Do not remove recovery state
  until pause or quit has restored every window.
- **Windows keep moving on their own**: another window manager is running; quit it, then
  `pane ctl resume`.
- **Updates**: signed and delivered through Sparkle in Pane Settings.

## Uninstall

Turn off `Launch at Login` in Pane Settings first, then restore and quit:

```sh
pane ctl pause   # wait for "mode":"paused" and "busy":false
pane ctl quit    # wait for spaceman-agent to exit
```

Confirm windows returned, then:

```sh
rm -rf "/Applications/Pane.app"
rm -f "$HOME/.local/bin/pane" "$HOME/.local/bin/spaceman"
rm -rf "$HOME/Library/Application Support/Pane" "$HOME/.config/pane"
```
