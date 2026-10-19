//! The identity of the foreground application: its process and its window.

use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// `"<process id>/<window handle>"` of the foreground window, `None` when no window has the focus.
pub(crate) fn identity() -> Option<String> {
    // SAFETY: no argument; returns a handle, possibly null, that is only read as a number below.
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut process = 0u32;
    // SAFETY: `process` is a valid, writable u32 for the duration of the call; a window that
    // vanished meanwhile only makes the call return 0.
    let thread = unsafe { GetWindowThreadProcessId(window, &raw mut process) };
    (thread != 0).then(|| format!("{process}/{:x}", window.addr()))
}
