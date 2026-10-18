# Session agent in Rust/egui, with a copy of Kairo

ProofReadMe follows the shape of Hikyaku: a single Rust binary launched at login (LaunchAgent on macOS, `Run` key on Windows) that lives in the menu bar or the notification area and carries its own settings window, drawn with egui and dressed with Kairo. The Kairo crate is copied into `crates/kairo` rather than taken as a git dependency on `hikyaku-desktop`: the two applications evolve on their own, and a change to Kairo in one never breaks the build of the other.

## Considered Options

- **Kairo as a git dependency**: a single source of truth, but couples the releases of the two repositories.
- **Tauri, or native Swift and C#**: a webview or two codebases, for no gain over the patterns already proven in Hikyaku.
