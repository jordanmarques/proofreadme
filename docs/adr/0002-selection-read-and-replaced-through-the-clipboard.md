# Selection read and replaced through the clipboard

To read the selection, the agent saves the clipboard, simulates Ctrl/Cmd+C, and reads the text. To replace it, it writes the correction, simulates Ctrl/Cmd+V, then restores the saved clipboard (text and image only; other formats are lost). This works in every application that supports copy and paste, and the replacement can be undone with Ctrl/Cmd+Z in that application. Our temporary writes are marked as transient so that clipboard histories ignore them.

## Considered Options

- **Accessibility APIs (UI Automation, AXUIElement)**: they read and write the selection without touching the clipboard, but Electron apps, browsers and terminals expose it poorly or not at all.

## Consequences

- A proofread whose foreground application changed during the request does not paste: the correction is left on the clipboard (not marked transient) and a notification says so.
- On Windows, the agent cannot send keys to an application running elevated; the copy brings nothing back and the proofread silently does nothing.
