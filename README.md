# ProofReadMe

ProofReadMe proofreads the text selected in any application and replaces it with its corrected
version, on a double press of a modifier key (Ctrl by default on Windows, Cmd on macOS). The
replacement goes through the clipboard (ADR 0002): Ctrl/Cmd+Z in the application undoes it, and
the clipboard holds what it held before (text and image).

## Known limits

- **Windows, elevated applications.** An application running as administrator receives neither
  the keystrokes nor the copy of a non-elevated agent: there, the copy brings nothing back and the
  proofread silently does nothing. Run the application without elevation to proofread in it.
