# Pane

Pane (formerly Spaceman) is an experimental macOS tiling window manager built around stable logical
windows. A native window, including a selected Ghostty native tab, can change physical Accessibility
identity without creating a new tile. Pane keeps the logical slot and replaces its physical binding
when the evidence is unambiguous.

[Website](https://anoma.ly/pane/) · [Download for Apple silicon](https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Pane-v0.3.0-macos-arm64.zip)

> [!WARNING]
> Disable AeroSpace, yabai, Amethyst, and every other window manager (including any earlier
> `Spaceman.app`) before first opening Pane or installing it from source. Two managers can
> repeatedly move the same windows. Keep the parked window state until Pane has successfully paused
> or quit and restored every window.

Pane currently:

- starts automatic tiling when Accessibility permission and its global hotkeys are available;
- provides ten default numbered workspaces plus opt-in named workspaces A-Z, with 1-5 on the main
  display and 6-10 on the secondary display by default;
- assigns workspaces to `main`, `secondary`, a display UUID, or an exact display name, falling back
  to the main display when a preferred display disconnects;
- persists active workspaces, nested split layouts, tile order and weights, and parked-window recovery
  metadata across Pane restarts, and remembers each application's window placements by bundle
  identifier so reboots and startup recovery return windows to their workspaces;
- migrates existing Spaceman configuration and recovery state safely on first launch without deleting
  the originals;
- journals window arrangements and model/screen disagreements locally, summarized in Settings (`Activity`)
  and by `pane doctor`;
- shows a non-activating Workspace HUD capsule on each display on workspace switches,
  cross-workspace moves, and layout toggles (`[settings] hud = true`): a row of that display's
  workspaces, drawn by default as tabular numbers and app icons (`hud_style = "icons"`), or as
  layout thumbnails (`hud_style = "layouts"`), with a marker that travels to the one you are on. It
  follows quick switches without restarting and honors Reduce Motion;
- can register its signed app bundle with macOS to launch automatically at login;
- supports directional focus and movement across displays, drag-to-reorder/nest, mouse and keyboard
  resizing, balancing, and a workspace-wide Fullscreen Layout;
- keeps movable non-resizable and rule-floated windows with their logical workspace without tiling
  or resizing them;
- exposes a local CLI over a user-only Unix socket.

Pane uses Accessibility APIs to observe, focus, move, and resize eligible windows. It requires no
SIP change. Read [Security and privacy](docs/security-and-privacy.md),
[Compatibility](docs/compatibility.md), and [Limitations](docs/limitations.md) before trying it.

## Install Preview

The current preview is a Developer ID-signed, notarized arm64 build for Apple silicon Macs. It remains
experimental; read [Known limitations](docs/limitations.md) before installing it.

1. Disable every other window manager. For AeroSpace: `aerospace enable off`. If upgrading from
   Spaceman, pause and quit it first (`spaceman ctl pause && spaceman ctl quit`).
2. Download, install and open the latest build:

   ```sh
   curl -fL https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Pane-latest-macos-arm64.zip -o /tmp/Pane.zip \
     && ditto -x -k /tmp/Pane.zip /Applications && open /Applications/Pane.app
   ```

3. Follow the setup tour, which walks through granting Accessibility access.
4. Optionally link the bundled CLI: `mkdir -p "$HOME/.local/bin" && ln -sf "/Applications/Pane.app/Contents/MacOS/spaceman" "$HOME/.local/bin/pane"`.

To check the download first, get
[Pane 0.3.0 for Apple silicon](https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Pane-v0.3.0-macos-arm64.zip)
and its [SHA-256 checksum](https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Pane-v0.3.0-macos-arm64.zip.sha256),
run `shasum -a 256 -c Pane-v0.3.0-macos-arm64.zip.sha256`, then unzip it and move `Pane.app`
to `/Applications`.

Later preview updates are signed and delivered through Sparkle. Use Pane Settings for upgrades. If
a manual replacement is necessary, pause and quit the running version and confirm that every parked
window has returned before replacing its bundle.

## Install From Source

Source builds are distinct from the notarized preview artifact and require your own `Developer ID
Application` signing identity.

Requirements: macOS, Xcode command-line tools, Rust 1.100.0, and a Developer ID Application identity.

From an existing repository checkout, first disable every other window manager, then run:

```sh
./scripts/install-app.sh
```

Set `PANE_CODESIGN_IDENTITY` (or `SPACEMAN_CODESIGN_IDENTITY`) if auto-detection does not choose the
desired identity. The script builds and signs `target/Pane.app`, installs it at `/Applications/Pane.app`
(removing any legacy `/Applications/Spaceman.app` after restoring windows), links the CLI to
`~/.local/bin/pane` and `~/.local/bin/spaceman`, and opens the app.

Ensure `~/.local/bin` is on `PATH`. For zsh:

```sh
printf '\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$HOME/.zshrc"
exec zsh
```

Artifact users do not need a Developer ID identity. See [Releasing](docs/releasing.md) for the
maintainer workflow.

## First Open

1. Disable every other window manager. For AeroSpace: `aerospace enable off`.
2. Open Pane: `open "/Applications/Pane.app"`.
3. On a fresh install, Pane opens its four-step setup tour when Accessibility access is missing.
   Choose `Allow`, enable Pane under **System Settings > Privacy & Security > Accessibility**,
   optionally enable `Launch at Login`, and pick starter app rules.
4. Confirm operation with `pane ctl status`.

When Accessibility permission is granted and hotkeys register successfully, Pane starts automatic
tiling; it does not otherwise start paused intentionally. Without Accessibility permission, the menu
reports the permission error and tiling remains paused. While tiling, the menu lists each display's
workspaces (the visible one checked, plus any holding windows, with their apps' icons); choosing one
switches to it. The menu actions are `Run Setup...` (while
Accessibility is missing), `Disable Tiling` (or `Enable Tiling` while paused), `Workspace HUD`,
`Launch at Login`, `Settings...` (`⌘,`), `Check for Updates...`, and `Quit Pane` (`⌘Q`). Settings
includes `General`, `Rules`, `Workspaces`, `Shortcuts`, and `Activity` (title-free arrangement
diagnostics and process vitals). If macOS requires approval, the login item action opens **System
Settings > General > Login Items & Extensions**.

## Everyday Use

Core controls:

| Keys | Action |
| --- | --- |
| `Option+1` ... `Option+9`, `Option+0` | Switch to workspace 1 ... 10 |
| `Option+Shift+1` ... `Option+Shift+0` | Move the focused window to workspace 1 ... 10 |
| `Option+A` ... `Option+Z` | Switch to a configured named workspace |
| `Option+Shift+A` ... `Option+Shift+Z` | Move the focused window to a configured named workspace |
| `Option+Tab` | Return to the previous workspace on the focused display |
| Option-backtick | Focus the next tiled window in the workspace |
| `Option+H/J/K/L` | Focus left/down/up/right |
| `Option+Shift+H/J/K/L` | Move the focused tile left/down/up/right |
| `Option+/` | Toggle the active layout's root split axis |
| `Option+-`, `Option+=` | Shrink/grow the focused tile by 50 pixels |
| `Option+Shift+F` | Toggle Fullscreen Layout without native fullscreen |
| Three-finger swipe left/right | Next/previous occupied workspace on the display under the pointer |

The swipe shares three fingers with macOS's "Swipe between full-screen applications"; set that to
four fingers or off in Trackpad settings so only one of them moves.

Character shortcuts follow the active macOS keyboard layout. Workspace digits remain the physical
ANSI number row, and Tab remains physical Tab. See [Controls](docs/controls.md) for drag, resize,
balance, CLI, and socket behavior.

Commands and completed gestures wait in arrival order when the worker is busy; repeated resize or
focus commands are not collapsed into one. Workspace switches keep the outgoing windows visible if
incoming parked windows cannot finish restoring. Parking targets are checked against every attached
display, including middle displays in three-display arrangements.

```sh
pane ctl status
pane ctl switch 2
pane ctl move 7
pane ctl layout
pane ctl balance
pane ctl pause
pane ctl resume
```

## Configure

Pane reads `~/.config/pane/config.toml` at startup and reloads rule, display, and HUD (on/off and
style) edits automatically when the file changes. If `~/.config/pane/config.toml` does not exist yet
and a legacy Spaceman config exists at `~/.config/spaceman/config.toml` or
`~/Library/Application Support/Spaceman/config.toml`, Pane copies it safely without modifying the
original. See the exact current schema and
[config.example.toml](config.example.toml) in [Configuration](docs/configuration.md).

```sh
pane config-check "$HOME/.config/pane/config.toml"
pane displays
```

## Recover Or Remove

Use `pane ctl pause` or `pane ctl quit` and wait for `pane ctl status` or process exit to
confirm that restoration completed **before** removing the app or state. Do not delete
`parked-windows.json` as an early troubleshooting step: it records where off-screen parked windows
must return.

After a crash, recorded parked windows missing from otherwise valid workspace state are restored
before automatic reconciliation resumes. Recovery metadata stays intact until restoration and its
durable removal succeed.

Follow [Recovery](docs/recovery.md) for stuck or crashed processes and safe uninstall instructions.

## Documentation

- [Configuration](docs/configuration.md)
- [Controls](docs/controls.md)
- [Recovery and uninstall](docs/recovery.md)
- [Security and privacy](docs/security-and-privacy.md)
- [Compatibility](docs/compatibility.md)
- [Known limitations](docs/limitations.md)
- [Development](docs/development.md)
- [Releasing](docs/releasing.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Historical implementation plan](PLAN.md) and [historical research](docs/RESEARCH.md)

## Status

Public experimental preview. The arm64 artifact is signed, notarized, stapled, checksummed, and served
through a signed Sparkle feed. Broad release readiness is not proven: minimum-OS runtime validation,
clean-machine install/upgrade/recovery/uninstall qualification, wider architecture support, and a
concise product demo remain outstanding.

## License

MIT
