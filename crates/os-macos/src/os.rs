//! The pasteboard, the keystrokes and the frontmost application, behind the OS port of the agent.

use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSPasteboard, NSWorkspace};
use objc2_foundation::{NSString, ns_string};
use proofreadme_core::{ClipboardContent, Image, Keystroke, Mark, Os, OsError};

use crate::INJECTED;

/// The type of plain text on the pasteboard (`NSPasteboardTypeString`).
const TEXT: &str = "public.utf8-plain-text";
/// The marker of a temporary write, that clipboard managers skip (nspasteboard.org). Not
/// `ConcealedType`: it would mark the restored clipboard of the user as a password.
const TRANSIENT: &str = "org.nspasteboard.TransientType";

/// `kVK_ANSI_C` and `kVK_ANSI_V`: the keys by position on the keyboard.
const KEY_C: CGKeyCode = 8;
const KEY_V: CGKeyCode = 9;

/// The OS port on macOS.
#[derive(Debug, Default)]
pub struct MacOs;

impl MacOs {
    /// The OS port on macOS.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

fn clipboard() -> Result<Clipboard, OsError> {
    Clipboard::new().map_err(|error| OsError::Clipboard(error.to_string()))
}

/// The content, or `None` if the pasteboard holds none of this kind.
fn optional<T>(read: Result<T, arboard::Error>) -> Result<Option<T>, OsError> {
    match read {
        Ok(value) => Ok(Some(value)),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(error) => Err(OsError::Clipboard(error.to_string())),
    }
}

impl Os for MacOs {
    fn read_clipboard(&mut self) -> Result<ClipboardContent, OsError> {
        let mut clipboard = clipboard()?;
        let text = optional(clipboard.get_text())?;
        let image = optional(clipboard.get_image())?.map(|image| Image { width: image.width, height: image.height, rgba: image.bytes.into_owned() });
        Ok(ClipboardContent { text, image })
    }

    fn write_clipboard(&mut self, content: &ClipboardContent, mark: Mark) -> Result<(), OsError> {
        let pasteboard = NSPasteboard::generalPasteboard();
        if let Some(image) = &content.image {
            // arboard clears the pasteboard and writes the image; the text and the marker join it.
            // Its `exclude_from_history` is not used: it writes `ConcealedType`.
            let data = ImageData { width: image.width, height: image.height, bytes: Cow::Borrowed(&image.rgba) };
            clipboard()?.set_image(data).map_err(|error| OsError::Clipboard(error.to_string()))?;
        } else {
            pasteboard.clearContents();
        }
        if mark == Mark::Transient && (content.text.is_some() || content.image.is_some()) {
            pasteboard.setString_forType(ns_string!(""), &NSString::from_str(TRANSIENT));
        }
        if let Some(text) = &content.text
            && !pasteboard.setString_forType(&NSString::from_str(text), &NSString::from_str(TEXT))
        {
            return Err(OsError::Clipboard("the pasteboard refused the text".to_owned()));
        }
        Ok(())
    }

    fn clipboard_change_count(&mut self) -> u64 {
        NSPasteboard::generalPasteboard().changeCount() as u64
    }

    fn synthesize(&mut self, keystroke: Keystroke) -> Result<(), OsError> {
        let key = match keystroke {
            Keystroke::Copy => KEY_C,
            Keystroke::Paste => KEY_V,
        };
        let refused = || OsError::Keystrokes("macOS refused to create the keystroke".to_owned());
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|()| refused())?;
        for down in [true, false] {
            let event = CGEvent::new_keyboard_event(source.clone(), key, down).map_err(|()| refused())?;
            // Cmd alone, whatever modifier the user still holds.
            event.set_flags(CGEventFlags::CGEventFlagCommand);
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, INJECTED);
            event.post(CGEventTapLocation::HID);
        }
        Ok(())
    }

    fn foreground_application(&mut self) -> Option<String> {
        let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(application.processIdentifier().to_string())
    }
}
