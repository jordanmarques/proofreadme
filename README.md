# ProofReadMe

ProofReadMe proofreads the text selected in any application and replaces it with its corrected
version, on a double press of a modifier key (Ctrl by default on Windows, Cmd on macOS). The
replacement goes through the clipboard (ADR 0002): Ctrl/Cmd+Z in the application undoes it, and
the clipboard holds what it held before (text and image).

## Install

Download the package of your system from the
[Releases](https://github.com/jordanmarques/proofreadme/releases/latest) of the repository
(`SHA256SUMS.txt` holds their checksums).

- **macOS**: open `proofreadme-<version>-macos-universal.dmg` (Apple silicon and Intel, macOS 11 or
  later) and drag `ProofReadMe.app` onto `Applications`, then open it from there. Grant the
  Accessibility permission when the window asks (Privacy & Security > Accessibility, turn
  ProofReadMe on), then relaunch it as the window says. Enter the provider in the settings.
- **Windows**: run `proofreadme-<version>-windows-x64.msi`. It installs for the current user, with
  no administrator rights, to `%LOCALAPPDATA%\Programs\ProofReadMe\ProofReadMe.exe`, adds a Start
  menu shortcut, and launches ProofReadMe. No permission is asked. Enter the provider in the
  settings.

The packages are not signed with a Developer ID: on first launch Gatekeeper (macOS: allow it under
System Settings, Privacy & Security) or SmartScreen (Windows: "More info", then "Run anyway") may
hold them back.

On its first launch the agent registers itself at login (LaunchAgent on macOS, `Run` key of the
user on Windows), pointing at the location it was launched from: launch it from its final place.
The option is in the settings.

## Update

- **macOS**: quit ProofReadMe (session icon > Quit), drag the `ProofReadMe.app` of the new disk
  image onto `Applications` (replace), and launch it.
- **Windows**: run the new `.msi`. It stops the running agent, replaces the installed version and
  launches it again in the background.

Settings and the API key are kept.

On macOS the Accessibility permission stays granted as long as the new build is signed with the
same certificate. If ProofReadMe asks for it again (a build signed ad hoc or with another
certificate), remove ProofReadMe from the Accessibility list with the minus button, then add it
again.

## Uninstall

Quit ProofReadMe and turn off "launch at login" in its settings first (or remove the entries listed
below by hand).

- **macOS**: delete `/Applications/ProofReadMe.app`, the folder
  `~/Library/Application Support/ProofReadMe` (settings and agent log) and, if still there,
  `~/Library/LaunchAgents/io.github.jordanmarques.proofreadme.plist`. Remove the API key from
  Keychain Access (item `io.github.jordanmarques.proofreadme`) and the permission with
  `tccutil reset Accessibility io.github.jordanmarques.proofreadme`.
- **Windows**: uninstall ProofReadMe from Settings > Apps > Installed apps (this also removes its
  registration at login), then delete the folder `%LOCALAPPDATA%\ProofReadMe` (settings, agent
  log, icon of the notifications). Remove the API key from the Credential Manager (Windows
  Credentials, the entry whose name contains `io.github.jordanmarques.proofreadme`), then the
  registration of the notifications:
  ```
  reg delete HKCU\Software\Classes\AppUserModelId\io.github.jordanmarques.proofreadme /f
  ```

## Build the package locally

```
cargo xtask package
```

builds a release for the host system and writes the package to `dist/`:

- **macOS**: `dist/ProofReadMe.app`, with no Dock icon (the agent lives in the menu bar), signed
  with the self-signed certificate of the next section. `--universal` builds it for Apple silicon
  and Intel (`rustup target add aarch64-apple-darwin x86_64-apple-darwin`), `--dmg` also writes
  the disk image `dist/ProofReadMe.dmg`.
- **Windows**: `dist/ProofReadMe.exe`, a single executable with no console, carrying its icon and
  its version information. `--msi` also writes the installer `dist/ProofReadMe.msi` (needs WiX:
  `dotnet tool install --global wix --version 5.0.2`).

Releases are built by the CI: see [docs/releasing.md](docs/releasing.md).

## Signing certificate (macOS, once per build machine)

macOS ties the Accessibility permission to the signature of the application (ADR 0004). The
`.app` is therefore signed with a stable self-signed certificate of the login keychain: every
build signed with it keeps the permission. The certificate is named `ProofReadMe Signing`; to use
another name, set `PROOFREADME_SIGNING_IDENTITY=<name>` when packaging.

Create it once, either with **Keychain Access**:

1. Keychain Access > Certificate Assistant > Create a Certificate…
2. Name `ProofReadMe Signing`, Identity Type **Self Signed Root**, Certificate Type **Code Signing**.
3. Create, and keep it in the **login** keychain.

or in a terminal:

```
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=ProofReadMe Signing" \
  -addext "keyUsage=critical,digitalSignature" -addext "extendedKeyUsage=critical,codeSigning" \
  -keyout signing.key -out signing.crt
openssl pkcs12 -export -legacy -inkey signing.key -in signing.crt -out signing.p12 -passout pass:once
security import signing.p12 -k ~/Library/Keychains/login.keychain-db -P once -T /usr/bin/codesign
rm signing.key signing.crt signing.p12
```

(`-legacy` is for OpenSSL 3; drop it with the LibreSSL of macOS.) `security find-identity -p
codesigning` must then list `"ProofReadMe Signing"`; "not trusted" next to it is expected and does
not prevent signing. On the first package, macOS asks whether `codesign` may use the key: answer
**Always Allow**.

Without the certificate, `cargo xtask package` stops before building and says how to create it.
`cargo xtask package --ad-hoc` packages anyway with an ad hoc signature (this is what the CI
does without the secrets of the next section), with a warning: such a build loses the Accessibility permission at every update. Keep the
certificate: deleting it and creating a new one also means granting the permission again.

## Signing the CI builds (macOS, once)

The release pipeline signs the `.app` with the same `ProofReadMe Signing` certificate, taken from
two repository secrets: `MACOS_SIGNING_P12` (the certificate and its key, PKCS#12, in base64) and
`MACOS_SIGNING_PASSWORD` (the password of that file). Until they are set, the CI signs ad hoc and
says so in a warning: its builds then lose the Accessibility permission at every update.

Create the certificate and set the two secrets once, on a Mac, from a clone of the repository
(`gh` logged in with access to it):

```
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=ProofReadMe Signing" \
  -addext "keyUsage=critical,digitalSignature" -addext "extendedKeyUsage=critical,codeSigning" \
  -keyout signing.key -out signing.crt
password="$(openssl rand -hex 24)"
openssl pkcs12 -export -legacy -inkey signing.key -in signing.crt -out signing.p12 -passout "pass:$password"
base64 -i signing.p12 | tr -d '\n' | gh secret set MACOS_SIGNING_P12
printf '%s' "$password" | gh secret set MACOS_SIGNING_PASSWORD
security import signing.p12 -k ~/Library/Keychains/login.keychain-db -P "$password" -T /usr/bin/codesign
rm signing.key signing.crt signing.p12
```

(`-legacy` is for OpenSSL 3, as above.) The `security import` line puts the same certificate in
the login keychain, so local builds and CI builds share the signature and the permission. If a
`ProofReadMe Signing` certificate is already in the login keychain, export it instead (Keychain
Access, right-click on it > Export, format `.p12`, with a password), set the secrets from that file
with the `base64` and `printf` lines above, and skip the other lines: two certificates of the same
name would make `codesign` refuse to choose.

Keep the secrets: replacing the certificate means granting the permission again on every Mac.

## Permissions

- **macOS**: Accessibility, for the keyboard hook that sees the trigger and for the copy and paste
  of the selection. Until it is granted, the window shows a permission panel and the trigger is
  off.
- **Windows**: none. Notifications show as coming from "ProofReadMe": the agent registers its
  AppUserModelID for the user at launch (`HKCU\Software\Classes\AppUserModelId`), and the Start menu
  shortcut the installer adds carries the same AppUserModelID.

## Known limits

- **Windows, elevated applications.** An application running as administrator receives neither
  the keystrokes nor the copy of a non-elevated agent: there, the copy brings nothing back and the
  proofread silently does nothing. Run the application without elevation to proofread in it.
- **Clipboard formats.** Only the text and the image of the clipboard are given back after a
  proofread. Anything else it held (copied files, rich text, formats of a single application) is
  lost.
