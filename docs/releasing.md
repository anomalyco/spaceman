# Releasing

Spaceman is not release-ready merely because `scripts/build-app.sh` produces a signed bundle. The
script builds locked release binaries, embeds Sparkle 2.9.4, assembles `Spaceman.app`, applies
hardened-runtime Developer ID signing from the innermost Sparkle helpers outward, and verifies the
signature. It does not notarize, staple, or publish an artifact.

## Direct Distribution

Spaceman's release path is direct distribution outside the Mac App Store. Release artifacts use
Developer ID signing and Apple's notarization service so Gatekeeper can verify them. App Store
Connect upload and TestFlight tooling are not part of this release path.

Release preparation requires Xcode command-line tools, Rust 1.100.0, a Developer ID Application
identity, Sparkle's Ed25519 key in Keychain, and the notarization profile below. Publication also
requires an authenticated `wrangler` CLI. `notarytool` prompts securely for an app-specific password
and stores the validated credential in Keychain; no secret enters the repository:

```sh
xcrun notarytool store-credentials spaceman-notary \
  --apple-id "YOUR_APPLE_ID" \
  --team-id "YOUR_TEAM_ID"
```

Update the package version in `Cargo.toml`/`Cargo.lock`, `CFBundleShortVersionString`, the monotonically
increasing `CFBundleVersion`, release notes, and versioned download references in README, the setup
skill, and website. Commit and push a clean release revision, then prepare the exact artifact:

```sh
./scripts/release-app.sh
```

The script auto-detects the first `Developer ID Application` identity and uses the
`spaceman-notary` Keychain profile. Override those choices with `SPACEMAN_CODESIGN_IDENTITY` and
`SPACEMAN_NOTARY_KEYCHAIN_PROFILE`. Preparation requires release notes at
`changelog/<version>.md` (frontmatter `date` and `summary`, then `## Major`, `## Improvements` and
`## Fixes`); set `SPACEMAN_RELEASE_NOTES` to use another file.
`SPACEMAN_RELEASE_ARCH` defaults to the host architecture, must be `arm64` or `x86_64`, and cannot
request a cross-architecture build. Preparation builds, signs, notarizes, staples, validates,
archives, checksums, and signs a local Sparkle appcast. Artifacts are written to `dist/` but are not
uploaded.

After testing the prepared archive, publish it and its appcast:

```sh
./scripts/release-app.sh publish
```

Publishing uploads the versioned ZIP, checksum, and latest alias before replacing `appcast.xml`.
Published version names must never be reused. The script downloads and compares each public object
before publishing the feed, then
compares the public feed byte-for-byte. This artifact-first/feed-last order prevents clients from
seeing an update whose download is not available. The bucket is `spaceman-releases`; the public URL
is the `SUFeedURL` baked into `Info.plist`. A missing feed is accepted only when
`SPACEMAN_FIRST_RELEASE=1` explicitly confirms initial publication; other fetch failures abort.

Sparkle update archives and the appcast are signed with the Ed25519 private key stored in Keychain
under service `https://sparkle-project.org`, account `ed25519`. Only its public key is embedded in
`Info.plist`; release preparation derives the Keychain key's public half and refuses to continue if it
does not match. Installed builds require a valid signed feed as well as a valid signed archive.

## Where Releases Run

Only the MacBook Pro (`ssh pro`) has the Developer ID identity, the `spaceman-notary` profile, the
Sparkle key and `wrangler` auth. Push the release commit from any Mac, then on the Pro, in a fresh
worktree of that commit, run `./scripts/release-app.sh`, test, run `./scripts/release-app.sh publish`
and `bun run deploy` in `website/`. Notarization needs the login keychain, so run those from a
LaunchAgent (or a logged-in terminal) rather than a plain `ssh` session. Tag the release `vX.Y.Z`
after publishing.

## Source Build

```sh
SPACEMAN_CODESIGN_IDENTITY="Developer ID Application: Example (TEAMID)" \
  ./scripts/build-app.sh
codesign --verify --deep --strict target/Spaceman.app
```

Do not publish `target/Spaceman.app` as a release based only on those commands.

## Remaining Qualification Gaps

- Decide and validate supported architectures, including universal packaging if promised.
- Validate the declared minimum macOS version and a current macOS version.
- Test first install, Accessibility onboarding, launch-at-login registration and relaunch, shortcuts,
  displays, upgrade, crash recovery, pause/quit restoration, and uninstall on a clean machine.
- Add and verify a concise product demo.
- Recheck the private `_AXUIElementGetWindow` risk and state clearly that the Mac App Store is not
  supported.
- Review `SECURITY.md`, `CONTRIBUTING.md`, license, version, changelog/release notes, and support policy
  before broadening the public preview.

## Release Rules

1. Release from a clean, reviewed commit and retain exact source revision and toolchain metadata.
2. Run all verification in [Development](development.md), both isolated live harnesses, and
   `git diff --check`.
3. Build once for release; notarize and distribute that exact artifact rather than rebuilding it.
4. Verify signature, notarization/stapling, archive contents, executable architecture, plist minimum
   OS, and checksum after packaging.
5. Install the downloadable artifact on a clean supported Mac and execute the recovery/uninstall path.
6. Never ask users to disable SIP or delete parked-window metadata to complete an install or upgrade.
7. Keep source-build instructions separate from artifact installation so users are not told they need
   a Developer ID identity for a future release.
