# ProofReadMe

ProofReadMe proofreads the text selected in any application and replaces it with its corrected version, on a double press of a key.

## Language

**Trigger**:
Two presses in quick succession of a single modifier key, with no other key in between, which starts a proofread.
_Avoid_: shortcut, hotkey

**Selection**:
The text selected in the foreground application at the moment of the trigger.
_Avoid_: input, highlighted text

**Proofread**:
One full cycle started by a trigger: capture of the selection, request to the provider, then replacement of the selection by the correction. Only one proofread runs at a time.
_Avoid_: correction job, request, check

**Correction**:
The text returned by the provider for a selection, once cleaned, which replaces the selection.
_Avoid_: answer, result, output

**Proofreading prompt**:
The template sent to the provider, which carries the `{{TEXTE}}` placeholder where the selection is inserted.
_Avoid_: system prompt, instructions

**Provider**:
The chat completions endpoint that produces the correction, defined by its base URL, its API key and its model.
_Avoid_: backend, LLM, server

**Agent**:
The ProofReadMe process opened with the session, which listens for the trigger and runs the proofreads.
_Avoid_: service, daemon
