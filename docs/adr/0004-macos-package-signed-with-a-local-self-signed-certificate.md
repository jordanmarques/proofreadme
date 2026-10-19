# macOS package signed with a local self-signed certificate

macOS ties the Accessibility permission to the code signature of the application: an unsigned build loses it at every update. `cargo xtask package` therefore signs the `.app` with a stable self-signed certificate from the login keychain, created once on the build machine, so the permission survives updates.

The release pipeline signs with the same certificate, imported from the repository secrets `MACOS_SIGNING_P12` and `MACOS_SIGNING_PASSWORD` into a temporary keychain; without them it signs ad hoc, with a warning.

## Considered Options

- **No signature, as in Hikyaku**: the user grants the permission again after every update.
- **Apple Developer ID with notarisation**: paid account, unneeded for a personal tool.
