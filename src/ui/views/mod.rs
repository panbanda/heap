//! Application views.
//!
//! Views are the top-level UI components that compose together to form
//! the application interface. Each view typically manages its own state
//! and handles user interactions.

mod command_palette;
mod composer;
mod main_window;
mod message_list;
mod reading_pane;
mod settings;
mod sidebar;
mod stats;

pub use command_palette::{Command, CommandCategory, CommandPalette};
pub use composer::{Composer, ComposerAttachment};
pub use main_window::MainWindow;
pub use message_list::{MessageList, ThreadListItem};
pub use reading_pane::{AttachmentInfo, MessageDetail, ReadingPane, ThreadDetail};
pub use settings::{SettingsSection, SettingsView};
pub use sidebar::{Sidebar, SidebarAccount, SidebarLabel};
pub use stats::{DashboardStats, StatsView, TimeRange};
