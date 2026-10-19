# ProofReadMe

ProofReadMe proofreads the text selected in any application and replaces it with its corrected
version, on a double press of a modifier key (Ctrl by default on Windows, Cmd on macOS). The
replacement goes through the clipboard (ADR 0002): Ctrl/Cmd+Z in the application undoes it, and
the clipboard holds what it held before (text and image).

## Build the package

```
cargo xtask package
```

builds a release for the host system and writes the package to `dist/`:

- **macOS**: `dist/ProofReadMe.app`, with no Dock icon (the agent lives in the menu bar), signed
  with the self-signed certificate of the next section.
- **Windows**: `dist/ProofReadMe.exe`, a single executable with no console, carrying its icon and
  its version information.

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
does), with a warning: such a build loses the Accessibility permission at every update. Keep the
certificate: deleting it and creating a new one also means granting the permission again.

## Install

- **macOS**: move `ProofReadMe.app` to `/Applications` and open it. Grant the Accessibility
  permission when the window asks (Privacy & Security > Accessibility, turn ProofReadMe on), then
  relaunch it as the window says. Enter the provider in the settings.
- **Windows**: copy `ProofReadMe.exe` to a folder it will stay in, for example
  `%LOCALAPPDATA%\Programs\ProofReadMe\`, and run it. No permission is asked. Enter the provider in
  the settings.

On its first launch the agent registers itself at login (LaunchAgent on macOS, `Run` key of the
user on Windows), pointing at the location it was launched from: launch it from its final place.
The option is in the settings.

## Update

Quit ProofReadMe (session icon > Quit), replace the `.app` or the `.exe` **at the same location**
with the new build, and launch it. Settings and the API key are kept.

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
- **Windows**: delete `ProofReadMe.exe` and the folder `%LOCALAPPDATA%\ProofReadMe` (settings,
  agent log, icon of the notifications). Remove the API key from the Credential Manager (Windows
  Credentials, the entry whose name contains `io.github.jordanmarques.proofreadme`), then the registration of
  the notifications and, if still there, the one at login:
  ```
  reg delete HKCU\Software\Classes\AppUserModelId\io.github.jordanmarques.proofreadme /f
  reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v ProofReadMe /f
  ```

## Permissions

- **macOS**: Accessibility, for the keyboard hook that sees the trigger and for the copy and paste
  of the selection. Until it is granted, the window shows a permission panel and the trigger is
  off.
- **Windows**: none. Notifications show as coming from "ProofReadMe": the agent registers its
  AppUserModelID for the user at launch (`HKCU\Software\Classes\AppUserModelId`), no Start menu
  shortcut needed.

## Known limits

- **Windows, elevated applications.** An application running as administrator receives neither
  the keystrokes nor the copy of a non-elevated agent: there, the copy brings nothing back and the
  proofread silently does nothing. Run the application without elevation to proofread in it.
- **Clipboard formats.** Only the text and the image of the clipboard are given back after a
  proofread. Anything else it held (copied files, rich text, formats of a single application) is
  lost.
- **No installer, no Developer ID.** The packages are built locally. A copy downloaded from
  elsewhere would be held back by Gatekeeper (macOS) or SmartScreen (Windows) on first launch.
