//! Application state and lifecycle management.
//!
//! This module contains:
//! - Application state management (state.rs)
//! - Action definitions (inline via gpui::actions!)
//! - Event bus for cross-component communication (events.rs)
//! - Keybinding registration

pub mod events;
pub mod state;

pub use events::{AppEvent, EventBus};
pub use state::{
    AiStatus, AppState, ComposerMode, ComposerState, MessageListState, ReadingPaneState,
    SyncStatus, ViewType,
};

use anyhow::Result;
use gpui::{actions, AppContext, Application, KeyBinding, WindowOptions};

use crate::ui::MainWindow;

// Define application actions
actions!(
    heap,
    [
        Quit,
        Dismiss,
        Undo,
        Compose,
        Reply,
        ReplyAll,
        Forward,
        Archive,
        Trash,
        Star,
        Snooze,
        ApplyLabel,
        MarkRead,
        MarkUnread,
        NextMessage,
        PreviousMessage,
        OpenThread,
        GoToInbox,
        GoToStarred,
        GoToDrafts,
        GoToSent,
        GoToArchive,
        GoToScreener,
        GoToStats,
        ScreenerApprove,
        ScreenerReject,
        OpenCommandPalette,
        Search,
        ToggleTheme,
        OpenSettings,
    ]
);

/// Main application entry point
pub struct App;

impl App {
    /// Run the application
    pub fn run() -> Result<()> {
        Application::new().run(|cx: &mut gpui::App| {
            Self::register_keybindings(cx);

            cx.open_window(WindowOptions::default(), |window, cx| {
                cx.new(|cx| MainWindow::new(window, cx))
            })
            .expect("Failed to open window");
        });

        Ok(())
    }

    /// Register global keybindings
    fn register_keybindings(cx: &mut gpui::App) {
        // Context for single-letter keybindings that should not fire during text input
        let email_ctx = Some("EmailActions");

        cx.bind_keys([
            // Quit and dismiss - global, always available
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("escape", Dismiss, None),
            // Single-letter keybindings - only active in EmailActions context
            KeyBinding::new("z", Undo, email_ctx),
            KeyBinding::new("c", Compose, email_ctx),
            KeyBinding::new("r", Reply, email_ctx),
            KeyBinding::new("shift-r", ReplyAll, email_ctx),
            KeyBinding::new("f", Forward, email_ctx),
            KeyBinding::new("e", Archive, email_ctx),
            KeyBinding::new("shift-3", Trash, email_ctx),
            KeyBinding::new("s", Star, email_ctx),
            KeyBinding::new("h", Snooze, email_ctx),
            KeyBinding::new("l", ApplyLabel, email_ctx),
            KeyBinding::new("u", MarkRead, email_ctx),
            KeyBinding::new("shift-u", MarkUnread, email_ctx),
            KeyBinding::new("j", NextMessage, email_ctx),
            KeyBinding::new("k", PreviousMessage, email_ctx),
            KeyBinding::new("enter", OpenThread, email_ctx),
            KeyBinding::new("g i", GoToInbox, email_ctx),
            KeyBinding::new("g s", GoToStarred, email_ctx),
            KeyBinding::new("g d", GoToDrafts, email_ctx),
            KeyBinding::new("g t", GoToSent, email_ctx),
            KeyBinding::new("g a", GoToArchive, email_ctx),
            KeyBinding::new("g c", GoToScreener, email_ctx),
            KeyBinding::new("g p", GoToStats, email_ctx),
            KeyBinding::new("a", ScreenerApprove, email_ctx),
            KeyBinding::new("x", ScreenerReject, email_ctx),
            KeyBinding::new("/", Search, email_ctx),
            // Cmd-key bindings - global, always available
            KeyBinding::new("cmd-k", OpenCommandPalette, None),
            KeyBinding::new("cmd-,", OpenSettings, None),
        ]);
    }
}
