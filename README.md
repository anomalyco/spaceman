# Spaceman

Spaceman is an experimental macOS tiling window manager built around stable logical windows. A
native window, including a selected Ghostty native tab, can change physical Accessibility identity
without creating a new tile. Spaceman keeps the logical slot and replaces its physical binding when
the evidence is unambiguous.

[Website](https://spaceman.kitlangton.dev) · [Download for Apple silicon](https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v0.1.3-macos-arm64.zip)

> [!WARNING]
> Disable AeroSpace, yabai, Amethyst, and every other window manager before first opening Spaceman
> or installing it from source. Two managers can repeatedly move the same windows. Keep the parked
> window state until Spaceman has successfully paused or quit and restored every window.

Spaceman currently:

- starts automatic tiling when Accessibility permission and its global hotkeys are available;
- provides ten default numbered workspaces plus opt-in named workspaces A-Z, with 1-5 on the main
  display and 6-10 on the secondary display by default;
- assigns workspaces to `main`, `secondary`, a display UUID, or an exact display name, falling back
  to the main display when a preferred display disconnects;
- persists active workspaces, nested split layouts, tile order and weights, and parked-window recovery
  metadata across Spaceman restarts, and remembers each application's window placements by bundle
  identifier so reboots and startup recovery return windows to their workspaces;
- journals window arrangements and model/screen disagreements locally, summarized by
  `spaceman doctor`;
- can register its signed app bundle with macOS to launch automatically at login;
- supports directional focus and movement across displays, drag-to-reorder/nest, mouse and keyboard
  resizing, balancing, and a workspace-wide Fullscreen Layout;
- keeps movable non-resizable and rule-floated windows with their logical workspace without tiling
  or resizing them;
- exposes a local CLI over a user-only Unix socket.

Spaceman uses Accessibility APIs to observe, focus, move, and resize eligible windows. It requires no
SIP change. Read [Security and privacy](docs/security-and-privacy.md),
[Compatibility](docs/compatibility.md), and [Limitations](docs/limitations.md) before trying it.

## Install Preview

The current preview is a Developer ID-signed, notarized arm64 build for Apple silicon Macs. It remains
experimental; read [Known limitations](docs/limitations.md) before installing it.

1. Disable every other window manager. For AeroSpace: `aerospace enable off`.
2. Download [Spaceman 0.1.3 for Apple silicon](https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v0.1.3-macos-arm64.zip)
   and its [SHA-256 checksum](https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases/Spaceman-v0.1.3-macos-arm64.zip.sha256).
3. In the download directory, run `shasum -a 256 -c Spaceman-v0.1.3-macos-arm64.zip.sha256`.
4. Unzip it, move `Spaceman.app` to `/Applications`, and open it.
5. Link the bundled CLI: `mkdir -p "$HOME/.local/bin" && ln -sf "/Applications/Spaceman.app/Contents/MacOS/spaceman" "$HOME/.local/bin/spaceman"`.
6. Complete the Accessibility setup shown by Spaceman.

Later preview updates are signed and delivered through Sparkle. Use Spaceman Settings for upgrades. If
a manual replacement is necessary, pause and quit the running version and confirm that every parked
window has returned before replacing its bundle.

## Install From Source

Source builds are distinct from the notarized preview artifact and require your own `Developer ID
Application` signing identity.

Requirements: macOS, Xcode command-line tools, Rust 1.95.0, and a Developer ID Application identity.

From an existing repository checkout, first disable every other window manager, then run:

```sh
./scripts/install-app.sh
```

Set `SPACEMAN_CODESIGN_IDENTITY` if auto-detection does not choose the desired identity. The script
builds and signs `target/Spaceman.app`, installs it at `/Applications/Spaceman.app`, links the CLI to
`~/.local/bin/spaceman`, and opens the app.

Ensure `~/.local/bin` is on `PATH`. For zsh:

```sh
printf '\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$HOME/.zshrc"
exec zsh
```

Artifact users do not need a Developer ID identity. See [Releasing](docs/releasing.md) for the
maintainer workflow.

## First Open

1. Disable every other window manager. For AeroSpace: `aerospace enable off`.
2. Open Spaceman: `open "/Applications/Spaceman.app"`.
3. Spaceman opens Settings when Accessibility access is missing. Choose `Grant Access`, then enable
   Spaceman under **System Settings > Privacy & Security > Accessibility**.
4. Optionally enable `Launch at Login` in Spaceman Settings.
5. Confirm operation with `spaceman ctl status`.

When Accessibility permission is granted and hotkeys register successfully, Spaceman starts automatic
tiling; it does not otherwise start paused intentionally. Without Accessibility permission, the menu
reports the permission error and tiling remains paused. The menu actions are `Disable Tiling` (or
`Enable Tiling` while paused),
`Launch at Login`, `Settings...`, and `Quit Spaceman`. Settings shows Accessibility, launch-at-login,
and signed software-update controls. If macOS requires approval, the login item action opens **System
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

Character shortcuts follow the active macOS keyboard layout. Workspace digits remain the physical
ANSI number row, and Tab remains physical Tab. See [Controls](docs/controls.md) for drag, resize,
balance, CLI, and socket behavior.

Commands and completed gestures wait in arrival order when the worker is busy; repeated resize or
focus commands are not collapsed into one. Workspace switches keep the outgoing windows visible if
incoming parked windows cannot finish restoring. Parking targets are checked against every attached
display, including middle displays in three-display arrangements.

```sh
spaceman ctl status
spaceman ctl switch 2
spaceman ctl move 7
spaceman ctl layout
spaceman ctl balance
spaceman ctl pause
spaceman ctl resume
```

## Configure

Spaceman reads `~/Library/Application Support/Spaceman/config.toml` at process startup. See the exact
current schema and [config.example.toml](config.example.toml) in
[Configuration](docs/configuration.md).

```sh
spaceman config-check "$HOME/Library/Application Support/Spaceman/config.toml"
spaceman displays
```

## Recover Or Remove

Use `spaceman ctl pause` or `spaceman ctl quit` and wait for `spaceman ctl status` or process exit to
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
