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
mod screener_queue;
mod search_bar;
mod settings;
mod settings_panel;
mod sidebar;
mod smart_views;
mod stats;
mod stats_dashboard;

pub use command_palette::{Command, CommandCategory, CommandPalette};
pub use composer::{Composer, ComposerAttachment};
pub use main_window::MainWindow;
pub use message_list::{MessageList, ThreadListItem};
pub use reading_pane::{AttachmentInfo, MessageDetail, ReadingPane, ThreadDetail};
pub use screener_queue::{ScreenerEntry, ScreenerQueue};
pub use search_bar::{SearchBar, SearchOperator, SearchSuggestion};
pub use settings::{SettingsSection, SettingsView};
pub use settings_panel::{SettingsPanel, SettingsTab};
pub use sidebar::{Sidebar, SidebarAccount, SidebarLabel};
pub use smart_views::{
    SmartViewCriteria, SmartViewEntry, SmartViewManager, SmartViewMatch, SmartViewType,
    SmartViewsPanel,
};
pub use stats::{DashboardStats, StatsView, TimeRange};
pub use stats_dashboard::{AiStats, EmailStats, ProductivityStats, StatsDashboard, StatsTimeRange};
