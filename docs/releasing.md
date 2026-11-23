# Releasing

Spaceman is not release-ready merely because `scripts/build-app.sh` produces a signed bundle. The
script builds locked release binaries, embeds Sparkle 2.9.4, assembles `Spaceman.app`, applies
hardened-runtime Developer ID signing from the innermost Sparkle helpers outward, and verifies the
signature. It does not notarize, staple, or publish an artifact. With `SPACEMAN_RELEASE_UNSIGNED=1` it
stops before signing; the Release workflow builds that way and signs in a separate job.

## Direct Distribution

Spaceman ships outside the Mac App Store as `anomaly.spaceman`, signed with Anomaly's Developer ID
(team `5NZ4Q7NXJ4`) and notarized so Gatekeeper accepts it. App Store Connect and TestFlight are not
part of this path. Releases up to 0.3.0 were `com.kitlangton.spaceman` under a personal Developer ID;
the first update to `anomaly.spaceman` keeps config, workspace state and parked-window recovery (the
paths don't depend on the bundle ID) but macOS asks for Accessibility access again, because that
grant belongs to the old identity.

A release has two halves:

```text
Release workflow (GitHub Actions)              scripts/release-app.sh (release Mac)
build ─► sign + notarize ─► verify ─► publish   download that zip ─► validate ─► Sparkle appcast ─► R2
         (Apple keys from AWS)       anoma.ly/release/spaceman            (Sparkle key, wrangler)
```

### 1. The Release workflow

[`.github/workflows/release.yml`](../.github/workflows/release.yml) runs on `dev`, the default branch:
every push publishes the dev channel, and a dispatch picks a channel:

| Channel | Publishes |
| --- | --- |
| `check` | Nothing. Builds, signs, notarizes and verifies, then pushes and deletes a `v0.0.0-test-<run>` tag. |
| `dev` (also every push that changes more than docs, the website or Markdown) | `0.0.0-dev-<run>` on the dev channel (`https://anoma.ly/release/spaceman/desktop/dev.json`). |
| `stable` | `Cargo.toml`'s version on the stable channel, then tags `v<version>`. Needs `changelog/<version>.md` and no existing tag. |

Its jobs:

- **build-macos** (`macos-26`, no secrets): `SPACEMAN_RELEASE_UNSIGNED=1 scripts/build-app.sh`, zipped
  and uploaded with a `SHA256SUMS` whose digest the job reports.
- **sign-macos** (environment `release`): checks out only `.github/release/`, assumes
  `role/github/spaceman-release` through GitHub OIDC, reads `shared/APPLE_SIGNING_CERTIFICATE` and
  `shared/APPLE_NOTARY_KEY` from AWS Secrets Manager in one step, and with the environment's
  `APPLE_NOTARY_KEY_ID` and `APPLE_NOTARY_ISSUER_ID` signs inside-out, notarizes, staples and writes
  `Spaceman-<version>-macos-arm64.zip` and its `.sha256`. Every Mach-O in the bundle must be one it
  signs.
- **verify-macos** (no secrets): `scripts/validate-release.sh` on the signed app and zip, the team ID
  and designated requirement, Gatekeeper's `Notarized Developer ID`, and a run of the signed CLI.
- **publish** (environment `release`): publishes the zip as target `macos-arm64` (file
  `Spaceman.zip`) through the vendored release client, which checks the served bytes.
- **tag**: `v<version>` on the released commit, only after publishing.

`.github/release/` and this workflow are the release boundary: jobs that hold a secret or an OIDC
token run only those scripts, on artifacts they verify by digest and never execute. CODEOWNERS makes
every change under `.github/` need Dax's review.

### 2. Sparkle's appcast

Installed builds update through Sparkle, which reads the signed `appcast.xml` at `SUFeedURL` on R2
(bucket `spaceman-releases`); Sparkle doesn't read the release service. After a stable run, on the
release Mac (which holds the Sparkle key in Keychain and `wrangler` auth), in a clean worktree of
the released commit:

```sh
./scripts/release-app.sh          # prepare
./scripts/release-app.sh publish
```

`prepare` requires the stable channel's current release to be this version, downloads its
`macos-arm64` zip, checks its size and SHA-256 against the manifest, runs
`scripts/validate-release.sh` and the team check, and signs a local appcast with the Ed25519 key in
Keychain (service `https://sparkle-project.org`, account `ed25519`). It refuses a `CFBundleVersion`
that isn't newer than the published feed's. It never builds or signs the app, so R2 serves the exact
bytes the release service does. Generating the appcast needs the login keychain, so run it from a
LaunchAgent or a logged-in terminal rather than a plain `ssh` session. Test the zip in `dist/`, then
`publish` uploads the versioned zip, checksum and latest alias, compares each public object, and
only then replaces and compares `appcast.xml`. A missing feed is accepted only with
`SPACEMAN_FIRST_RELEASE=1`. Then deploy the website (`bun run deploy` in `website/`).

Before a release, update the version in `Cargo.toml`/`Cargo.lock` and `CFBundleShortVersionString`,
raise `CFBundleVersion`, add `changelog/<version>.md` (frontmatter `date` and `summary`, then
`## Major`, `## Improvements` and `## Fixes`), and update the versioned download links in README,
the setup skill and the website. Published version names are never reused.

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
