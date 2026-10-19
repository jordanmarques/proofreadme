//! The session icon: in the menu bar of macOS, in the notification area of Windows, with its menu
//! (Settings, Cancel, Quit). It goes through the `tray-icon` crate.
//!
//! The window only knows the [`IconPort`] port: it gives the icon its look whenever the state of
//! the agent changes it. The drawing is that of the `proofreadme-icon` crate.

use proofreadme_core::{AgentLog, IconState};
use proofreadme_icon::Look;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::session::{Request, RequestSender};

const SETTINGS: &str = "settings";
const CANCEL: &str = "cancel";
const QUIT: &str = "quit";

/// The Kairo command orange (`--k-orange-500`), for the notification area of Windows: that of the
/// application icon.
const ORANGE: [u8; 3] = proofreadme_icon::ORANGE;
/// On macOS the icon is a template: only its transparency counts, the system tints it according to
/// the menu bar (light or dark).
const BLACK: [u8; 3] = [0, 0, 0];

/// The port through which the window shows the state of the agent in the session icon.
pub trait IconPort {
    /// Gives the icon the look of `icon`, and enables its Cancel item if `can_cancel`.
    fn show(&mut self, icon: IconState, can_cancel: bool);
}

/// What prevents installing the session icon.
#[derive(Debug, thiserror::Error)]
pub enum IconError {
    /// The system refused the menu of the icon.
    #[error("the icon menu was refused: {0}")]
    Menu(#[from] tray_icon::menu::Error),
    /// The drawing of the icon is not a valid image.
    #[error("the icon image is invalid: {0}")]
    Image(#[from] tray_icon::BadIcon),
    /// The system refused the icon.
    #[error("the icon was refused: {0}")]
    Icon(#[from] tray_icon::Error),
}

/// The installed session icon. It goes away when this value is dropped.
pub struct SessionIcon {
    icon: TrayIcon,
    cancel: MenuItem,
    /// Where a look the icon cannot take is told.
    log: AgentLog,
}

/// The side and the colour of the icon on this OS.
fn drawing() -> (u32, [u8; 3]) {
    if cfg!(windows) { (32, ORANGE) } else { (44, BLACK) }
}

/// The image of the icon for a state of the agent.
fn image(icon: IconState) -> Result<Icon, IconError> {
    let look = match icon {
        IconState::Idle => Look::Idle,
        IconState::Running => Look::Running,
        IconState::Alert => Look::Alert,
    };
    let (side, color) = drawing();
    Ok(Icon::from_rgba(proofreadme_icon::session(side, color, look), side, side)?)
}

impl SessionIcon {
    /// Installs the icon and its menu; each choice of the menu is sent to the window by `sender`,
    /// and a look the icon later cannot take is told in `log`.
    ///
    /// To be called on the main thread, once the event loop has started (with eframe: in the
    /// creation closure of the application), as `tray-icon` requires on macOS.
    ///
    /// # Errors
    ///
    /// The cause, if the system refuses the icon or its menu.
    pub fn install(sender: &RequestSender, log: AgentLog) -> Result<Self, IconError> {
        let cancel = MenuItem::with_id(CANCEL, "Cancel", false, None);
        let menu = Menu::new();
        menu.append_items(&[
            &MenuItem::with_id(SETTINGS, "Settings…", true, None),
            &cancel,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(QUIT, "Quit ProofReadMe", true, None),
        ])?;

        let on_windows = cfg!(windows);
        let icon = TrayIconBuilder::new()
            .with_tooltip(crate::NAME)
            .with_menu(Box::new(menu))
            // On Windows the left click opens the window and the right click the menu, as is
            // customary in the notification area; on macOS any click drops the menu down.
            .with_menu_on_left_click(!on_windows);
        let image = image(IconState::Idle)?;
        // On macOS the icon is a template, which the system tints according to the menu bar.
        #[cfg(target_os = "macos")]
        let icon = icon.with_icon_templated(image);
        #[cfg(not(target_os = "macos"))]
        let icon = icon.with_icon(image);
        let icon = icon.build()?;

        let for_the_menu = sender.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
            SETTINGS => for_the_menu.send(Request::OpenSettings),
            CANCEL => for_the_menu.send(Request::Cancel),
            QUIT => for_the_menu.send(Request::Quit),
            _ => {}
        }));
        let for_the_click = sender.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let left_click = matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. });
            if on_windows && left_click {
                for_the_click.send(Request::Open);
            }
        }));
        Ok(Self { icon, cancel, log })
    }

    fn set_look(&self, icon: IconState) -> Result<(), IconError> {
        let image = image(icon)?;
        #[cfg(target_os = "macos")]
        self.icon.set_icon_templated(Some(image))?;
        #[cfg(not(target_os = "macos"))]
        self.icon.set_icon(Some(image))?;
        Ok(())
    }
}

impl IconPort for SessionIcon {
    fn show(&mut self, icon: IconState, can_cancel: bool) {
        self.cancel.set_enabled(can_cancel);
        // The icon keeps its previous look: the window says the same thing.
        if let Err(error) = self.set_look(icon) {
            self.log.note(&format!("the session icon could not change its look: {error}"));
        }
    }
}
