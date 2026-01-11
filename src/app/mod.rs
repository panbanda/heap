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
    margin,
    [
        Quit,
        Compose,
        Reply,
        ReplyAll,
        Forward,
        Archive,
        Trash,
        Star,
        Snooze,
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
        cx.bind_keys([
            // Quit
            KeyBinding::new("cmd-q", Quit, None),
            // Compose
            KeyBinding::new("c", Compose, None),
            // Reply
            KeyBinding::new("r", Reply, None),
            KeyBinding::new("shift-r", ReplyAll, None),
            // Forward
            KeyBinding::new("f", Forward, None),
            // Archive/Delete
            KeyBinding::new("e", Archive, None),
            KeyBinding::new("shift-3", Trash, None),
            // Star/Snooze
            KeyBinding::new("s", Star, None),
            KeyBinding::new("h", Snooze, None),
            // Read state
            KeyBinding::new("u", MarkRead, None),
            KeyBinding::new("shift-u", MarkUnread, None),
            // Navigation
            KeyBinding::new("j", NextMessage, None),
            KeyBinding::new("k", PreviousMessage, None),
            KeyBinding::new("enter", OpenThread, None),
            // Go to views
            KeyBinding::new("g i", GoToInbox, None),
            KeyBinding::new("g s", GoToStarred, None),
            KeyBinding::new("g d", GoToDrafts, None),
            KeyBinding::new("g t", GoToSent, None),
            KeyBinding::new("g a", GoToArchive, None),
            // Command palette and search
            KeyBinding::new("cmd-k", OpenCommandPalette, None),
            KeyBinding::new("/", Search, None),
            // Settings
            KeyBinding::new("cmd-,", OpenSettings, None),
        ]);
    }
}
