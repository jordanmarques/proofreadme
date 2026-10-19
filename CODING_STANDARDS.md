# Coding standards

Read during review. These are the judgement calls no tool checks. Formatting, the never-crash lints, `#![forbid(unsafe_code)]` (lifted only in `crates/os-windows`, each `unsafe` block with its `SAFETY` comment), the layer boundaries and `Harness::run` in the app are enforced by `cargo xtask ci`.

## The text of the user

ProofReadMe handles text the user is writing, in any application, and a damaged or leaked text cannot be taken back.

- **The text is never logged.** The selection, the correction and the API key never reach the agent log, a notification, an error message or a panic message. The log of a failed proofread carries its cause, the model and the duration, nothing else.
- **A failure never touches the selection.** A proofread that fails (network, refused key, provider error, timeout, empty correction) leaves the selection as it was and says why in a notification.
- **The clipboard is given back.** Every path through a proofread restores the clipboard saved at its start (text and image), except the focus loss of ADR 0002, where the correction is left on it on purpose.
- **The correction lands where the selection was, or nowhere.** The agent pastes only into the foreground application the selection was read from.
- **The API key stays in the keychain of the OS.** It is never written to the config file.

## Threads

- **The UI thread does no network I/O.** `Window::logic` and `Window::ui` read the agent's state; the request to the provider runs on an agent thread and reports back through the state.
- **One proofread at a time.** A trigger during a running proofread is ignored; nothing starts a second one in parallel.
- **A waiting test waits on a signal.** Tests reach an intermediate state through an observer or a bounded wait, with no `sleep`.

## Errors

- **Typed errors.** Production code returns an error type (`thiserror`), not `String`. `xtask` is exempt.
- **No silent failure.** An error that does not stop the agent is surfaced as a notification, a refusal in the state, or a line in the agent log. The only silent ending is a trigger with nothing selected.
- **Hostile numbers.** Durations, sizes and timeouts coming from the config, the OS or the provider go through checked or saturating arithmetic.

## Boundaries

- **`crates/kairo` knows nothing about ProofReadMe**, including in doc examples and gallery text. It is a copy of the Kairo crate of Hikyaku (ADR 0001) and evolves on its own.
- **`crates/core` holds every rule.** The trigger detector, the proofread flow, the cleaning of the correction and the settings rules live in the agent. The window renders state and sends commands; a decision made in the window belongs in the agent.
- **Platform code stays minimal.** The keyboard hook, keystroke synthesis, the clipboard, the foreground application and notifications are thin adapters behind the port of the agent: no rule lives in them. `cfg(target_os = "macos")` and `cfg(windows)` code is only proved by the CI job of the other system; keep it small.

## Language and vocabulary

- Everything in the repository is in English. The `{{TEXTE}}` placeholder, the default proofreading prompt and `TEST_SENTENCE` (French and English, to exercise both rules of the default prompt) are the agreed exceptions.
- Domain concepts use the terms of `GLOSSARY.md` (trigger, selection, proofread, correction, proofreading prompt, provider, agent), in identifiers, tests, docs and UI labels. A synonym listed under `_Avoid_` (hotkey, answer, result, backend, LLM, daemon…) is a finding.
- UI labels follow Kairo: uppercase English, then short Japanese dressing (`AGENT 校正`).

## Tests

- Tests go through the two agreed seams: the headless agent with a fake OS port and a real local HTTP server standing in for the provider, and the window under `egui_kittest`. A test of an internal function needs a reason the seams cannot give.
- Tests assert external effects: what was pasted, what the clipboard holds at the end, the notifications sent, the state of the agent, the request received by the provider, and the log.
- Platform adapters are not unit tested; the CI build on macOS and Windows proves them.
