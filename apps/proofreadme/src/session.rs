//! The presence of the agent in the session: the window hides instead of closing, comes back to
//! the foreground when the agent needs the user or when asked to, and only closes for good on the
//! request to quit.
//!
//! What the window asks of the OS goes through the egui viewport commands (`Visible`, `Focus`,
//! `CancelClose`, `Close`). eframe also handles them with the window hidden: it then calls
//! `App::logic` without drawing, at each requested wakeup.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock};

use egui::ViewportCommand;

/// What is asked of the agent from outside its window: the menu of the session icon, or a second
/// launch of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Show the window in the foreground.
    Open,
    /// Show the window in the foreground, on the settings screen.
    OpenSettings,
    /// Cancel the running proofread.
    Cancel,
    /// Quit the agent.
    Quit,
}

/// The means to send a request to the window from any thread, and to wake it so that it handles it
/// even hidden.
#[derive(Debug, Clone)]
pub struct RequestSender {
    channel: Sender<Request>,
    context: Arc<OnceLock<egui::Context>>,
}

impl RequestSender {
    /// Creates the sender and the end of the channel to give to the window
    /// ([`crate::Window::with_requests`]).
    #[must_use]
    pub fn new() -> (Self, Receiver<Request>) {
        let (channel, requests) = mpsc::channel();
        (Self { channel, context: Arc::default() }, requests)
    }

    /// Gives the egui context to wake at each request. Without it, a request waits for the next
    /// frame.
    pub fn wake(&self, context: egui::Context) {
        // A second call has no effect: the first context stays.
        let _ = self.context.set(context);
    }

    /// Sends a request to the window.
    pub fn send(&self, request: Request) {
        // If the window no longer exists, the agent is quitting: the request has no object.
        let _ = self.channel.send(request);
        if let Some(context) = self.context.get() {
            context.request_repaint();
        }
    }
}

/// The presence of the window on screen.
pub(crate) struct Presence {
    requests: Option<Receiver<Request>>,
    /// True if the agent was launched in the background: the window hides from its first frame.
    in_background: bool,
    first_frame: bool,
    /// True once the request to quit has been received: the close is no longer cancelled.
    quitting: bool,
    /// True since the window asked to be hidden, until it asks to come back.
    hidden: bool,
    /// True if the agent has no session icon: the window then never hides, since nothing would
    /// allow finding it again or quitting.
    no_icon: bool,
}

impl Presence {
    pub(crate) fn new() -> Self {
        Self { requests: None, in_background: false, first_frame: true, quitting: false, hidden: false, no_icon: false }
    }

    pub(crate) fn listen(&mut self, requests: Receiver<Request>) {
        self.requests = Some(requests);
    }

    pub(crate) fn start_in_background(&mut self) {
        self.in_background = true;
    }

    pub(crate) fn without_session_icon(&mut self) {
        self.no_icon = true;
    }

    /// True if the agent has no session icon.
    pub(crate) fn has_no_icon(&self) -> bool {
        self.no_icon
    }

    /// The requests that arrived since the last frame.
    pub(crate) fn received_requests(&self) -> Vec<Request> {
        self.requests.as_ref().map(|requests| requests.try_iter().collect()).unwrap_or_default()
    }

    pub(crate) fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Hides the window: the agent goes on in the background.
    fn hide(&mut self, context: &egui::Context) {
        self.hidden = true;
        context.send_viewport_cmd(ViewportCommand::Visible(false));
    }

    /// Shows the window and brings it to the foreground.
    pub(crate) fn show(&mut self, context: &egui::Context) {
        self.hidden = false;
        context.send_viewport_cmd(ViewportCommand::Visible(true));
        context.send_viewport_cmd(ViewportCommand::Minimized(false));
        context.send_viewport_cmd(ViewportCommand::Focus);
        context.request_repaint();
    }

    /// Closes the window for good, which ends the agent.
    pub(crate) fn quit(&mut self, context: &egui::Context) {
        self.quitting = true;
        context.send_viewport_cmd(ViewportCommand::Close);
        // The command only sets the close request: it takes one more frame for eframe to notice
        // it.
        context.request_repaint();
    }

    /// To be called at each frame: hides the window when the user closes it (or quits, without a
    /// session icon), shows it when the agent needs the user, and hides it at the first frame of a
    /// launch in the background otherwise.
    pub(crate) fn update(&mut self, context: &egui::Context, needs_the_user: bool) {
        if close_requested(context) && !self.quitting {
            if self.no_icon {
                // Without a session icon, closing the window is the only way to quit the agent.
                self.quitting = true;
            } else {
                context.send_viewport_cmd(ViewportCommand::CancelClose);
                self.hide(context);
            }
        }
        if needs_the_user {
            self.show(context);
        } else if self.first_frame && self.in_background && !self.no_icon {
            self.hide(context);
        }
        self.first_frame = false;
    }
}

/// True if the close of the window has just been requested (close box, Cmd+W, Alt+F4).
fn close_requested(context: &egui::Context) -> bool {
    context.input(|input| input.viewport().close_requested())
}
