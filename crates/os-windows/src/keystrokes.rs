//! Ctrl+C and Ctrl+V synthesized in the foreground application with `SendInput`. The keyboard
//! hook sees them flagged `LLKHF_INJECTED`, so they never feed the trigger.

use proofreadme_core::{Keystroke, OsError};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_CONTROL,
};

/// Virtual keys of the letters C and V (layout-independent: Windows maps them to the shortcut).
const VK_C: u16 = 0x43;
const VK_V: u16 = 0x56;

/// Ctrl down, the letter down, the letter up, Ctrl up, in one `SendInput`.
pub(crate) fn send(keystroke: Keystroke) -> Result<(), OsError> {
    let letter = match keystroke {
        Keystroke::Copy => VK_C,
        Keystroke::Paste => VK_V,
    };
    let inputs = [key(VK_CONTROL, 0), key(letter, 0), key(letter, KEYEVENTF_KEYUP), key(VK_CONTROL, KEYEVENTF_KEYUP)];
    let size = i32::try_from(size_of::<INPUT>()).map_err(|_| OsError::Keystrokes("INPUT too large".to_owned()))?;
    let count = u32::try_from(inputs.len()).map_err(|_| OsError::Keystrokes("too many inputs".to_owned()))?;
    // SAFETY: `inputs` is an array of `count` initialised INPUT structures that outlives the call,
    // and `size` is the size of one of them.
    let sent = unsafe { SendInput(count, inputs.as_ptr(), size) };
    if sent == count {
        Ok(())
    } else {
        // Blocked by UIPI (an elevated foreground application) or by another program.
        Err(OsError::Keystrokes(format!("SendInput sent {sent} of {count} events: {}", std::io::Error::last_os_error())))
    }
}

fn key(virtual_key: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: virtual_key, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }
}
