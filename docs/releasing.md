# Releasing ProofReadMe

The pipeline is [`.github/workflows/release.yml`](../.github/workflows/release.yml). It runs on the push of a tag `v<version>` and publishes the GitHub Release of that tag with the macOS disk image, the Windows installer and their checksums.

## Make a release

1. Bump the version on a branch: `cargo xtask version set 1.2.0` writes `Cargo.toml` and `Cargo.lock`. A version with a hyphen (`1.2.0-rc.1`) is published as a pre-release.
2. Commit, open a pull request and merge it into `main` once the CI is green.
3. Tag the merge commit and push the tag:
   ```
   git switch main && git pull
   git tag v1.2.0
   git push origin v1.2.0
   ```
4. Wait for the workflow (`gh run watch`). The release `ProofReadMe v1.2.0` is published with generated notes; edit them on GitHub if needed.

The workflow refuses a tag that does not match the workspace version of its commit (`v1.2.0` on a commit whose `Cargo.toml` says `1.1.0`). If the release of the tag already exists (created by hand, for instance), the packages are added to it, replacing files of the same name, instead of failing.

## What is built

| File | Content |
|---|---|
| `proofreadme-<version>-macos-universal.dmg` | disk image holding `ProofReadMe.app` (one executable for Apple silicon and Intel, macOS 11 or later) and a link to `/Applications` |
| `proofreadme-<version>-windows-x64.msi` | per-user installer of `ProofReadMe.exe` (64-bit, no console) at `%LOCALAPPDATA%\Programs\ProofReadMe`, with a Start menu shortcut; a newer one replaces the installed version |
| `SHA256SUMS.txt` | checksums of the two files |

Both come from `cargo xtask package` (`--universal --dmg` on macOS, `--msi` on Windows), the same command used locally. The Windows job installs the MSI, installs it again as an update, then uninstalls it, before keeping it.

A pre-release takes the installer version of the release it precedes (`1.2.0-rc.1` installs as `1.2.0`): Windows Installer only knows three numbers. The installer accepts the same version again, so the final `1.2.0` still replaces the release candidate.

## Signing

The macOS bundle is signed with the self-signed `ProofReadMe Signing` certificate (ADR 0004), taken from the repository secrets `MACOS_SIGNING_P12` and `MACOS_SIGNING_PASSWORD` (README, "Signing the CI builds"). Without them the bundle is signed ad hoc and the run carries a warning: such a build loses the Accessibility permission at every update.

Nothing is signed with a Developer ID, nothing is notarised, and the Windows installer is not signed. A downloaded build is therefore held back on first launch: by Gatekeeper on macOS (allow it under System Settings, Privacy & Security) and by SmartScreen on Windows ("More info", then "Run anyway").

## Dry run

A pull request that touches `release.yml`, `xtask/` or the application's build script runs the same build jobs and keeps the packages as workflow artifacts, without publishing anything. So does a manual run (`gh workflow run release.yml --ref <branch>`).
