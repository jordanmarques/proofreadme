//! The keyboard hook: a listen-only `CGEventTap` on a thread of its own, running its own run loop.
//! It only translates events and reports the permission; the trigger is detected by the agent.

use std::cell::RefCell;
use std::collections::HashSet;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use core_foundation::runloop::CFRunLoop;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CallbackResult, EventField,
};
use proofreadme_core::{Direction, Hook, Key, KeyEvent, Modifier, Permission};

use crate::{INJECTED, permission};

/// How often the hook checks the permission again while it is missing.
const POLL: Duration = Duration::from_secs(2);

/// Starts the keyboard hook on a thread of its own, which lives as long as the process. While the
/// Accessibility permission is missing the hook reports it and checks again every few seconds; once
/// granted, the tap is created without a relaunch. If macOS still refuses the tap, the hook reports
/// that a relaunch is needed.
pub fn listen(hook: Hook) -> io::Result<()> {
    thread::Builder::new().name("keyboard-hook".to_owned()).spawn(move || run(&hook)).map(|_| ())
}

fn run(hook: &Hook) {
    let mut requested = false;
    loop {
        if !permission::granted() {
            hook.permission(Permission::Missing);
            if !requested {
                permission::request();
                requested = true;
            }
            thread::sleep(POLL);
            continue;
        }
        let modifiers = RefCell::new(HashSet::new());
        let tapped = CGEventTap::with_enabled(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
            |_proxy, kind, event| {
                if let Some(key_event) = translate(kind, event, &mut modifiers.borrow_mut()) {
                    hook.key_event(key_event);
                }
                CallbackResult::Keep
            },
            || {
                hook.permission(Permission::Granted);
                CFRunLoop::run_current();
            },
        );
        // The run loop stops when macOS disables the tap (a slow callback, the permission
        // revoked): the loop checks the permission and creates the tap again.
        if tapped.is_err() {
            hook.permission(Permission::RelaunchNeeded);
            thread::sleep(POLL);
        }
    }
}

/// The key event of the agent for an event of the tap. `down` holds the modifier keys held, by
/// keycode, since modifiers come only as `FlagsChanged`.
fn translate(kind: CGEventType, event: &CGEvent, down: &mut HashSet<i64>) -> Option<KeyEvent> {
    let (key, direction) = match kind {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            CFRunLoop::get_current().stop();
            return None;
        }
        CGEventType::KeyDown => (Key::Other(keycode(event)), Direction::Down),
        CGEventType::KeyUp => (Key::Other(keycode(event)), Direction::Up),
        CGEventType::FlagsChanged => {
            let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            let Some((modifier, flag)) = modifier(code) else {
                // Caps Lock, Fn: another key between two presses of the trigger.
                return Some(key_event(Key::Other(keycode(event)), Direction::Down, event));
            };
            // Both keys of a modifier share its flag: a key already down goes up, whatever the flag.
            let direction = if down.remove(&code) || !event.get_flags().contains(flag) {
                Direction::Up
            } else {
                down.insert(code);
                Direction::Down
            };
            (Key::Modifier(modifier), direction)
        }
        _ => return None,
    };
    Some(key_event(key, direction, event))
}

fn key_event(key: Key, direction: Direction, event: &CGEvent) -> KeyEvent {
    let injected = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA) == INJECTED;
    KeyEvent { key, direction, at: Instant::now(), injected }
}

fn keycode(event: &CGEvent) -> u32 {
    u32::try_from(event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)).unwrap_or(u32::MAX)
}

/// The modifier of a keycode (`kVK_*` of `Events.h`), with its flag.
fn modifier(code: i64) -> Option<(Modifier, CGEventFlags)> {
    match code {
        0x37 | 0x36 => Some((Modifier::Cmd, CGEventFlags::CGEventFlagCommand)),
        0x38 | 0x3C => Some((Modifier::Shift, CGEventFlags::CGEventFlagShift)),
        0x3A | 0x3D => Some((Modifier::Alt, CGEventFlags::CGEventFlagAlternate)),
        0x3B | 0x3E => Some((Modifier::Ctrl, CGEventFlags::CGEventFlagControl)),
        _ => None,
    }
}
