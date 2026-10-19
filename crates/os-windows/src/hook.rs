//! The low-level keyboard hook (`WH_KEYBOARD_LL`, ADR 0003). It runs on a thread of its own that
//! pumps its messages; its callback only turns the event into a [`KeyEvent`] and passes it on
//! through a channel, so that it always returns at once (Windows removes a hook that is too slow).
//! A second thread hands the events to the sink.

use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Instant;

use proofreadme_core::{Direction, Key, KeyEvent, Modifier};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Why the keyboard hook could not be installed.
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    /// The hook is installed once per process.
    #[error("the keyboard hook is already installed")]
    AlreadyInstalled,
    /// A thread of the hook could not be started.
    #[error("the keyboard hook thread could not start: {0}")]
    Thread(#[source] std::io::Error),
    /// Windows refused the hook.
    #[error("Windows refused the keyboard hook: {0}")]
    Refused(#[source] std::io::Error),
}

/// Where the callback of the hook, which has no context of its own, sends the key events.
static EVENTS: OnceLock<Sender<KeyEvent>> = OnceLock::new();

/// Installs the keyboard hook for the life of the process: every key event of the session, the
/// injected ones flagged as such, reaches `sink` on a thread of the hook.
pub fn listen(sink: impl Fn(KeyEvent) + Send + 'static) -> Result<(), HookError> {
    let (sender, events) = mpsc::channel();
    EVENTS.set(sender).map_err(|_| HookError::AlreadyInstalled)?;
    thread::Builder::new().name("proofreadme-keys".to_owned()).spawn(move || events.into_iter().for_each(sink)).map_err(HookError::Thread)?;
    let (installed, outcome) = mpsc::sync_channel(1);
    thread::Builder::new().name("proofreadme-hook".to_owned()).spawn(move || pump(&installed)).map_err(HookError::Thread)?;
    outcome.recv().map_err(|_| HookError::Refused(std::io::Error::other("the hook thread ended")))?
}

/// Installs the hook on this thread and pumps its messages, forever.
fn pump(installed: &mpsc::SyncSender<Result<(), HookError>>) {
    // SAFETY: a null name asks for the module of the running executable; no pointer is kept.
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    // SAFETY: `callback` has the signature of a HOOKPROC and lives for the whole process; the
    // module is the executable that contains it; 0 hooks every thread of the session.
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(callback), module, 0) };
    if hook.is_null() {
        let _ = installed.send(Err(HookError::Refused(std::io::Error::last_os_error())));
        return;
    }
    let _ = installed.send(Ok(()));
    let mut message = MSG::default();
    // The callback of a low-level hook runs inside this loop, on this thread. `GetMessageW`
    // returns 0 on WM_QUIT and -1 on error: both end the loop.
    // SAFETY: `message` is a valid, writable MSG for each call; a null window takes the messages
    // of the thread.
    while unsafe { GetMessageW(&raw mut message, std::ptr::null_mut(), 0, 0) } > 0 {}
    // SAFETY: `hook` is the handle returned above, not unhooked yet.
    unsafe { UnhookWindowsHookEx(hook) };
}

/// The callback of the hook: passes the event on, then always to the next hook.
unsafe extern "system" fn callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION.cast_signed() {
        let direction = match u32::try_from(wparam) {
            Ok(WM_KEYDOWN | WM_SYSKEYDOWN) => Some(Direction::Down),
            Ok(WM_KEYUP | WM_SYSKEYUP) => Some(Direction::Up),
            _ => None,
        };
        if let (Some(direction), Some(events)) = (direction, EVENTS.get()) {
            // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, `lparam` points to a KBDLLHOOKSTRUCT
            // valid for the duration of the callback; it is copied out at once.
            let event = unsafe { *(lparam as *const KBDLLHOOKSTRUCT) };
            let _ = events.send(KeyEvent { key: key(event.vkCode), direction, at: Instant::now(), injected: event.flags & LLKHF_INJECTED != 0 });
        }
    }
    // SAFETY: the arguments are those received; the hook handle is ignored by Windows.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// The key of a virtual key code: left, right and generic modifiers alike.
fn key(virtual_key: u32) -> Key {
    match virtual_key {
        0x10 | 0xA0 | 0xA1 => Key::Modifier(Modifier::Shift),
        0x11 | 0xA2 | 0xA3 => Key::Modifier(Modifier::Ctrl),
        0x12 | 0xA4 | 0xA5 => Key::Modifier(Modifier::Alt),
        0x5B | 0x5C => Key::Modifier(Modifier::Cmd),
        other => Key::Other(other),
    }
}
