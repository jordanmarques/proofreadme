# Double press of a modifier through a low-level keyboard hook

The trigger is two presses of a single modifier key (chosen in the settings; Ctrl by default on Windows, Cmd on macOS) within 300 ms, with no other key in between. It fires on the second release, so that the modifier is no longer held when Ctrl/Cmd+C is simulated. Detecting it needs a low-level keyboard hook, which on macOS requires the Accessibility (or Input Monitoring) permission; while it is missing, the window explains it and links to the right pane of System Settings.

## Considered Options

- **A classic global shortcut (Ctrl+Alt+P)**: no hook and no permission for listening, but slower to type than a double press.
- **A regular key pressed twice**: types its character into the application.
