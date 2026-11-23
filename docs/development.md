# Development

Read the root [AGENTS.md](../AGENTS.md) before changing implementation. It defines architecture and
safety invariants. [PLAN.md](../PLAN.md) and [RESEARCH.md](RESEARCH.md) are historical design records,
not current product documentation.

## Dev Loop

```sh
scripts/dev          # build and install now, then again on every save
scripts/dev --once   # build and install once
```

Each install restores every window, swaps `/Applications/Spaceman.app`, and resumes tiling, so
workspaces and layouts come back. `scripts/install-app.sh --bundle PATH --wait-idle SECS` installs a
built bundle once the keyboard and mouse have been idle; the updater
(`scripts/spaceman-updater`, set up with `scripts/install-updater`) uses it to keep the installed
app on `main`.

## Verify

```sh
scripts/check            # fmt, release metadata, clippy, tests, audit, CA roots, replay, config, website
scripts/check --no-web   # the same without the website
```

### Focused Reliability Checks

The reliability regressions in `src/app/tests/reliability.rs` exercise ambiguous multi-display tabs,
cross-file crash recovery, delayed window-server confirmation, and display-gap parking through the
fake backend. `src/app/input.rs` tests ordered input, queue limits, and complete gestures without
preview ticks. Layout and geometry tests cover resize-then-drag and multi-display parking. Run the
focused checks without touching desktop windows:

```sh
cargo test --locked --lib app::tests::reliability
cargo test --locked --lib app::input::tests
cargo test --locked --lib layout_tree::tests
cargo test --locked --lib geometry::tests
```

Read-only or dry-run macOS probes:

```sh
cargo run --locked -- config-check config.example.toml
cargo run --locked -- displays
cargo run --locked -- snapshot
cargo run --locked -- retile
cargo run --locked -- watch 20
cargo run --locked -p spaceman-agent -- --size 1324x580 --playground Activity/activity --snap /tmp/activity.png
```

Commands with `--apply` and the menu-bar app move real windows. Disable every other window manager and
use disposable windows before running them. The live harnesses below are the exception: they require
AeroSpace to be enabled initially so they can capture a baseline, then they disable and restore it.

## Live Acceptance

```sh
./scripts/live-ghostty-tabs.sh
./scripts/live-workspaces.sh
```

`live-workspaces.sh` requires a current signed bundle at `target/Spaceman.app`; build it with
`./scripts/build-app.sh` first. It isolates control and recovery files with
`SPACEMAN_APPLICATION_SUPPORT`. `live-ghostty-tabs.sh` instead uses `cargo run` to start a scoped,
non-persistent `watch-window` process. Both restore their baseline but still exercise live
Accessibility writes. Start them with AeroSpace enabled, read each script, preserve unrelated
windows, and confirm its expected display and Ghostty assumptions. Never run a destructive live test
on a user's normal workspace merely because unit tests pass.

## Documentation Checks

When commands, menu labels, config fields, state paths, shortcuts, packaging, or recovery semantics
change, update README and the corresponding document in the same change. Inspect links, stale labels,
and whitespace:

```sh
rg -n '\[[^]]+\]\([^)]+' --glob '*.md'
rg -n 'Disable Tiling|Enable Tiling|Settings\.\.\.|Quit Spaceman' --glob '*.md'
rg -n 'Automatic Tiling|Retile Now|starts paused|paused startup' --glob '*.md' --glob '!docs/development.md'
git diff --check
```
