//! Main application window
//!
//! Integrates sidebar, message list, and reading pane with full interactivity.

use std::collections::HashSet;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, ClickEvent, Context, CursorStyle, FocusHandle,
    Focusable, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::ui::components::{KeyInputResult, TextBuffer};

/// Command palette commands (label, shortcut).
const COMMANDS: &[(&str, &str)] = &[
    ("Go to Inbox", "g i"),
    ("Go to Starred", "g s"),
    ("Go to Sent", "g t"),
    ("Go to Drafts", "g d"),
    ("Go to Archive", "g a"),
    ("Go to New Senders", "g c"),
    ("Go to Statistics", "g p"),
    ("Approve Sender", "a"),
    ("Reject Sender", "x"),
    ("Compose", "c"),
    ("Reply", "r"),
    ("Reply All", "R"),
    ("Forward", "f"),
    ("Archive", "e"),
    ("Trash", "#"),
    ("Star", "s"),
    ("Snooze", "h"),
    ("Apply Labels", "l"),
    ("Mark Read", "u"),
    ("Mark Unread", "U"),
    ("Undo", "z"),
    ("Settings", "Cmd+,"),
    ("Search", "/"),
];

use crate::app::{
    ApplyLabel, Archive, Compose, Dismiss, Forward, GoToArchive, GoToDrafts, GoToInbox,
    GoToScreener, GoToSent, GoToStarred, GoToStats, MarkRead, MarkUnread, NextMessage,
    OpenCommandPalette, OpenSettings, PreviousMessage, Reply, ReplyAll, ScreenerApprove,
    ScreenerReject, Search, Snooze, Star, Trash, Undo, ViewType,
};
use crate::domain::{EmailId, LabelId, ScreenerAction, SenderType, ThreadId};
use crate::services::SnoozeDuration;
use crate::ui::theme::Theme;
use crate::ui::views::{ScreenerEntry, StatsTimeRange};

/// Active overlay state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveOverlay {
    #[default]
    None,
    CommandPalette,
    Search,
    Settings,
    Composer,
    AccountSetup,
    SnoozePicker,
    LabelPicker,
}

/// Account setup mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccountSetupMode {
    #[default]
    Selection,
    Gmail,
    Imap,
}

/// Active field in IMAP setup form
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImapField {
    #[default]
    ImapServer,
    ImapPort,
    SmtpServer,
    SmtpPort,
    Username,
    Password,
}

/// Active field in the composer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComposerField {
    #[default]
    To,
    Cc,
    Bcc,
    Subject,
    Body,
}

/// Active tab in the settings panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Accounts,
    AiFeatures,
    KeyboardShortcuts,
    Appearance,
}

/// Theme mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

/// Font size selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Display density selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayDensity {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

/// AI provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiProvider {
    #[default]
    Ollama,
    OpenAi,
    Anthropic,
}

/// General toggle setting identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneralToggle {
    DesktopNotifications,
    SoundAlerts,
    BadgeCount,
    LaunchAtStartup,
    ShowInMenuBar,
    AutoArchiveAfterReply,
    MarkAsReadWhenOpened,
    ShowConversationView,
}

/// AI toggle setting identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiToggle {
    SmartCompose,
    EmailSummarization,
    PriorityInbox,
    SenderCategorization,
}

/// An action that can be undone
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used when undo is fully implemented
pub enum UndoableAction {
    /// Thread was archived (contains thread id, previous view)
    Archive {
        thread_id: ThreadId,
        from_view: ViewType,
    },
    /// Thread was trashed (contains thread id, previous view)
    Trash {
        thread_id: ThreadId,
        from_view: ViewType,
    },
    /// Thread was starred (contains thread id)
    Star { thread_id: ThreadId },
    /// Thread was unstarred (contains thread id)
    Unstar { thread_id: ThreadId },
    /// Thread was marked read (contains thread id)
    MarkRead { thread_id: ThreadId },
    /// Thread was marked unread (contains thread id)
    MarkUnread { thread_id: ThreadId },
    /// Thread was snoozed (contains thread id)
    Snooze { thread_id: ThreadId },
}

impl UndoableAction {
    fn description(&self) -> &'static str {
        match self {
            UndoableAction::Archive { .. } => "Archived",
            UndoableAction::Trash { .. } => "Moved to Trash",
            UndoableAction::Star { .. } => "Starred",
            UndoableAction::Unstar { .. } => "Unstarred",
            UndoableAction::MarkRead { .. } => "Marked as read",
            UndoableAction::MarkUnread { .. } => "Marked as unread",
            UndoableAction::Snooze { .. } => "Snoozed",
        }
    }

    fn undo_description(&self) -> &'static str {
        match self {
            UndoableAction::Archive { .. } => "Unarchived",
            UndoableAction::Trash { .. } => "Restored from Trash",
            UndoableAction::Star { .. } => "Unstarred",
            UndoableAction::Unstar { .. } => "Re-starred",
            UndoableAction::MarkRead { .. } => "Marked as unread",
            UndoableAction::MarkUnread { .. } => "Marked as read",
            UndoableAction::Snooze { .. } => "Unsnooze",
        }
    }
}

/// Toast notification state
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub can_undo: bool,
    pub created_at: std::time::Instant,
}

impl Toast {
    fn new(message: impl Into<String>, can_undo: bool) -> Self {
        Self {
            message: message.into(),
            can_undo,
            created_at: std::time::Instant::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > std::time::Duration::from_secs(5)
    }
}

/// Main window view containing the primary application layout
pub struct MainWindow {
    theme: Theme,
    focus_handle: FocusHandle,

    // App state
    current_view: ViewType,

    // Overlay state
    active_overlay: ActiveOverlay,
    command_palette_buffer: TextBuffer,
    command_palette_selected: usize,
    search_buffer: TextBuffer,

    // Composer state
    composer_to: TextBuffer,
    composer_cc: TextBuffer,
    composer_bcc: TextBuffer,
    composer_subject: TextBuffer,
    composer_body: TextBuffer,
    composer_active_field: ComposerField,
    composer_show_cc: bool,
    composer_show_bcc: bool,

    // Settings state
    settings_active_tab: SettingsTab,

    // General settings
    settings_desktop_notifications: bool,
    settings_sound_alerts: bool,
    settings_badge_count: bool,
    settings_launch_at_startup: bool,
    settings_show_in_menu_bar: bool,
    settings_auto_archive_after_reply: bool,
    settings_mark_as_read_when_opened: bool,
    settings_show_conversation_view: bool,

    // AI settings
    settings_smart_compose: bool,
    settings_email_summarization: bool,
    settings_priority_inbox: bool,
    settings_sender_categorization: bool,
    settings_ai_provider: AiProvider,

    // Appearance settings
    settings_theme_mode: ThemeMode,
    settings_font_size: FontSize,
    settings_display_density: DisplayDensity,

    // Account setup state
    account_setup_mode: AccountSetupMode,
    imap_active_field: ImapField,
    imap_server: TextBuffer,
    imap_port: TextBuffer,
    imap_username: TextBuffer,
    imap_password: TextBuffer,
    smtp_server: TextBuffer,
    smtp_port: TextBuffer,

    // Sidebar state
    sidebar_accounts: Vec<SidebarAccount>,
    sidebar_labels: Vec<SidebarLabel>,
    sidebar_collapsed_sections: HashSet<String>,

    // Message list state
    threads: Vec<ThreadListItem>,
    selected_thread_id: Option<ThreadId>,
    focused_index: usize,

    // Reading pane state
    current_thread: Option<ThreadDetail>,
    expanded_messages: HashSet<EmailId>,

    // Status bar state
    is_syncing: bool,
    sync_progress: u8,
    is_offline: bool,
    ai_status: Option<String>,
    last_sync: Option<String>,

    // Undo system
    undo_stack: Vec<UndoableAction>,
    toast: Option<Toast>,

    // Screener state
    screener_entries: Vec<ScreenerEntry>,
    screener_selected_index: usize,

    // Stats state
    stats_email_received: u32,
    stats_email_sent: u32,
    stats_email_archived: u32,
    stats_email_starred: u32,
    stats_avg_response_mins: u32,
    stats_inbox_zero_count: u32,
    stats_sessions: u32,
    stats_time_in_app_mins: u32,
    stats_ai_summaries: u32,
    stats_ai_drafts: u32,
    stats_ai_searches: u32,
    stats_ai_tokens: u64,
    stats_time_range: StatsTimeRange,

    // Snooze picker state
    snooze_selected_index: usize,

    // Label picker state
    label_picker_selected: HashSet<String>,
    available_labels: Vec<(String, String)>, // (id, name)

    // Pane widths (resizable)
    sidebar_width: f32,
    message_list_width: f32,
    resize_dragging: Option<ResizeHandle>,
    resize_start_x: f32,
    resize_start_width: f32,
}

/// Which resize handle is being dragged
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    Sidebar,
    MessageList,
}

/// Label representation for sidebar
#[derive(Clone)]
#[allow(dead_code)]
pub struct SidebarLabel {
    pub id: LabelId,
    pub name: String,
    pub color: Option<String>,
}

/// Account representation for sidebar
#[derive(Clone)]
#[allow(dead_code)]
pub struct SidebarAccount {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub unread_count: u32,
    pub is_expanded: bool,
}

/// Thread item for the message list
#[derive(Clone)]
#[allow(dead_code)]
pub struct ThreadListItem {
    pub id: ThreadId,
    pub subject: String,
    pub sender_name: String,
    pub sender_email: String,
    pub snippet: String,
    pub timestamp: String,
    pub is_unread: bool,
    pub is_starred: bool,
    pub message_count: u32,
}

/// Detailed thread data for reading pane
#[derive(Clone)]
#[allow(dead_code)]
pub struct ThreadDetail {
    pub id: ThreadId,
    pub subject: String,
    pub messages: Vec<MessageDetail>,
    pub labels: Vec<String>,
}

/// Individual message in a thread
#[derive(Clone)]
#[allow(dead_code)]
pub struct MessageDetail {
    pub id: EmailId,
    pub sender_name: String,
    pub sender_email: String,
    pub recipients: Vec<String>,
    pub timestamp: String,
    pub body_text: String,
    pub is_unread: bool,
}

impl MainWindow {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let mut this = Self {
            theme: Theme::dark(),
            focus_handle,
            current_view: ViewType::Inbox,
            active_overlay: ActiveOverlay::None,
            command_palette_buffer: TextBuffer::new(),
            command_palette_selected: 0,
            search_buffer: TextBuffer::new(),
            composer_to: TextBuffer::new(),
            composer_cc: TextBuffer::new(),
            composer_bcc: TextBuffer::new(),
            composer_subject: TextBuffer::new(),
            composer_body: TextBuffer::new(),
            composer_active_field: ComposerField::To,
            composer_show_cc: false,
            composer_show_bcc: false,
            settings_active_tab: SettingsTab::General,

            // General settings defaults
            settings_desktop_notifications: true,
            settings_sound_alerts: false,
            settings_badge_count: true,
            settings_launch_at_startup: false,
            settings_show_in_menu_bar: true,
            settings_auto_archive_after_reply: false,
            settings_mark_as_read_when_opened: true,
            settings_show_conversation_view: true,

            // AI settings defaults
            settings_smart_compose: true,
            settings_email_summarization: true,
            settings_priority_inbox: true,
            settings_sender_categorization: true,
            settings_ai_provider: AiProvider::Ollama,

            // Appearance settings defaults
            settings_theme_mode: ThemeMode::Dark,
            settings_font_size: FontSize::Medium,
            settings_display_density: DisplayDensity::Comfortable,

            account_setup_mode: AccountSetupMode::Selection,
            imap_active_field: ImapField::ImapServer,
            imap_server: TextBuffer::new(),
            imap_port: TextBuffer::with_text("993"),
            imap_username: TextBuffer::new(),
            imap_password: TextBuffer::new(),
            smtp_server: TextBuffer::new(),
            smtp_port: TextBuffer::with_text("587"),
            sidebar_accounts: Vec::new(),
            sidebar_labels: Vec::new(),
            sidebar_collapsed_sections: HashSet::new(),
            threads: Vec::new(),
            selected_thread_id: None,
            focused_index: 0,
            current_thread: None,
            expanded_messages: HashSet::new(),
            is_syncing: false,
            sync_progress: 0,
            is_offline: false,
            ai_status: None,
            last_sync: Some("2 minutes ago".to_string()),
            undo_stack: Vec::new(),
            toast: None,
            screener_entries: Vec::new(),
            screener_selected_index: 0,
            stats_email_received: 247,
            stats_email_sent: 89,
            stats_email_archived: 156,
            stats_email_starred: 12,
            stats_avg_response_mins: 45,
            stats_inbox_zero_count: 3,
            stats_sessions: 28,
            stats_time_in_app_mins: 340,
            stats_ai_summaries: 42,
            stats_ai_drafts: 15,
            stats_ai_searches: 23,
            stats_ai_tokens: 156_000,
            stats_time_range: StatsTimeRange::Week,
            snooze_selected_index: 0,
            label_picker_selected: HashSet::new(),
            available_labels: vec![
                ("label-1".to_string(), "Work".to_string()),
                ("label-2".to_string(), "Personal".to_string()),
                ("label-3".to_string(), "Important".to_string()),
                ("label-4".to_string(), "Follow Up".to_string()),
                ("label-5".to_string(), "Waiting".to_string()),
            ],
            sidebar_width: 220.0,
            message_list_width: 380.0,
            resize_dragging: None,
            resize_start_x: 0.0,
            resize_start_width: 0.0,
        };

        this.load_sample_data();
        // Focus is managed via track_focus() in render

        this
    }

    fn load_sample_data(&mut self) {
        self.threads = vec![
            ThreadListItem {
                id: ThreadId::from("thread-1"),
                subject: "Welcome to The Heap".to_string(),
                sender_name: "The Heap Team".to_string(),
                sender_email: "team@theheap.email".to_string(),
                snippet: "Get started with your new email client...".to_string(),
                timestamp: "Just now".to_string(),
                is_unread: true,
                is_starred: true,
                message_count: 1,
            },
            ThreadListItem {
                id: ThreadId::from("thread-2"),
                subject: "Project Update: Q1 Planning".to_string(),
                sender_name: "Alice Chen".to_string(),
                sender_email: "alice@example.com".to_string(),
                snippet: "Hey team, I wanted to share the latest updates...".to_string(),
                timestamp: "10:30 AM".to_string(),
                is_unread: true,
                is_starred: false,
                message_count: 5,
            },
            ThreadListItem {
                id: ThreadId::from("thread-3"),
                subject: "Re: Code Review Request".to_string(),
                sender_name: "Bob Smith".to_string(),
                sender_email: "bob@example.com".to_string(),
                snippet: "Looks good to me! Just a few minor suggestions...".to_string(),
                timestamp: "Yesterday".to_string(),
                is_unread: false,
                is_starred: false,
                message_count: 3,
            },
            ThreadListItem {
                id: ThreadId::from("thread-4"),
                subject: "Meeting Notes - Product Sync".to_string(),
                sender_name: "Carol Davis".to_string(),
                sender_email: "carol@example.com".to_string(),
                snippet: "Here are the notes from today's meeting...".to_string(),
                timestamp: "Yesterday".to_string(),
                is_unread: false,
                is_starred: true,
                message_count: 1,
            },
            ThreadListItem {
                id: ThreadId::from("thread-5"),
                subject: "Weekend Plans?".to_string(),
                sender_name: "David Lee".to_string(),
                sender_email: "david@example.com".to_string(),
                snippet: "Anyone up for hiking this weekend?".to_string(),
                timestamp: "2 days ago".to_string(),
                is_unread: false,
                is_starred: false,
                message_count: 8,
            },
        ];

        self.sidebar_accounts = vec![
            SidebarAccount {
                id: "account-1".to_string(),
                email: "user@gmail.com".to_string(),
                display_name: Some("Personal".to_string()),
                unread_count: 3,
                is_expanded: true,
            },
            SidebarAccount {
                id: "account-2".to_string(),
                email: "user@company.com".to_string(),
                display_name: Some("Work".to_string()),
                unread_count: 12,
                is_expanded: false,
            },
        ];

        self.sidebar_labels = vec![
            SidebarLabel {
                id: LabelId::from("work"),
                name: "Work".to_string(),
                color: Some("#3b82f6".to_string()),
            },
            SidebarLabel {
                id: LabelId::from("personal"),
                name: "Personal".to_string(),
                color: Some("#22c55e".to_string()),
            },
            SidebarLabel {
                id: LabelId::from("urgent"),
                name: "Urgent".to_string(),
                color: Some("#ef4444".to_string()),
            },
        ];

        self.screener_entries = vec![
            ScreenerEntry::new("screener-1", "newsletter@techweekly.io")
                .with_name("Tech Weekly")
                .with_email_preview(
                    Some("This Week in Tech: AI Revolution".to_string()),
                    "The latest news in artificial intelligence and machine learning...",
                )
                .with_ai_analysis(
                    SenderType::Newsletter,
                    "Newsletter subscription based on sender pattern and content structure",
                    ScreenerAction::Reject,
                ),
            ScreenerEntry::new("screener-2", "sarah.recruiter@linkedin.com")
                .with_name("Sarah Johnson")
                .with_email_preview(
                    Some("Exciting opportunity at TechCorp".to_string()),
                    "Hi! I came across your profile and wanted to reach out about...",
                )
                .with_ai_analysis(
                    SenderType::Recruiter,
                    "Recruiter outreach based on LinkedIn domain and job-related content",
                    ScreenerAction::Review,
                ),
            ScreenerEntry::new("screener-3", "notifications@github.com")
                .with_name("GitHub")
                .with_email_preview(
                    Some("New issue assigned to you".to_string()),
                    "You have been assigned to issue #1234 in repo/project...",
                )
                .with_ai_analysis(
                    SenderType::Support,
                    "Development platform notification - likely legitimate",
                    ScreenerAction::Approve,
                ),
            ScreenerEntry::new("screener-4", "promo@randomstore.xyz")
                .with_name("Random Store")
                .with_email_preview(
                    Some("EXCLUSIVE DEAL: 90% OFF TODAY ONLY!!!".to_string()),
                    "Don't miss this incredible limited-time offer! Click now to save...",
                )
                .with_ai_analysis(
                    SenderType::Marketing,
                    "Aggressive marketing tactics and suspicious domain - likely spam",
                    ScreenerAction::Reject,
                ),
        ];
    }

    fn navigate_to(&mut self, view: ViewType, cx: &mut Context<Self>) {
        self.current_view = view;
        self.selected_thread_id = None;
        self.current_thread = None;
        self.focused_index = 0;
        cx.notify();
    }

    fn select_thread(&mut self, thread_id: ThreadId, cx: &mut Context<Self>) {
        self.selected_thread_id = Some(thread_id.clone());

        // Find index
        if let Some(idx) = self.threads.iter().position(|t| t.id == thread_id) {
            self.focused_index = idx;
        }

        // Load thread detail
        self.current_thread = Some(self.get_thread_detail(&thread_id));
        self.expanded_messages.clear();

        // Expand last message
        if let Some(ref thread) = self.current_thread {
            if let Some(last) = thread.messages.last() {
                self.expanded_messages.insert(last.id.clone());
            }
        }

        cx.notify();
    }

    fn focus_next(&mut self, cx: &mut Context<Self>) {
        if self.current_view == ViewType::Screener {
            self.screener_select_next();
            cx.notify();
        } else if self.focused_index + 1 < self.threads.len() {
            self.focused_index += 1;
            let thread_id = self.threads[self.focused_index].id.clone();
            self.select_thread(thread_id, cx);
        }
    }

    fn focus_previous(&mut self, cx: &mut Context<Self>) {
        if self.current_view == ViewType::Screener {
            self.screener_select_previous();
            cx.notify();
        } else if self.focused_index > 0 {
            self.focused_index -= 1;
            let thread_id = self.threads[self.focused_index].id.clone();
            self.select_thread(thread_id, cx);
        }
    }

    // Overlay management
    fn show_overlay(&mut self, overlay: ActiveOverlay, cx: &mut Context<Self>) {
        self.active_overlay = overlay;
        cx.notify();
    }

    fn dismiss_overlay(&mut self, cx: &mut Context<Self>) {
        self.active_overlay = ActiveOverlay::None;
        self.command_palette_buffer.clear();
        self.command_palette_selected = 0;
        self.search_buffer.clear();
        self.composer_to.clear();
        self.composer_cc.clear();
        self.composer_bcc.clear();
        self.composer_subject.clear();
        self.composer_body.clear();
        self.composer_active_field = ComposerField::To;
        self.composer_show_cc = false;
        self.composer_show_bcc = false;
        self.settings_active_tab = SettingsTab::General;
        self.account_setup_mode = AccountSetupMode::Selection;
        self.imap_active_field = ImapField::ImapServer;
        self.imap_server.clear();
        self.imap_port.set_text("993");
        self.imap_username.clear();
        self.imap_password.clear();
        self.smtp_server.clear();
        self.smtp_port.set_text("587");
        cx.notify();
    }

    /// Handle keyboard input for active overlay.
    fn handle_overlay_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let key = &event.keystroke.key;
        let shift = event.keystroke.modifiers.shift;
        let ctrl = event.keystroke.modifiers.control;
        let cmd = event.keystroke.modifiers.platform;

        // Handle up/down navigation in command palette
        if self.active_overlay == ActiveOverlay::CommandPalette {
            match key.as_str() {
                "up" => {
                    if self.command_palette_selected > 0 {
                        self.command_palette_selected -= 1;
                    }
                    cx.notify();
                    return true;
                }
                "down" => {
                    let max = self.get_filtered_commands_count();
                    if self.command_palette_selected + 1 < max {
                        self.command_palette_selected += 1;
                    }
                    cx.notify();
                    return true;
                }
                _ => {}
            }
        }

        // Handle composer-specific keys
        if self.active_overlay == ActiveOverlay::Composer {
            return self.handle_composer_key(key, shift, ctrl, cmd, cx);
        }

        // Handle account setup keys
        if self.active_overlay == ActiveOverlay::AccountSetup {
            return self.handle_account_setup_key(key, shift, cx);
        }

        // Get the active buffer
        let buffer = match self.active_overlay {
            ActiveOverlay::CommandPalette => &mut self.command_palette_buffer,
            ActiveOverlay::Search => &mut self.search_buffer,
            _ => return false,
        };

        let result = buffer.process_key(key, shift, ctrl, cmd);

        match result {
            KeyInputResult::TextChanged => {
                // Reset selection when query changes
                if self.active_overlay == ActiveOverlay::CommandPalette {
                    self.command_palette_selected = 0;
                }
                cx.notify();
                true
            }
            KeyInputResult::Consumed => {
                cx.notify();
                true
            }
            KeyInputResult::Submit => {
                // Execute the command or search
                self.execute_overlay_submit(cx);
                true
            }
            KeyInputResult::Cancel => {
                self.dismiss_overlay(cx);
                true
            }
            KeyInputResult::Ignored => false,
        }
    }

    /// Handle keyboard input for the composer.
    fn handle_composer_key(
        &mut self,
        key: &str,
        shift: bool,
        ctrl: bool,
        cmd: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        // Handle tab to move between fields
        if key == "tab" {
            if shift {
                self.composer_previous_field();
            } else {
                self.composer_next_field();
            }
            cx.notify();
            return true;
        }

        // Handle escape to close
        if key == "escape" {
            self.dismiss_overlay(cx);
            return true;
        }

        // Handle Cmd+Enter to send
        if key == "enter" && cmd {
            self.send_email(cx);
            return true;
        }

        // Handle enter in body to insert newline
        if key == "enter" && self.composer_active_field == ComposerField::Body {
            self.composer_body.insert_char('\n');
            cx.notify();
            return true;
        }

        // Handle enter in other fields to move to next
        if key == "enter" {
            self.composer_next_field();
            cx.notify();
            return true;
        }

        // Get the active buffer
        let buffer = match self.composer_active_field {
            ComposerField::To => &mut self.composer_to,
            ComposerField::Cc => &mut self.composer_cc,
            ComposerField::Bcc => &mut self.composer_bcc,
            ComposerField::Subject => &mut self.composer_subject,
            ComposerField::Body => &mut self.composer_body,
        };

        let result = buffer.process_key(key, shift, ctrl, cmd);

        match result {
            KeyInputResult::TextChanged | KeyInputResult::Consumed => {
                cx.notify();
                true
            }
            KeyInputResult::Submit | KeyInputResult::Cancel => {
                // Already handled above
                true
            }
            KeyInputResult::Ignored => false,
        }
    }

    /// Move to the next composer field.
    fn composer_next_field(&mut self) {
        self.composer_active_field = match self.composer_active_field {
            ComposerField::To => {
                if self.composer_show_cc {
                    ComposerField::Cc
                } else if self.composer_show_bcc {
                    ComposerField::Bcc
                } else {
                    ComposerField::Subject
                }
            }
            ComposerField::Cc => {
                if self.composer_show_bcc {
                    ComposerField::Bcc
                } else {
                    ComposerField::Subject
                }
            }
            ComposerField::Bcc => ComposerField::Subject,
            ComposerField::Subject => ComposerField::Body,
            ComposerField::Body => ComposerField::Body, // Stay on body
        };
    }

    /// Move to the previous composer field.
    fn composer_previous_field(&mut self) {
        self.composer_active_field = match self.composer_active_field {
            ComposerField::To => ComposerField::To, // Stay on To
            ComposerField::Cc => ComposerField::To,
            ComposerField::Bcc => {
                if self.composer_show_cc {
                    ComposerField::Cc
                } else {
                    ComposerField::To
                }
            }
            ComposerField::Subject => {
                if self.composer_show_bcc {
                    ComposerField::Bcc
                } else if self.composer_show_cc {
                    ComposerField::Cc
                } else {
                    ComposerField::To
                }
            }
            ComposerField::Body => ComposerField::Subject,
        };
    }

    /// Send the composed email.
    fn send_email(&mut self, cx: &mut Context<Self>) {
        let to = self.composer_to.text().to_string();
        let cc = self.composer_cc.text().to_string();
        let bcc = self.composer_bcc.text().to_string();
        let subject = self.composer_subject.text().to_string();
        let _body = self.composer_body.text().to_string();

        if to.is_empty() {
            tracing::warn!("Cannot send: no recipients");
            return;
        }

        tracing::info!(
            "Sending email - To: {}, Cc: {}, Bcc: {}, Subject: {}",
            to,
            cc,
            bcc,
            subject
        );

        // TODO: Actually send via email service
        self.dismiss_overlay(cx);
    }

    /// Handle keyboard input for account setup overlay.
    fn handle_account_setup_key(&mut self, key: &str, shift: bool, cx: &mut Context<Self>) -> bool {
        // Only handle keys in IMAP mode
        if self.account_setup_mode != AccountSetupMode::Imap {
            if key == "escape" {
                if self.account_setup_mode == AccountSetupMode::Selection {
                    self.dismiss_overlay(cx);
                } else {
                    self.account_setup_mode = AccountSetupMode::Selection;
                    cx.notify();
                }
                return true;
            }
            return false;
        }

        // Handle escape
        if key == "escape" {
            self.account_setup_mode = AccountSetupMode::Selection;
            cx.notify();
            return true;
        }

        // Handle tab to move between fields
        if key == "tab" {
            if shift {
                self.imap_previous_field();
            } else {
                self.imap_next_field();
            }
            cx.notify();
            return true;
        }

        // Handle enter to move to next field
        if key == "enter" {
            self.imap_next_field();
            cx.notify();
            return true;
        }

        // Get the active buffer
        let buffer = match self.imap_active_field {
            ImapField::ImapServer => &mut self.imap_server,
            ImapField::ImapPort => &mut self.imap_port,
            ImapField::SmtpServer => &mut self.smtp_server,
            ImapField::SmtpPort => &mut self.smtp_port,
            ImapField::Username => &mut self.imap_username,
            ImapField::Password => &mut self.imap_password,
        };

        let result = buffer.process_key(key, shift, false, false);

        match result {
            KeyInputResult::TextChanged | KeyInputResult::Consumed => {
                cx.notify();
                true
            }
            _ => false,
        }
    }

    /// Move to the next IMAP field.
    fn imap_next_field(&mut self) {
        self.imap_active_field = match self.imap_active_field {
            ImapField::ImapServer => ImapField::ImapPort,
            ImapField::ImapPort => ImapField::SmtpServer,
            ImapField::SmtpServer => ImapField::SmtpPort,
            ImapField::SmtpPort => ImapField::Username,
            ImapField::Username => ImapField::Password,
            ImapField::Password => ImapField::Password, // Stay on password
        };
    }

    /// Move to the previous IMAP field.
    fn imap_previous_field(&mut self) {
        self.imap_active_field = match self.imap_active_field {
            ImapField::ImapServer => ImapField::ImapServer, // Stay on first
            ImapField::ImapPort => ImapField::ImapServer,
            ImapField::SmtpServer => ImapField::ImapPort,
            ImapField::SmtpPort => ImapField::SmtpServer,
            ImapField::Username => ImapField::SmtpPort,
            ImapField::Password => ImapField::Username,
        };
    }

    /// Get the count of filtered commands for the current query.
    fn get_filtered_commands_count(&self) -> usize {
        let query = self.command_palette_buffer.text();
        if query.is_empty() {
            17 // Total number of commands
        } else {
            let q = query.to_lowercase();
            COMMANDS
                .iter()
                .filter(|(label, _)| label.to_lowercase().contains(&q))
                .count()
        }
    }

    /// Get the filtered commands list.
    fn get_filtered_commands(&self) -> Vec<(&'static str, &'static str)> {
        let query = self.command_palette_buffer.text();
        if query.is_empty() {
            COMMANDS.to_vec()
        } else {
            let q = query.to_lowercase();
            COMMANDS
                .iter()
                .filter(|(label, _)| label.to_lowercase().contains(&q))
                .copied()
                .collect()
        }
    }

    /// Execute submit action for the active overlay.
    fn execute_overlay_submit(&mut self, cx: &mut Context<Self>) {
        match self.active_overlay {
            ActiveOverlay::CommandPalette => {
                let filtered = self.get_filtered_commands();
                if let Some((label, _)) = filtered.get(self.command_palette_selected) {
                    self.execute_command(label, cx);
                } else {
                    self.dismiss_overlay(cx);
                }
            }
            ActiveOverlay::Search => {
                let query = self.search_buffer.text().to_string();
                if !query.is_empty() {
                    tracing::info!("Executing search: {}", query);
                    self.navigate_to(ViewType::Search(query), cx);
                }
                self.dismiss_overlay(cx);
            }
            _ => {}
        }
    }

    /// Execute a command by label.
    fn execute_command(&mut self, label: &str, cx: &mut Context<Self>) {
        match label {
            "Go to Inbox" => {
                self.dismiss_overlay(cx);
                self.navigate_to(ViewType::Inbox, cx);
            }
            "Go to Starred" => {
                self.dismiss_overlay(cx);
                self.navigate_to(ViewType::Starred, cx);
            }
            "Go to Sent" => {
                self.dismiss_overlay(cx);
                self.navigate_to(ViewType::Sent, cx);
            }
            "Go to Drafts" => {
                self.dismiss_overlay(cx);
                self.navigate_to(ViewType::Drafts, cx);
            }
            "Go to Archive" => {
                self.dismiss_overlay(cx);
                self.navigate_to(ViewType::Archive, cx);
            }
            "Compose" => {
                self.dismiss_overlay(cx);
                self.show_overlay(ActiveOverlay::Composer, cx);
            }
            "Reply" | "Reply All" | "Forward" => {
                if self.selected_thread_id.is_some() {
                    self.dismiss_overlay(cx);
                    self.show_overlay(ActiveOverlay::Composer, cx);
                }
            }
            "Archive" => {
                self.dismiss_overlay(cx);
                self.archive_selected(cx);
            }
            "Trash" => {
                self.dismiss_overlay(cx);
                self.trash_selected(cx);
            }
            "Star" => {
                self.dismiss_overlay(cx);
                self.star_selected(cx);
            }
            "Snooze" => {
                self.dismiss_overlay(cx);
                self.snooze_selected(cx);
            }
            "Mark Read" => {
                self.dismiss_overlay(cx);
                self.mark_read_selected(cx);
            }
            "Mark Unread" => {
                self.dismiss_overlay(cx);
                self.mark_unread_selected(cx);
            }
            "Settings" => {
                self.dismiss_overlay(cx);
                self.show_overlay(ActiveOverlay::Settings, cx);
            }
            "Search" => {
                self.dismiss_overlay(cx);
                self.show_overlay(ActiveOverlay::Search, cx);
            }
            _ => self.dismiss_overlay(cx),
        }
    }

    fn toggle_overlay(&mut self, overlay: ActiveOverlay, cx: &mut Context<Self>) {
        if self.active_overlay == overlay {
            self.dismiss_overlay(cx);
        } else {
            self.show_overlay(overlay, cx);
        }
    }

    // Undo system
    fn push_undo_action(&mut self, action: UndoableAction) {
        let description = action.description();
        self.undo_stack.push(action);
        // Limit undo stack size
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.show_toast(format!("{} - Press Z to undo", description), true);
    }

    fn show_toast(&mut self, message: impl Into<String>, can_undo: bool) {
        self.toast = Some(Toast::new(message, can_undo));
    }

    fn dismiss_toast(&mut self) {
        self.toast = None;
    }

    fn undo_last(&mut self, cx: &mut Context<Self>) {
        if let Some(action) = self.undo_stack.pop() {
            let description = action.undo_description();
            match action {
                UndoableAction::Archive {
                    thread_id,
                    from_view: _,
                } => {
                    tracing::info!("Undo archive: {:?}", thread_id);
                    // TODO: Actually unarchive via service
                }
                UndoableAction::Trash {
                    thread_id,
                    from_view: _,
                } => {
                    tracing::info!("Undo trash: {:?}", thread_id);
                    // TODO: Actually restore from trash via service
                }
                UndoableAction::Star { thread_id } => {
                    tracing::info!("Undo star: {:?}", thread_id);
                    if let Some(thread) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                        thread.is_starred = false;
                    }
                }
                UndoableAction::Unstar { thread_id } => {
                    tracing::info!("Undo unstar: {:?}", thread_id);
                    if let Some(thread) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                        thread.is_starred = true;
                    }
                }
                UndoableAction::MarkRead { thread_id } => {
                    tracing::info!("Undo mark read: {:?}", thread_id);
                    if let Some(thread) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                        thread.is_unread = true;
                    }
                }
                UndoableAction::MarkUnread { thread_id } => {
                    tracing::info!("Undo mark unread: {:?}", thread_id);
                    if let Some(thread) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                        thread.is_unread = false;
                    }
                }
                UndoableAction::Snooze { thread_id } => {
                    tracing::info!("Undo snooze: {:?}", thread_id);
                    // TODO: Actually unsnooze via service
                }
            }
            self.show_toast(description, false);
            cx.notify();
        }
    }

    // Email actions on selected thread
    fn archive_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            tracing::info!("Archive thread: {:?}", thread_id);
            self.push_undo_action(UndoableAction::Archive {
                thread_id: thread_id.clone(),
                from_view: self.current_view.clone(),
            });
            // TODO: Actually archive via service
        }
        cx.notify();
    }

    fn trash_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            tracing::info!("Trash thread: {:?}", thread_id);
            self.push_undo_action(UndoableAction::Trash {
                thread_id: thread_id.clone(),
                from_view: self.current_view.clone(),
            });
            // TODO: Actually trash via service
        }
        cx.notify();
    }

    fn star_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            tracing::info!("Star thread: {:?}", thread_id);
            // Toggle star in local state for now
            if let Some(thread) = self.threads.iter_mut().find(|t| t.id == *thread_id) {
                let was_starred = thread.is_starred;
                thread.is_starred = !thread.is_starred;
                if was_starred {
                    self.push_undo_action(UndoableAction::Unstar {
                        thread_id: thread_id.clone(),
                    });
                } else {
                    self.push_undo_action(UndoableAction::Star {
                        thread_id: thread_id.clone(),
                    });
                }
            }
        }
        cx.notify();
    }

    fn snooze_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            tracing::info!("Snooze thread: {:?}", thread_id);
            self.push_undo_action(UndoableAction::Snooze {
                thread_id: thread_id.clone(),
            });
            // TODO: Show snooze picker
        }
        cx.notify();
    }

    fn mark_read_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            if let Some(thread) = self.threads.iter_mut().find(|t| t.id == *thread_id) {
                if thread.is_unread {
                    thread.is_unread = false;
                    self.push_undo_action(UndoableAction::MarkRead {
                        thread_id: thread_id.clone(),
                    });
                }
            }
        }
        cx.notify();
    }

    fn mark_unread_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            if let Some(thread) = self.threads.iter_mut().find(|t| t.id == *thread_id) {
                if !thread.is_unread {
                    thread.is_unread = true;
                    self.push_undo_action(UndoableAction::MarkUnread {
                        thread_id: thread_id.clone(),
                    });
                }
            }
        }
        cx.notify();
    }

    fn get_thread_detail(&self, thread_id: &ThreadId) -> ThreadDetail {
        match thread_id.0.as_str() {
            "thread-1" => ThreadDetail {
                id: thread_id.clone(),
                subject: "Welcome to The Heap".to_string(),
                messages: vec![MessageDetail {
                    id: EmailId::from("msg-1-1"),
                    sender_name: "The Heap Team".to_string(),
                    sender_email: "team@theheap.email".to_string(),
                    recipients: vec!["you@example.com".to_string()],
                    timestamp: "Today at 9:00 AM".to_string(),
                    body_text: "Welcome to The Heap!\n\nWe're excited to have you on board. Here are some tips to get started:\n\n1. Use 'j' and 'k' to navigate through your messages\n2. Press 'e' to archive, 's' to star\n3. Press 'c' to compose a new email\n4. Press '/' to search\n\nEnjoy your new email experience!".to_string(),
                    is_unread: true,
                }],
                labels: vec!["Getting Started".to_string()],
            },
            "thread-2" => ThreadDetail {
                id: thread_id.clone(),
                subject: "Project Update: Q1 Planning".to_string(),
                messages: vec![
                    MessageDetail {
                        id: EmailId::from("msg-2-1"),
                        sender_name: "Alice Chen".to_string(),
                        sender_email: "alice@example.com".to_string(),
                        recipients: vec!["team@example.com".to_string()],
                        timestamp: "Today at 10:30 AM".to_string(),
                        body_text: "Hey team,\n\nI wanted to share the latest updates on our Q1 planning. We've made great progress on the roadmap.\n\nKey highlights:\n- Feature A is on track for release next week\n- Feature B needs some additional work\n- We'll be hiring two new engineers\n\nLet me know if you have any questions!".to_string(),
                        is_unread: false,
                    },
                    MessageDetail {
                        id: EmailId::from("msg-2-2"),
                        sender_name: "You".to_string(),
                        sender_email: "you@example.com".to_string(),
                        recipients: vec!["alice@example.com".to_string()],
                        timestamp: "Today at 10:45 AM".to_string(),
                        body_text: "Thanks for the update, Alice! This looks great.\n\nQuick question - what's the timeline for Feature B?".to_string(),
                        is_unread: false,
                    },
                    MessageDetail {
                        id: EmailId::from("msg-2-3"),
                        sender_name: "Alice Chen".to_string(),
                        sender_email: "alice@example.com".to_string(),
                        recipients: vec!["you@example.com".to_string()],
                        timestamp: "Today at 11:00 AM".to_string(),
                        body_text: "Good question! We're aiming for end of February, but I'll have a more concrete timeline by next week.".to_string(),
                        is_unread: true,
                    },
                ],
                labels: vec!["Work".to_string(), "Planning".to_string()],
            },
            _ => ThreadDetail {
                id: thread_id.clone(),
                subject: self.threads.iter().find(|t| t.id == *thread_id).map(|t| t.subject.clone()).unwrap_or_else(|| "Message".to_string()),
                messages: vec![MessageDetail {
                    id: EmailId::from("msg-default"),
                    sender_name: self.threads.iter().find(|t| t.id == *thread_id).map(|t| t.sender_name.clone()).unwrap_or_else(|| "Sender".to_string()),
                    sender_email: "sender@example.com".to_string(),
                    recipients: vec!["you@example.com".to_string()],
                    timestamp: "Recently".to_string(),
                    body_text: self.threads.iter().find(|t| t.id == *thread_id).map(|t| t.snippet.clone()).unwrap_or_else(|| "This is a sample message.".to_string()),
                    is_unread: false,
                }],
                labels: vec![],
            },
        }
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_searching = self.active_overlay == ActiveOverlay::Search;
        let search_text = if self.search_buffer.is_empty() {
            "Search...".to_string()
        } else {
            self.search_buffer.text().to_string()
        };
        let text_color = if self.search_buffer.is_empty() {
            colors.text_muted
        } else {
            colors.text_primary
        };
        let search_bg = if is_searching {
            colors.surface_elevated
        } else {
            colors.surface
        };
        let border_color = if is_searching {
            colors.accent
        } else {
            colors.border
        };

        div()
            .id("title-bar")
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(16.0))
            .bg(colors.surface)
            .border_b_1()
            .border_color(colors.border)
            // Left side: brand
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from("The Heap")),
            )
            // Center: search bar
            .child(
                div().flex_1().flex().justify_center().mx(px(48.0)).child(
                    div()
                        .id("title-search")
                        .w(px(400.0))
                        .px(px(12.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .bg(search_bg)
                        .border_1()
                        .border_color(border_color)
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.show_overlay(ActiveOverlay::Search, cx);
                        }))
                        .child(
                            div()
                                .text_sm()
                                .text_color(colors.text_muted)
                                .child(SharedString::from("/")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .text_color(text_color)
                                .truncate()
                                .child(SharedString::from(search_text)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(colors.text_muted)
                                .child(SharedString::from("cmd-k")),
                        ),
                ),
            )
            // Right side: spacer to balance the layout
            .child(
                div().w(px(100.0)), // Match approximate width of left brand area
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id("sidebar")
            .w(px(self.sidebar_width))
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.surface)
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .py(px(8.0))
                    // Accounts section
                    .child(self.render_sidebar_section_header("ACCOUNTS", "accounts", cx))
                    .when(
                        !self.sidebar_collapsed_sections.contains("accounts"),
                        |this| {
                            this.children(self.sidebar_accounts.iter().enumerate().map(
                                |(idx, account)| self.render_sidebar_account(idx, account, cx),
                            ))
                        },
                    )
                    // Mailboxes section
                    .child(self.render_sidebar_section_header("MAILBOXES", "mailboxes", cx))
                    .when(
                        !self.sidebar_collapsed_sections.contains("mailboxes"),
                        |this| {
                            this.child(self.render_sidebar_item(
                                "inbox",
                                "Inbox",
                                ViewType::Inbox,
                                Some(2),
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "starred",
                                "Starred",
                                ViewType::Starred,
                                None,
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "snoozed",
                                "Snoozed",
                                ViewType::Snoozed,
                                None,
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "sent",
                                "Sent",
                                ViewType::Sent,
                                None,
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "drafts",
                                "Drafts",
                                ViewType::Drafts,
                                Some(1),
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "archive",
                                "Archive",
                                ViewType::Archive,
                                None,
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "trash",
                                "Trash",
                                ViewType::Trash,
                                None,
                                cx,
                            ))
                        },
                    )
                    // Smart Views section
                    .child(self.render_sidebar_section_header("SMART VIEWS", "smart-views", cx))
                    .when(
                        !self.sidebar_collapsed_sections.contains("smart-views"),
                        |this| {
                            this.child(self.render_sidebar_item(
                                "actionable",
                                "Actionable",
                                ViewType::Actionable,
                                Some(5),
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "newsletters",
                                "Newsletters",
                                ViewType::Newsletters,
                                Some(12),
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "social",
                                "Social",
                                ViewType::Social,
                                Some(8),
                                cx,
                            ))
                            .child(self.render_sidebar_item(
                                "updates",
                                "Updates",
                                ViewType::Updates,
                                Some(23),
                                cx,
                            ))
                        },
                    )
                    // Screener section
                    .child(self.render_sidebar_section_header("SCREENER", "screener", cx))
                    .when(
                        !self.sidebar_collapsed_sections.contains("screener"),
                        |this| {
                            this.child(self.render_sidebar_item(
                                "new-senders",
                                "New Senders",
                                ViewType::Screener,
                                Some(3),
                                cx,
                            ))
                        },
                    )
                    // Labels section
                    .when(!self.sidebar_labels.is_empty(), |this| {
                        this.child(self.render_sidebar_section_header("LABELS", "labels", cx))
                            .when(
                                !self.sidebar_collapsed_sections.contains("labels"),
                                |this| {
                                    this.children(
                                        self.sidebar_labels
                                            .iter()
                                            .map(|label| self.render_label_item(label, cx)),
                                    )
                                },
                            )
                    }),
            )
            // Settings button at bottom
            .child(self.render_sidebar_footer(cx))
    }

    fn render_sidebar_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let hover_bg = colors.surface_elevated;

        div()
            .px(px(12.0))
            .py(px(8.0))
            .border_t_1()
            .border_color(colors.border)
            .child(
                div()
                    .id("sidebar-settings")
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .hover(move |style| style.bg(hover_bg))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.show_overlay(ActiveOverlay::Settings, cx);
                    }))
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child(SharedString::from("Settings")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.text_muted)
                            .child(SharedString::from("cmd-,")),
                    ),
            )
    }

    fn render_resize_handle(
        &self,
        handle: ResizeHandle,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_dragging = self.resize_dragging == Some(handle);
        let accent_color = colors.accent;
        let border_color = colors.border;

        let down_handler = cx.listener(move |this, event: &MouseDownEvent, _window, _cx| {
            if event.button == MouseButton::Left {
                this.resize_dragging = Some(handle);
                this.resize_start_x = f32::from(event.position.x);
                this.resize_start_width = match handle {
                    ResizeHandle::Sidebar => this.sidebar_width,
                    ResizeHandle::MessageList => this.message_list_width,
                };
            }
        });

        div()
            .id(SharedString::from(match handle {
                ResizeHandle::Sidebar => "resize-sidebar",
                ResizeHandle::MessageList => "resize-message-list",
            }))
            .w(px(6.0))
            .h_full()
            .cursor(CursorStyle::ResizeLeftRight)
            .flex()
            .justify_center()
            .child(
                div()
                    .w(px(2.0))
                    .h_full()
                    .when(is_dragging, |this| this.bg(accent_color))
                    .hover(move |style| style.bg(border_color)),
            )
            .on_mouse_down(MouseButton::Left, down_handler)
    }

    fn render_sidebar_section_header(
        &self,
        title: &str,
        section_id: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_collapsed = self.sidebar_collapsed_sections.contains(section_id);
        let section_id_owned = section_id.to_string();
        let muted_color = colors.text_muted;

        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            if this.sidebar_collapsed_sections.contains(&section_id_owned) {
                this.sidebar_collapsed_sections.remove(&section_id_owned);
            } else {
                this.sidebar_collapsed_sections
                    .insert(section_id_owned.clone());
            }
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("section-{}", section_id)))
            .px(px(12.0))
            .py(px(8.0))
            .mt(px(8.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .cursor_pointer()
            .on_click(click_handler)
            .child(
                div()
                    .text_xs()
                    .text_color(muted_color)
                    .child(SharedString::from(if is_collapsed { "+" } else { "-" })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted_color)
                    .font_weight(FontWeight::MEDIUM)
                    .child(SharedString::from(title.to_string())),
            )
    }

    fn render_sidebar_account(
        &self,
        idx: usize,
        account: &SidebarAccount,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let text_color = colors.text_primary;
        let muted_color = colors.text_muted;
        let accent = colors.accent;
        let hover_bg = colors.surface_elevated;

        let display = account
            .display_name
            .as_ref()
            .unwrap_or(&account.email)
            .clone();

        div()
            .id(SharedString::from(format!("account-{}", idx)))
            .px(px(12.0))
            .py(px(6.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(24.0))
                            .rounded_full()
                            .bg(accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div().text_xs().text_color(colors.background).child(
                                    SharedString::from(
                                        display
                                            .chars()
                                            .next()
                                            .unwrap_or('?')
                                            .to_uppercase()
                                            .to_string(),
                                    ),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(text_color)
                                    .child(SharedString::from(display)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_color)
                                    .child(SharedString::from(account.email.clone())),
                            ),
                    )
                    .when(account.unread_count > 0, |this| {
                        this.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(10.0))
                                .bg(accent)
                                .text_xs()
                                .text_color(colors.background)
                                .child(SharedString::from(account.unread_count.to_string())),
                        )
                    }),
            )
    }

    fn render_sidebar_item(
        &self,
        id: &str,
        label: &str,
        view: ViewType,
        count: Option<u32>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_active = self.current_view == view;
        let bg = if is_active {
            colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = colors.surface_elevated;
        let text_color = colors.text_primary;
        let muted_color = colors.text_muted;

        let target_view = view.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.navigate_to(target_view.clone(), cx);
        });

        div()
            .id(SharedString::from(id.to_string()))
            .px(px(12.0))
            .py(px(8.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(click_handler)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(text_color)
                            .child(SharedString::from(label.to_string())),
                    )
                    .when_some(count, |this, c| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(muted_color)
                                .child(SharedString::from(c.to_string())),
                        )
                    }),
            )
    }

    fn render_label_item(&self, label: &SidebarLabel, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_active = matches!(&self.current_view, ViewType::Label(id) if *id == label.id);
        let bg = if is_active {
            colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = colors.surface_elevated;

        let label_id = label.id.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.navigate_to(ViewType::Label(label_id.clone()), cx);
        });

        div()
            .id(SharedString::from(format!("label-{}", label.id.0)))
            .px(px(12.0))
            .py(px(6.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(click_handler)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().size(px(8.0)).rounded_full().bg(colors.accent))
                    .child(
                        div()
                            .text_color(colors.text_primary)
                            .child(SharedString::from(label.name.clone())),
                    ),
            )
    }

    fn render_message_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let view_title = self.view_title();

        let thread_items: Vec<_> = self
            .threads
            .iter()
            .enumerate()
            .map(|(idx, thread)| self.render_thread_item(thread, idx, cx))
            .collect();

        div()
            .id("message-list")
            .w(px(self.message_list_width))
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from(view_title.to_string())),
                            )
                            .child(div().text_sm().text_color(colors.text_muted).child(
                                SharedString::from(format!("{} threads", self.threads.len())),
                            )),
                    ),
            )
            .child(
                div()
                    .id("message-list-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(thread_items),
            )
    }

    fn render_thread_item(
        &self,
        thread: &ThreadListItem,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_selected = self.selected_thread_id.as_ref() == Some(&thread.id);
        let is_focused = index == self.focused_index;

        let bg = if is_selected {
            colors.surface_elevated
        } else if is_focused {
            colors.surface
        } else {
            gpui::Hsla::transparent_black()
        };

        let text_weight = if thread.is_unread {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        };

        let hover_bg = colors.surface;
        let border_color = colors.border;
        let text_primary = colors.text_primary;
        let text_secondary = colors.text_secondary;
        let text_muted = colors.text_muted;
        let starred_color = colors.starred;

        let thread_id = thread.id.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.select_thread(thread_id.clone(), cx);
        });

        div()
            .id(SharedString::from(format!("thread-{}", index)))
            .px(px(16.0))
            .py(px(12.0))
            .bg(bg)
            .border_b_1()
            .border_color(border_color)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(click_handler)
            .child(
                div()
                    .flex()
                    .justify_between()
                    .mb(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .font_weight(text_weight)
                                    .text_color(text_primary)
                                    .child(SharedString::from(thread.sender_name.clone())),
                            )
                            .when(thread.message_count > 1, |this| {
                                this.child(div().text_xs().text_color(text_muted).child(
                                    SharedString::from(format!("({})", thread.message_count)),
                                ))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when(thread.is_starred, |this| {
                                this.child(
                                    div()
                                        .text_color(starred_color)
                                        .child(SharedString::from("*")),
                                )
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child(SharedString::from(thread.timestamp.clone())),
                            ),
                    ),
            )
            .child(
                div()
                    .font_weight(text_weight)
                    .text_color(text_primary)
                    .text_sm()
                    .truncate()
                    .child(SharedString::from(thread.subject.clone())),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(text_secondary)
                    .truncate()
                    .child(SharedString::from(thread.snippet.clone())),
            )
    }

    fn render_reading_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id("reading-pane")
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .when(self.current_thread.is_none(), |this| {
                this.child(
                    div().flex_1().flex().items_center().justify_center().child(
                        div()
                            .text_color(colors.text_muted)
                            .child(SharedString::from("Select a message to read")),
                    ),
                )
            })
            .when_some(self.current_thread.clone(), |this, thread| {
                this.child(self.render_thread_header(&thread)).child(
                    div()
                        .id("reading-pane-scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(thread.messages.iter().map(|msg| {
                            let is_expanded = self.expanded_messages.contains(&msg.id);
                            self.render_message(msg, is_expanded, cx)
                        })),
                )
            })
    }

    fn render_thread_header(&self, thread: &ThreadDetail) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .px(px(24.0))
            .py(px(16.0))
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .child(SharedString::from(thread.subject.clone())),
            )
            .when(!thread.labels.is_empty(), |this| {
                this.child(div().flex().gap(px(8.0)).mt(px(8.0)).children(
                    thread.labels.iter().map(|label| {
                        div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(colors.surface_elevated)
                            .text_xs()
                            .text_color(colors.text_secondary)
                            .child(SharedString::from(label.clone()))
                    }),
                ))
            })
    }

    fn render_message(
        &self,
        message: &MessageDetail,
        is_expanded: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;

        let msg_id = message.id.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            if this.expanded_messages.contains(&msg_id) {
                this.expanded_messages.remove(&msg_id);
            } else {
                this.expanded_messages.insert(msg_id.clone());
            }
            cx.notify();
        });

        if is_expanded {
            div()
                .id(SharedString::from(format!("msg-{}", message.id.0)))
                .px(px(24.0))
                .py(px(16.0))
                .border_b_1()
                .border_color(colors.border)
                .cursor_pointer()
                .on_click(click_handler)
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .mb(px(12.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(12.0))
                                .child(
                                    div()
                                        .size(px(40.0))
                                        .rounded_full()
                                        .bg(colors.surface_elevated)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_color(colors.text_secondary)
                                        .child(SharedString::from(
                                            message
                                                .sender_name
                                                .chars()
                                                .next()
                                                .unwrap_or('?')
                                                .to_string(),
                                        )),
                                )
                                .child(
                                    div()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(colors.text_primary)
                                                .child(SharedString::from(
                                                    message.sender_name.clone(),
                                                )),
                                        )
                                        .child(
                                            div().text_sm().text_color(colors.text_muted).child(
                                                SharedString::from(format!(
                                                    "to {}",
                                                    message.recipients.join(", ")
                                                )),
                                            ),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(colors.text_muted)
                                .child(SharedString::from(message.timestamp.clone())),
                        ),
                )
                .child(
                    div()
                        .text_color(colors.text_primary)
                        .child(SharedString::from(message.body_text.clone())),
                )
        } else {
            div()
                .id(SharedString::from(format!("msg-{}", message.id.0)))
                .px(px(24.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(colors.border)
                .cursor_pointer()
                .hover(move |style| style.bg(colors.surface))
                .on_click(click_handler)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            div()
                                .size(px(32.0))
                                .rounded_full()
                                .bg(colors.surface_elevated)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(colors.text_secondary)
                                .child(SharedString::from(
                                    message
                                        .sender_name
                                        .chars()
                                        .next()
                                        .unwrap_or('?')
                                        .to_string(),
                                )),
                        )
                        .child(
                            div()
                                .flex_1()
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div().text_sm().text_color(colors.text_primary).child(
                                                SharedString::from(message.sender_name.clone()),
                                            ),
                                        )
                                        .child(
                                            div().text_xs().text_color(colors.text_muted).child(
                                                SharedString::from(message.timestamp.clone()),
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(colors.text_secondary)
                                        .truncate()
                                        .child(SharedString::from(truncate_text(
                                            &message.body_text,
                                            80,
                                        ))),
                                ),
                        ),
                )
        }
    }

    fn render_screener_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let entries = &self.screener_entries;
        let count = entries.len();

        if count == 0 {
            return div()
                .id("screener-view")
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(16.0))
                .bg(colors.background)
                .child(
                    div()
                        .text_xl()
                        .text_color(colors.text_muted)
                        .child(SharedString::from("[inbox]")),
                )
                .child(
                    div()
                        .text_lg()
                        .text_color(colors.text_secondary)
                        .child(SharedString::from("No pending senders")),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.text_muted)
                        .child(SharedString::from(
                            "New senders will appear here for review",
                        )),
                );
        }

        let entry_elements: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| self.render_screener_entry(entry, idx, cx))
            .collect();

        div()
            .id("screener-view")
            .flex_1()
            .flex()
            .overflow_hidden()
            .bg(colors.background)
            // Left panel: entry list
            .child(
                div()
                    .w(px(400.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .border_r_1()
                    .border_color(colors.border)
                    // Header
                    .child(
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.text_primary)
                                            .child(SharedString::from("New Senders")),
                                    )
                                    .child(
                                        div()
                                            .px(px(6.0))
                                            .py(px(2.0))
                                            .bg(colors.accent)
                                            .rounded_full()
                                            .text_xs()
                                            .text_color(colors.background)
                                            .child(SharedString::from(count.to_string())),
                                    ),
                            ),
                    )
                    // Entry list
                    .child(
                        div()
                            .id("screener-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .children(entry_elements),
                    ),
            )
            // Right panel: selected entry detail
            .child(self.render_screener_detail(cx))
    }

    fn render_screener_entry(
        &self,
        entry: &ScreenerEntry,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_selected = index == self.screener_selected_index;

        let display_name = entry.name.clone().unwrap_or_else(|| entry.email.clone());
        let first_char = display_name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        let email = entry.email.clone();
        let subject = entry.first_email_subject.clone();
        let badge_text = entry.sender_type_badge();
        let entry_id = entry.id.clone();

        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            if let Some(idx) = this.screener_entries.iter().position(|e| e.id == entry_id) {
                this.screener_selected_index = idx;
            }
            cx.notify();
        });

        let bg = if is_selected {
            colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };

        div()
            .id(SharedString::from(format!("screener-{}", index)))
            .px(px(16.0))
            .py(px(12.0))
            .flex()
            .gap(px(12.0))
            .bg(bg)
            .border_b_1()
            .border_color(colors.border)
            .cursor_pointer()
            .hover(move |style| style.bg(colors.surface))
            .on_click(click_handler)
            // Avatar
            .child(
                div()
                    .size(px(40.0))
                    .rounded_full()
                    .bg(colors.surface_elevated)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.text_secondary)
                    .child(SharedString::from(first_char)),
            )
            // Content
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Name row
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.text_primary)
                                    .truncate()
                                    .child(SharedString::from(display_name)),
                            )
                            .when_some(badge_text, |this, badge| {
                                this.child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .bg(colors.surface)
                                        .rounded(px(4.0))
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from(badge.to_string())),
                                )
                            }),
                    )
                    // Email
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .truncate()
                            .child(SharedString::from(email)),
                    )
                    // Subject preview
                    .when_some(subject, |this, subj| {
                        this.child(
                            div()
                                .mt(px(4.0))
                                .text_sm()
                                .text_color(colors.text_secondary)
                                .truncate()
                                .child(SharedString::from(subj)),
                        )
                    }),
            )
    }

    fn render_screener_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        if let Some(entry) = self.screener_entries.get(self.screener_selected_index) {
            let display_name = entry.name.clone().unwrap_or_else(|| entry.email.clone());
            let email = entry.email.clone();
            let subject = entry.first_email_subject.clone();
            let preview = entry.first_email_preview.clone();
            let ai_reasoning = entry.ai_reasoning.clone();
            let suggested_action = entry.suggested_action_text();
            let first_char = display_name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();

            let entry_id_approve = entry.id.clone();
            let entry_id_reject = entry.id.clone();

            let approve_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.approve_screener_entry(&entry_id_approve);
                cx.notify();
            });

            let reject_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.reject_screener_entry(&entry_id_reject);
                cx.notify();
            });

            div()
                .flex_1()
                .flex()
                .flex_col()
                .p(px(24.0))
                // Header with sender info
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.0))
                        .mb(px(24.0))
                        .child(
                            div()
                                .size(px(64.0))
                                .rounded_full()
                                .bg(colors.surface_elevated)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xl()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.text_secondary)
                                .child(SharedString::from(first_char)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_xl()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(colors.text_primary)
                                        .child(SharedString::from(display_name)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from(email)),
                                ),
                        ),
                )
                // AI Analysis section
                .when(ai_reasoning.is_some(), |this| {
                    this.child(
                        div()
                            .mb(px(24.0))
                            .p(px(16.0))
                            .bg(colors.surface)
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .mb(px(8.0))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(colors.accent)
                                            .child(SharedString::from("AI Analysis")),
                                    )
                                    .when_some(suggested_action, |d, action| {
                                        d.child(
                                            div()
                                                .px(px(8.0))
                                                .py(px(2.0))
                                                .bg(if action == "Approve" {
                                                    colors.success
                                                } else if action == "Reject" {
                                                    colors.error
                                                } else {
                                                    colors.warning
                                                })
                                                .rounded(px(4.0))
                                                .text_xs()
                                                .text_color(colors.background)
                                                .child(SharedString::from(format!(
                                                    "Suggests: {}",
                                                    action
                                                ))),
                                        )
                                    }),
                            )
                            .when_some(ai_reasoning.clone(), |d, reasoning| {
                                d.child(
                                    div()
                                        .text_sm()
                                        .text_color(colors.text_secondary)
                                        .child(SharedString::from(reasoning)),
                                )
                            }),
                    )
                })
                // Email preview
                .child(
                    div()
                        .mb(px(24.0))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(colors.text_muted)
                                .mb(px(8.0))
                                .child(SharedString::from("First Email")),
                        )
                        .when_some(subject, |d, subj| {
                            d.child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.text_primary)
                                    .mb(px(8.0))
                                    .child(SharedString::from(subj)),
                            )
                        })
                        .child(
                            div()
                                .text_sm()
                                .text_color(colors.text_secondary)
                                .child(SharedString::from(preview)),
                        ),
                )
                // Action buttons
                .child(
                    div()
                        .mt_auto()
                        .flex()
                        .gap(px(12.0))
                        .child(
                            div()
                                .id("approve-btn")
                                .flex_1()
                                .px(px(16.0))
                                .py(px(12.0))
                                .bg(colors.success)
                                .rounded(px(8.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_click(approve_handler)
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(colors.background)
                                        .child(SharedString::from("Approve (a)")),
                                ),
                        )
                        .child(
                            div()
                                .id("reject-btn")
                                .flex_1()
                                .px(px(16.0))
                                .py(px(12.0))
                                .bg(colors.error)
                                .rounded(px(8.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_click(reject_handler)
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(colors.background)
                                        .child(SharedString::from("Reject (r)")),
                                ),
                        ),
                )
                // Keyboard hints
                .child(
                    div()
                        .mt(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(16.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .bg(colors.surface)
                                        .rounded(px(4.0))
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("j/k")),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("navigate")),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .bg(colors.surface)
                                        .rounded(px(4.0))
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("a")),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("approve")),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .bg(colors.surface)
                                        .rounded(px(4.0))
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("r")),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("reject")),
                                ),
                        ),
                )
        } else {
            div().flex_1().flex().items_center().justify_center().child(
                div()
                    .text_color(colors.text_muted)
                    .child(SharedString::from("Select a sender to review")),
            )
        }
    }

    fn approve_screener_entry(&mut self, id: &str) {
        if let Some(idx) = self.screener_entries.iter().position(|e| e.id == id) {
            let entry = self.screener_entries.remove(idx);
            self.show_toast(format!("Approved: {}", entry.email), false);
            if self.screener_selected_index >= self.screener_entries.len()
                && !self.screener_entries.is_empty()
            {
                self.screener_selected_index = self.screener_entries.len() - 1;
            }
        }
    }

    fn reject_screener_entry(&mut self, id: &str) {
        if let Some(idx) = self.screener_entries.iter().position(|e| e.id == id) {
            let entry = self.screener_entries.remove(idx);
            self.show_toast(format!("Rejected: {}", entry.email), false);
            if self.screener_selected_index >= self.screener_entries.len()
                && !self.screener_entries.is_empty()
            {
                self.screener_selected_index = self.screener_entries.len() - 1;
            }
        }
    }

    fn screener_select_next(&mut self) {
        if self.screener_selected_index + 1 < self.screener_entries.len() {
            self.screener_selected_index += 1;
        }
    }

    fn screener_select_previous(&mut self) {
        if self.screener_selected_index > 0 {
            self.screener_selected_index -= 1;
        }
    }

    fn render_stats_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id("stats-view")
            .flex_1()
            .flex()
            .flex_col()
            .bg(colors.background)
            .overflow_y_scroll()
            .child(self.render_stats_header(cx))
            .child(
                div()
                    .id("stats-content-scroll")
                    .flex_1()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(24.0))
                    .overflow_y_scroll()
                    .child(self.render_stats_email_section())
                    .child(self.render_stats_productivity_section())
                    .child(self.render_stats_ai_section()),
            )
    }

    fn render_stats_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let current_range = self.stats_time_range;

        div()
            .px(px(24.0))
            .py(px(16.0))
            .border_b_1()
            .border_color(colors.border)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .child(SharedString::from("Statistics")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .children(StatsTimeRange::all().iter().map(|range| {
                        let is_selected = *range == current_range;
                        let bg = if is_selected {
                            colors.accent
                        } else {
                            colors.surface
                        };
                        let text_color = if is_selected {
                            colors.background
                        } else {
                            colors.text_muted
                        };
                        let hover_bg = colors.surface_elevated;
                        let range_name = range.name();
                        let range_copy = *range;

                        div()
                            .id(SharedString::from(format!("range-{:?}", range)))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(bg)
                            .cursor_pointer()
                            .when(!is_selected, move |this| {
                                this.hover(move |s| s.bg(hover_bg))
                            })
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.stats_time_range = range_copy;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_color)
                                    .child(SharedString::from(range_name)),
                            )
                    })),
            )
    }

    fn render_stats_card(
        &self,
        title: &str,
        value: &str,
        subtitle: Option<&str>,
        accent: bool,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let value_color = if accent {
            colors.accent
        } else {
            colors.text_primary
        };

        div()
            .flex_1()
            .p(px(16.0))
            .bg(colors.surface)
            .rounded(px(8.0))
            .border_1()
            .border_color(colors.border)
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_xs()
                    .text_color(colors.text_secondary)
                    .child(SharedString::from(title.to_string())),
            )
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(value_color)
                    .child(SharedString::from(value.to_string())),
            )
            .when_some(subtitle, |this, sub| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child(SharedString::from(sub.to_string())),
                )
            })
    }

    fn render_stats_email_section(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.text_primary)
                    .child(SharedString::from("Email Volume")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(self.render_stats_card(
                        "Received",
                        &self.stats_email_received.to_string(),
                        None,
                        true,
                    ))
                    .child(self.render_stats_card(
                        "Sent",
                        &self.stats_email_sent.to_string(),
                        None,
                        false,
                    ))
                    .child(self.render_stats_card(
                        "Archived",
                        &self.stats_email_archived.to_string(),
                        None,
                        false,
                    ))
                    .child(self.render_stats_card(
                        "Starred",
                        &self.stats_email_starred.to_string(),
                        None,
                        false,
                    )),
            )
    }

    fn render_stats_productivity_section(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        let response_time = if self.stats_avg_response_mins >= 60 {
            format!(
                "{}h {}m",
                self.stats_avg_response_mins / 60,
                self.stats_avg_response_mins % 60
            )
        } else {
            format!("{}m", self.stats_avg_response_mins)
        };

        let time_in_app = if self.stats_time_in_app_mins >= 60 {
            format!(
                "{}h {}m",
                self.stats_time_in_app_mins / 60,
                self.stats_time_in_app_mins % 60
            )
        } else {
            format!("{}m", self.stats_time_in_app_mins)
        };

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.text_primary)
                    .child(SharedString::from("Productivity")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(self.render_stats_card("Avg Response Time", &response_time, None, false))
                    .child(self.render_stats_card(
                        "Inbox Zero",
                        &format!("{}x", self.stats_inbox_zero_count),
                        None,
                        self.stats_inbox_zero_count > 0,
                    ))
                    .child(self.render_stats_card(
                        "Time in App",
                        &time_in_app,
                        Some(&format!("{} sessions", self.stats_sessions)),
                        false,
                    ))
                    .child(self.render_stats_card(
                        "Emails/Session",
                        &format!(
                            "{:.1}",
                            if self.stats_sessions > 0 {
                                (self.stats_email_received + self.stats_email_sent) as f32
                                    / self.stats_sessions as f32
                            } else {
                                0.0
                            }
                        ),
                        None,
                        false,
                    )),
            )
    }

    fn render_stats_ai_section(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        let tokens_display = if self.stats_ai_tokens >= 1_000_000 {
            format!("{:.1}M", self.stats_ai_tokens as f64 / 1_000_000.0)
        } else if self.stats_ai_tokens >= 1_000 {
            format!("{:.1}K", self.stats_ai_tokens as f64 / 1_000.0)
        } else {
            self.stats_ai_tokens.to_string()
        };

        // Estimate cost at ~$0.002 per 1K tokens
        let estimated_cost = (self.stats_ai_tokens as f64 / 1000.0) * 0.002;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.text_primary)
                    .child(SharedString::from("AI Usage")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(self.render_stats_card(
                        "Summaries Generated",
                        &self.stats_ai_summaries.to_string(),
                        None,
                        false,
                    ))
                    .child(self.render_stats_card(
                        "Draft Suggestions",
                        &self.stats_ai_drafts.to_string(),
                        None,
                        false,
                    ))
                    .child(self.render_stats_card(
                        "Semantic Searches",
                        &self.stats_ai_searches.to_string(),
                        None,
                        false,
                    ))
                    .child(self.render_stats_card(
                        "Tokens Used",
                        &tokens_display,
                        Some(&format!("~${:.2}", estimated_cost)),
                        false,
                    )),
            )
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        // Left section: sync status and offline indicator
        let left_section = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(self.render_sync_status())
            .when(self.is_offline, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_color(colors.warning)
                                .text_xs()
                                .child(SharedString::from("[!]")),
                        )
                        .child(
                            div()
                                .text_color(colors.warning)
                                .text_xs()
                                .child(SharedString::from("Offline")),
                        ),
                )
            });

        // Center section: selection count or AI status
        let center_text = if let Some(ref ai_status) = self.ai_status {
            ai_status.clone()
        } else if self.selected_thread_id.is_some() {
            "1 selected".to_string()
        } else {
            "Ready".to_string()
        };

        let center_section = div()
            .text_color(colors.text_muted)
            .text_xs()
            .child(SharedString::from(center_text));

        // Right section: view title and last sync time
        let right_section = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .text_color(colors.text_secondary)
                    .text_xs()
                    .child(SharedString::from(self.view_title().to_string())),
            )
            .when(self.last_sync.is_some(), |this| {
                let sync_text = format!("Synced {}", self.last_sync.as_ref().unwrap());
                this.child(
                    div()
                        .text_color(colors.text_muted)
                        .text_xs()
                        .child(SharedString::from(sync_text)),
                )
            });

        div()
            .id("status-bar")
            .h(px(24.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .bg(colors.surface)
            .border_t_1()
            .border_color(colors.border)
            .child(left_section)
            .child(center_section)
            .child(right_section)
    }

    fn render_sync_status(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        if self.is_syncing {
            let progress_text = format!("Syncing... {}%", self.sync_progress);
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    // Simple spinner indicator
                    div()
                        .text_color(colors.accent)
                        .text_xs()
                        .child(SharedString::from("[~]")),
                )
                .child(
                    div()
                        .text_color(colors.text_muted)
                        .text_xs()
                        .child(SharedString::from(progress_text)),
                )
                .child(
                    // Progress bar
                    div()
                        .w(px(60.0))
                        .h(px(4.0))
                        .bg(colors.border)
                        .rounded(px(2.0))
                        .child(
                            div()
                                .h_full()
                                .w(px(60.0 * self.sync_progress as f32 / 100.0))
                                .bg(colors.accent)
                                .rounded(px(2.0)),
                        ),
                )
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .text_color(colors.success)
                        .text_xs()
                        .child(SharedString::from("[OK]")),
                )
                .child(
                    div()
                        .text_color(colors.text_muted)
                        .text_xs()
                        .child(SharedString::from("Synced")),
                )
        }
    }

    fn render_snooze_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let snooze_options = [
            (SnoozeDuration::LaterToday, "Later Today", "6 PM today"),
            (SnoozeDuration::Tomorrow, "Tomorrow", "8 AM tomorrow"),
            (SnoozeDuration::ThisWeekend, "This Weekend", "Saturday 9 AM"),
            (SnoozeDuration::NextWeek, "Next Week", "Monday 8 AM"),
        ];

        let backdrop_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        div()
            .id("snooze-backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .flex()
            .items_center()
            .justify_center()
            .on_click(backdrop_handler)
            .child(
                div()
                    .id("snooze-picker")
                    .w(px(320.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                    }))
                    .child(
                        // Header
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .border_b_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from("Snooze until")),
                            ),
                    )
                    .child(
                        // Options
                        div()
                            .py(px(8.0))
                            .children(snooze_options.iter().enumerate().map(
                                |(idx, (duration, title, subtitle))| {
                                    let is_selected = idx == self.snooze_selected_index;
                                    let bg = if is_selected {
                                        colors.surface
                                    } else {
                                        gpui::Hsla::transparent_black()
                                    };
                                    let hover_bg = colors.surface;
                                    let text_primary = colors.text_primary;
                                    let text_muted = colors.text_muted;
                                    let title_str = title.to_string();
                                    let subtitle_str = subtitle.to_string();
                                    let duration_copy = *duration;

                                    div()
                                        .id(SharedString::from(format!("snooze-{}", idx)))
                                        .px(px(16.0))
                                        .py(px(10.0))
                                        .bg(bg)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(hover_bg))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _, cx| {
                                                this.apply_snooze(duration_copy, cx);
                                            },
                                        ))
                                        .child(
                                            div()
                                                .flex()
                                                .justify_between()
                                                .items_center()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(text_primary)
                                                        .child(SharedString::from(title_str)),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(text_muted)
                                                        .child(SharedString::from(subtitle_str)),
                                                ),
                                        )
                                },
                            )),
                    )
                    .child(
                        // Footer
                        div()
                            .px(px(16.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.text_muted)
                                    .child(SharedString::from("Press Escape to cancel")),
                            ),
                    ),
            )
    }

    fn apply_snooze(&mut self, duration: SnoozeDuration, cx: &mut Context<Self>) {
        if let Some(ref thread_id) = self.selected_thread_id {
            let description = duration.description();
            self.push_undo_action(UndoableAction::Snooze {
                thread_id: thread_id.clone(),
            });
            self.show_toast(format!("Snoozed until {}", description), true);
        }
        self.dismiss_overlay(cx);
    }

    fn render_label_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let backdrop_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        div()
            .id("label-backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .flex()
            .items_center()
            .justify_center()
            .on_click(backdrop_handler)
            .child(
                div()
                    .id("label-picker")
                    .w(px(280.0))
                    .bg(colors.surface_elevated)
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(colors.border)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                    }))
                    .child(
                        // Header
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .border_b_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from("Apply Labels")),
                            ),
                    )
                    .child(
                        // Labels list
                        div().py(px(8.0)).children(self.available_labels.iter().map(
                            |(id, name)| {
                                let is_selected = self.label_picker_selected.contains(id);
                                let checkbox = if is_selected { "[x]" } else { "[ ]" };
                                let text_primary = colors.text_primary;
                                let text_muted = colors.text_muted;
                                let hover_bg = colors.surface;
                                let name_str = name.clone();
                                let id_str = id.clone();

                                div()
                                    .id(SharedString::from(format!("label-{}", id)))
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(hover_bg))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.toggle_label(&id_str);
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(text_muted)
                                                    .child(SharedString::from(checkbox)),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(text_primary)
                                                    .child(SharedString::from(name_str)),
                                            ),
                                    )
                            },
                        )),
                    )
                    .child(
                        // Footer with apply button
                        div()
                            .px(px(16.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_xs().text_color(colors.text_muted).child(
                                SharedString::from(format!(
                                    "{} selected",
                                    self.label_picker_selected.len()
                                )),
                            ))
                            .child(
                                div()
                                    .id("apply-labels")
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .bg(colors.accent)
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(colors.accent_hover))
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.apply_labels(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(colors.background)
                                            .child(SharedString::from("Apply")),
                                    ),
                            ),
                    ),
            )
    }

    fn toggle_label(&mut self, id: &str) {
        if self.label_picker_selected.contains(id) {
            self.label_picker_selected.remove(id);
        } else {
            self.label_picker_selected.insert(id.to_string());
        }
    }

    fn apply_labels(&mut self, cx: &mut Context<Self>) {
        if !self.label_picker_selected.is_empty() {
            let count = self.label_picker_selected.len();
            let label_text = if count == 1 { "label" } else { "labels" };
            self.show_toast(format!("Applied {} {}", count, label_text), false);
            self.label_picker_selected.clear();
        }
        self.dismiss_overlay(cx);
    }

    fn render_toast(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        if let Some(ref toast) = self.toast {
            if toast.is_expired() {
                return div().id("toast-empty");
            }

            let dismiss_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
                this.dismiss_toast();
                cx.notify();
            });

            div()
                .id("toast-container")
                .absolute()
                .bottom(px(40.0))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("toast")
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .px(px(16.0))
                        .py(px(10.0))
                        .bg(colors.surface_elevated)
                        .border_1()
                        .border_color(colors.border)
                        .rounded(px(8.0))
                        .shadow_lg()
                        .child(
                            div()
                                .text_sm()
                                .text_color(colors.text_primary)
                                .child(SharedString::from(toast.message.clone())),
                        )
                        .when(toast.can_undo, |this| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(colors.accent)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(SharedString::from("Z")),
                            )
                        })
                        .child(
                            div()
                                .id("toast-dismiss")
                                .text_xs()
                                .text_color(colors.text_muted)
                                .cursor_pointer()
                                .on_click(dismiss_handler)
                                .child(SharedString::from("[x]")),
                        ),
                )
        } else {
            div().id("toast-empty")
        }
    }

    fn view_title(&self) -> &str {
        match &self.current_view {
            ViewType::Inbox => "Inbox",
            ViewType::Starred => "Starred",
            ViewType::Sent => "Sent",
            ViewType::Drafts => "Drafts",
            ViewType::Archive => "Archive",
            ViewType::Trash => "Trash",
            ViewType::Snoozed => "Snoozed",
            ViewType::Screener => "New Senders",
            ViewType::Settings => "Settings",
            ViewType::Stats => "Statistics",
            ViewType::Label(_) => "Label",
            ViewType::Search(_) => "Search",
            ViewType::Actionable => "Actionable",
            ViewType::Newsletters => "Newsletters",
            ViewType::Social => "Social",
            ViewType::Updates => "Updates",
        }
    }

    fn render_command_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let filtered = self.get_filtered_commands();
        let selected_idx = self.command_palette_selected;

        let dismiss_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        // Block mouse events from passing through the backdrop
        let block_mouse_move = cx.listener(|_, _: &MouseMoveEvent, _, cx| {
            cx.stop_propagation();
        });
        let block_mouse_down = cx.listener(|_, _: &MouseDownEvent, _, cx| {
            cx.stop_propagation();
        });

        div()
            .id("command-palette-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(80.0))
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .on_click(dismiss_handler)
            .on_mouse_move(block_mouse_move)
            .on_mouse_down(MouseButton::Left, block_mouse_down)
            .child(
                div()
                    .id("command-palette-content")
                    .w(px(480.0))
                    .max_h(px(400.0))
                    .rounded(px(8.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .overflow_hidden()
                    .on_click(content_click)
                    .child(
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .border_b_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_color(colors.text_muted)
                                            .child(SharedString::from(">")),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_color(if self.command_palette_buffer.is_empty() {
                                                colors.text_muted
                                            } else {
                                                colors.text_primary
                                            })
                                            .child(SharedString::from(
                                                if self.command_palette_buffer.is_empty() {
                                                    "Type a command...".to_string()
                                                } else {
                                                    self.command_palette_buffer.text().to_string()
                                                },
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("command-palette-scroll")
                            .max_h(px(320.0))
                            .overflow_y_scroll()
                            .children(filtered.iter().enumerate().map(
                                |(idx, (label, shortcut))| {
                                    let is_selected = idx == selected_idx;
                                    let bg = if is_selected {
                                        colors.surface_elevated
                                    } else {
                                        gpui::Hsla::transparent_black()
                                    };
                                    let cmd_label = label.to_string();
                                    let click_handler =
                                        cx.listener(move |this, _: &ClickEvent, _, cx| {
                                            this.execute_command(&cmd_label, cx);
                                        });
                                    div()
                                        .id(SharedString::from(format!("cmd-{}", idx)))
                                        .px(px(16.0))
                                        .py(px(8.0))
                                        .bg(bg)
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(colors.surface_elevated))
                                        .on_click(click_handler)
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div().text_color(colors.text_primary).child(
                                                        SharedString::from(label.to_string()),
                                                    ),
                                                )
                                                .child(
                                                    div()
                                                        .px(px(6.0))
                                                        .py(px(2.0))
                                                        .rounded(px(4.0))
                                                        .bg(colors.surface)
                                                        .text_xs()
                                                        .text_color(colors.text_muted)
                                                        .child(SharedString::from(
                                                            shortcut.to_string(),
                                                        )),
                                                ),
                                        )
                                },
                            )),
                    ),
            )
    }

    fn render_search_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let dismiss_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        // Block mouse events from passing through the backdrop
        let block_mouse_move = cx.listener(|_, _: &MouseMoveEvent, _, cx| {
            cx.stop_propagation();
        });
        let block_mouse_down = cx.listener(|_, _: &MouseDownEvent, _, cx| {
            cx.stop_propagation();
        });

        // Search operators
        let operators = [
            ("from:", "Sender email"),
            ("to:", "Recipient email"),
            ("subject:", "Subject text"),
            ("has:attachment", "With attachments"),
            ("is:unread", "Unread only"),
            ("is:starred", "Starred only"),
            ("in:inbox", "In Inbox"),
            ("before:", "Before date"),
            ("after:", "After date"),
        ];

        // Calculate the position: center of window minus half the dropdown width
        // The dropdown should align with the search input in the title bar
        let dropdown_width = 500.0;

        div()
            .id("search-overlay")
            .absolute()
            .inset_0()
            .on_click(dismiss_handler)
            .on_mouse_move(block_mouse_move)
            .on_mouse_down(MouseButton::Left, block_mouse_down)
            // Position the dropdown below the title bar, centered
            .child(
                div()
                    .absolute()
                    .top(px(40.0)) // Below title bar
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("search-dropdown")
                            .w(px(dropdown_width))
                            .rounded_b(px(8.0))
                            .bg(colors.surface)
                            .border_1()
                            .border_color(colors.accent)
                            .border_t_0()
                            .overflow_hidden()
                            .shadow_lg()
                            .on_click(content_click)
                            // Search input area
                            .child(
                                div()
                                    .px(px(16.0))
                                    .py(px(12.0))
                                    .border_b_1()
                                    .border_color(colors.border)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(12.0))
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from("/")),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_color(if self.search_buffer.is_empty() {
                                                        colors.text_muted
                                                    } else {
                                                        colors.text_primary
                                                    })
                                                    .child(SharedString::from(
                                                        if self.search_buffer.is_empty() {
                                                            "Search emails...".to_string()
                                                        } else {
                                                            self.search_buffer.text().to_string()
                                                        },
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .px(px(8.0))
                                                    .py(px(4.0))
                                                    .rounded(px(4.0))
                                                    .bg(colors.surface_elevated)
                                                    .text_xs()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from("Esc")),
                                            ),
                                    ),
                            )
                            // Search operators grid
                            .child(
                                div()
                                    .px(px(16.0))
                                    .py(px(12.0))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(colors.text_muted)
                                            .mb(px(8.0))
                                            .child(SharedString::from("SEARCH OPERATORS")),
                                    )
                                    .child(div().flex().flex_wrap().gap(px(8.0)).children(
                                        operators.iter().map(|(op, desc)| {
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .child(
                                                    div()
                                                        .px(px(6.0))
                                                        .py(px(2.0))
                                                        .rounded(px(4.0))
                                                        .bg(colors.surface_elevated)
                                                        .text_xs()
                                                        .text_color(colors.accent)
                                                        .child(SharedString::from(op.to_string())),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(colors.text_muted)
                                                        .child(SharedString::from(
                                                            desc.to_string(),
                                                        )),
                                                )
                                        }),
                                    )),
                            )
                            // Footer with keyboard hints
                            .child(
                                div()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .border_t_1()
                                    .border_color(colors.border)
                                    .flex()
                                    .items_center()
                                    .gap(px(16.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .px(px(6.0))
                                                    .py(px(2.0))
                                                    .rounded(px(2.0))
                                                    .bg(colors.surface_elevated)
                                                    .text_xs()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from("Enter")),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from("to search")),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .px(px(6.0))
                                                    .py(px(2.0))
                                                    .rounded(px(2.0))
                                                    .bg(colors.surface_elevated)
                                                    .text_xs()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from("Esc")),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from("to close")),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn render_settings_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let active_tab = self.settings_active_tab;

        let dismiss_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        // Block mouse events from passing through the backdrop
        let block_mouse_move = cx.listener(|_, _: &MouseMoveEvent, _, cx| {
            cx.stop_propagation();
        });
        let block_mouse_down = cx.listener(|_, _: &MouseDownEvent, _, cx| {
            cx.stop_propagation();
        });

        div()
            .id("settings-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .on_click(dismiss_handler)
            .on_mouse_move(block_mouse_move)
            .on_mouse_down(MouseButton::Left, block_mouse_down)
            .child(
                div()
                    .id("settings-content")
                    .w(px(800.0))
                    .h(px(560.0))
                    .rounded(px(8.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .overflow_hidden()
                    .flex()
                    .on_click(content_click)
                    // Sidebar
                    .child(
                        div()
                            .w(px(200.0))
                            .h_full()
                            .bg(colors.background)
                            .border_r_1()
                            .border_color(colors.border)
                            .py(px(12.0))
                            .child(
                                div()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from("Settings")),
                            )
                            .child(
                                div()
                                    .mt(px(8.0))
                                    .child(self.render_settings_tab(
                                        "General",
                                        SettingsTab::General,
                                        active_tab,
                                        cx,
                                    ))
                                    .child(self.render_settings_tab(
                                        "Accounts",
                                        SettingsTab::Accounts,
                                        active_tab,
                                        cx,
                                    ))
                                    .child(self.render_settings_tab(
                                        "AI Features",
                                        SettingsTab::AiFeatures,
                                        active_tab,
                                        cx,
                                    ))
                                    .child(self.render_settings_tab(
                                        "Keyboard Shortcuts",
                                        SettingsTab::KeyboardShortcuts,
                                        active_tab,
                                        cx,
                                    ))
                                    .child(self.render_settings_tab(
                                        "Appearance",
                                        SettingsTab::Appearance,
                                        active_tab,
                                        cx,
                                    )),
                            ),
                    )
                    // Content
                    .child(
                        div()
                            .id("settings-content-scroll")
                            .flex_1()
                            .h_full()
                            .overflow_y_scroll()
                            .child(self.render_settings_content(active_tab, cx)),
                    ),
            )
    }

    fn render_settings_tab(
        &self,
        label: &str,
        tab: SettingsTab,
        active_tab: SettingsTab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_active = active_tab == tab;
        let bg = if is_active {
            colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.settings_active_tab = tab;
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("tab-{:?}", tab)))
            .px(px(16.0))
            .py(px(8.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(colors.surface_elevated))
            .text_sm()
            .text_color(colors.text_primary)
            .on_click(handler)
            .child(SharedString::from(label.to_string()))
    }

    fn render_settings_content(&self, tab: SettingsTab, cx: &mut Context<Self>) -> AnyElement {
        match tab {
            SettingsTab::General => self.render_settings_general(cx).into_any_element(),
            SettingsTab::Accounts => self.render_settings_accounts(cx).into_any_element(),
            SettingsTab::AiFeatures => self.render_settings_ai(cx).into_any_element(),
            SettingsTab::KeyboardShortcuts => self.render_settings_keybindings().into_any_element(),
            SettingsTab::Appearance => self.render_settings_appearance(cx).into_any_element(),
        }
    }

    fn render_settings_general(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .p(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .mb(px(20.0))
                    .child(SharedString::from("General Settings")),
            )
            // Notifications section
            .child(
                div()
                    .mb(px(24.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Notifications")),
                    )
                    .child(self.render_general_toggle(
                        "Desktop notifications",
                        GeneralToggle::DesktopNotifications,
                        self.settings_desktop_notifications,
                        cx,
                    ))
                    .child(self.render_general_toggle(
                        "Sound alerts",
                        GeneralToggle::SoundAlerts,
                        self.settings_sound_alerts,
                        cx,
                    ))
                    .child(self.render_general_toggle(
                        "Badge count in dock",
                        GeneralToggle::BadgeCount,
                        self.settings_badge_count,
                        cx,
                    )),
            )
            // Behavior section
            .child(
                div()
                    .mb(px(24.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Behavior")),
                    )
                    .child(self.render_general_toggle(
                        "Launch at startup",
                        GeneralToggle::LaunchAtStartup,
                        self.settings_launch_at_startup,
                        cx,
                    ))
                    .child(self.render_general_toggle(
                        "Show in menu bar",
                        GeneralToggle::ShowInMenuBar,
                        self.settings_show_in_menu_bar,
                        cx,
                    ))
                    .child(self.render_general_toggle(
                        "Auto-archive after reply",
                        GeneralToggle::AutoArchiveAfterReply,
                        self.settings_auto_archive_after_reply,
                        cx,
                    )),
            )
            // Reading section
            .child(
                div()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Reading")),
                    )
                    .child(self.render_general_toggle(
                        "Mark as read when opened",
                        GeneralToggle::MarkAsReadWhenOpened,
                        self.settings_mark_as_read_when_opened,
                        cx,
                    ))
                    .child(self.render_general_toggle(
                        "Show conversation view",
                        GeneralToggle::ShowConversationView,
                        self.settings_show_conversation_view,
                        cx,
                    )),
            )
    }

    fn render_general_toggle(
        &self,
        label: &str,
        toggle: GeneralToggle,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let accent = colors.accent;
        let surface_elevated = colors.surface_elevated;
        let text_secondary = colors.text_secondary;
        let text_primary = colors.text_primary;

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            match toggle {
                GeneralToggle::DesktopNotifications => {
                    this.settings_desktop_notifications = !this.settings_desktop_notifications;
                }
                GeneralToggle::SoundAlerts => {
                    this.settings_sound_alerts = !this.settings_sound_alerts;
                }
                GeneralToggle::BadgeCount => {
                    this.settings_badge_count = !this.settings_badge_count;
                }
                GeneralToggle::LaunchAtStartup => {
                    this.settings_launch_at_startup = !this.settings_launch_at_startup;
                }
                GeneralToggle::ShowInMenuBar => {
                    this.settings_show_in_menu_bar = !this.settings_show_in_menu_bar;
                }
                GeneralToggle::AutoArchiveAfterReply => {
                    this.settings_auto_archive_after_reply =
                        !this.settings_auto_archive_after_reply;
                }
                GeneralToggle::MarkAsReadWhenOpened => {
                    this.settings_mark_as_read_when_opened =
                        !this.settings_mark_as_read_when_opened;
                }
                GeneralToggle::ShowConversationView => {
                    this.settings_show_conversation_view = !this.settings_show_conversation_view;
                }
            }
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("toggle-{:?}", toggle)))
            .py(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .on_click(handler)
            .child(
                div()
                    .text_sm()
                    .text_color(text_secondary)
                    .child(SharedString::from(label.to_string())),
            )
            .child(
                div()
                    .w(px(40.0))
                    .h(px(22.0))
                    .rounded(px(11.0))
                    .bg(if enabled { accent } else { surface_elevated })
                    .flex()
                    .items_center()
                    .px(px(2.0))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded_full()
                            .bg(text_primary)
                            .when(enabled, |this| this.ml(px(18.0))),
                    ),
            )
    }

    fn render_ai_toggle(
        &self,
        label: &str,
        toggle: AiToggle,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let accent = colors.accent;
        let surface_elevated = colors.surface_elevated;
        let text_secondary = colors.text_secondary;
        let text_primary = colors.text_primary;

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            match toggle {
                AiToggle::SmartCompose => {
                    this.settings_smart_compose = !this.settings_smart_compose;
                }
                AiToggle::EmailSummarization => {
                    this.settings_email_summarization = !this.settings_email_summarization;
                }
                AiToggle::PriorityInbox => {
                    this.settings_priority_inbox = !this.settings_priority_inbox;
                }
                AiToggle::SenderCategorization => {
                    this.settings_sender_categorization = !this.settings_sender_categorization;
                }
            }
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("ai-toggle-{:?}", toggle)))
            .py(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .on_click(handler)
            .child(
                div()
                    .text_sm()
                    .text_color(text_secondary)
                    .child(SharedString::from(label.to_string())),
            )
            .child(
                div()
                    .w(px(40.0))
                    .h(px(22.0))
                    .rounded(px(11.0))
                    .bg(if enabled { accent } else { surface_elevated })
                    .flex()
                    .items_center()
                    .px(px(2.0))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded_full()
                            .bg(text_primary)
                            .when(enabled, |this| this.ml(px(18.0))),
                    ),
            )
    }

    #[allow(dead_code)]
    fn render_settings_toggle(&self, label: &str, enabled: bool) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .py(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_sm()
                    .text_color(colors.text_secondary)
                    .child(SharedString::from(label.to_string())),
            )
            .child(
                div()
                    .w(px(40.0))
                    .h(px(22.0))
                    .rounded(px(11.0))
                    .cursor_pointer()
                    .bg(if enabled {
                        colors.accent
                    } else {
                        colors.surface_elevated
                    })
                    .flex()
                    .items_center()
                    .px(px(2.0))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded_full()
                            .bg(colors.text_primary)
                            .when(enabled, |this| this.ml(px(18.0))),
                    ),
            )
    }

    fn render_settings_accounts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let add_gmail = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
            this.account_setup_mode = AccountSetupMode::Gmail;
            this.show_overlay(ActiveOverlay::AccountSetup, cx);
        });

        let add_imap = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
            this.account_setup_mode = AccountSetupMode::Imap;
            this.show_overlay(ActiveOverlay::AccountSetup, cx);
        });

        div()
            .p(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .mb(px(20.0))
                    .child(SharedString::from("Email Accounts")),
            )
            // No accounts placeholder
            .child(
                div()
                    .p(px(24.0))
                    .rounded(px(8.0))
                    .bg(colors.background)
                    .border_1()
                    .border_color(colors.border)
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .text_color(colors.text_muted)
                            .mb(px(16.0))
                            .child(SharedString::from("No accounts connected")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_secondary)
                            .mb(px(20.0))
                            .child(SharedString::from(
                                "Connect your email accounts to get started",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .id("add-gmail")
                                    .px(px(16.0))
                                    .py(px(10.0))
                                    .rounded(px(6.0))
                                    .bg(colors.accent)
                                    .text_color(colors.text_primary)
                                    .text_sm()
                                    .cursor_pointer()
                                    .on_click(add_gmail)
                                    .child(SharedString::from("Connect Gmail")),
                            )
                            .child(
                                div()
                                    .id("add-imap")
                                    .px(px(16.0))
                                    .py(px(10.0))
                                    .rounded(px(6.0))
                                    .bg(colors.surface_elevated)
                                    .text_color(colors.text_primary)
                                    .text_sm()
                                    .cursor_pointer()
                                    .hover(move |style| style.bg(colors.border))
                                    .on_click(add_imap)
                                    .child(SharedString::from("Connect IMAP")),
                            ),
                    ),
            )
    }

    fn render_settings_ai(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .p(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .mb(px(20.0))
                    .child(SharedString::from("AI Features")),
            )
            // AI toggles
            .child(
                div()
                    .mb(px(24.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Smart Features")),
                    )
                    .child(self.render_ai_toggle(
                        "Smart compose suggestions",
                        AiToggle::SmartCompose,
                        self.settings_smart_compose,
                        cx,
                    ))
                    .child(self.render_ai_toggle(
                        "Email summarization",
                        AiToggle::EmailSummarization,
                        self.settings_email_summarization,
                        cx,
                    ))
                    .child(self.render_ai_toggle(
                        "Priority inbox sorting",
                        AiToggle::PriorityInbox,
                        self.settings_priority_inbox,
                        cx,
                    ))
                    .child(self.render_ai_toggle(
                        "Sender categorization",
                        AiToggle::SenderCategorization,
                        self.settings_sender_categorization,
                        cx,
                    )),
            )
            // AI Provider
            .child(
                div()
                    .mb(px(24.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("AI Provider")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(self.render_ai_provider_option(
                                "Local (Ollama)",
                                "Privacy-first, runs on your machine",
                                AiProvider::Ollama,
                                cx,
                            ))
                            .child(self.render_ai_provider_option(
                                "OpenAI",
                                "GPT-4 and GPT-3.5 models",
                                AiProvider::OpenAi,
                                cx,
                            ))
                            .child(self.render_ai_provider_option(
                                "Anthropic",
                                "Claude models",
                                AiProvider::Anthropic,
                                cx,
                            )),
                    ),
            )
            // Privacy notice
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(colors.surface_elevated)
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_secondary)
                            .child(SharedString::from(
                                "AI features use local processing by default. Your emails never leave your device.",
                            )),
                    ),
            )
    }

    fn render_ai_provider_option(
        &self,
        name: &str,
        description: &str,
        provider: AiProvider,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_selected = self.settings_ai_provider == provider;
        let border_color = if is_selected {
            colors.accent
        } else {
            colors.border
        };
        let bg = colors.background;
        let success = colors.success;
        let text_primary = colors.text_primary;
        let text_muted = colors.text_muted;

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.settings_ai_provider = provider;
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("provider-{:?}", provider)))
            .p(px(12.0))
            .rounded(px(6.0))
            .bg(bg)
            .border_1()
            .border_color(border_color)
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_between()
            .on_click(handler)
            .child(
                div()
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_primary)
                            .child(SharedString::from(name.to_string())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child(SharedString::from(description.to_string())),
                    ),
            )
            .when(is_selected, |this| {
                this.child(
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .bg(success)
                        .text_xs()
                        .text_color(text_primary)
                        .child(SharedString::from("Active")),
                )
            })
    }

    fn render_settings_keybindings(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        let keybindings = [
            (
                "Navigation",
                vec![
                    ("j / k", "Next / Previous message"),
                    ("g i", "Go to Inbox"),
                    ("g s", "Go to Starred"),
                    ("g d", "Go to Drafts"),
                    ("g t", "Go to Sent"),
                    ("g a", "Go to Archive"),
                ],
            ),
            (
                "Actions",
                vec![
                    ("c", "Compose new email"),
                    ("r", "Reply"),
                    ("R", "Reply all"),
                    ("f", "Forward"),
                    ("e", "Archive"),
                    ("#", "Move to trash"),
                    ("s", "Star / Unstar"),
                    ("h", "Snooze"),
                    ("u / U", "Mark read / unread"),
                ],
            ),
            (
                "App",
                vec![
                    ("Cmd+K", "Command palette"),
                    ("/", "Search"),
                    ("Cmd+,", "Settings"),
                    ("Esc", "Close overlay"),
                ],
            ),
        ];

        div()
            .p(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .mb(px(20.0))
                    .child(SharedString::from("Keyboard Shortcuts")),
            )
            .children(keybindings.iter().map(|(section, bindings)| {
                div()
                    .mb(px(20.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(8.0))
                            .child(SharedString::from(section.to_string())),
                    )
                    .children(bindings.iter().map(|(key, desc)| {
                        div()
                            .py(px(6.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_secondary)
                                    .child(SharedString::from(desc.to_string())),
                            )
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(colors.surface_elevated)
                                    .text_xs()
                                    .text_color(colors.text_muted)
                                    .child(SharedString::from(key.to_string())),
                            )
                    }))
            }))
    }

    fn render_settings_appearance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .p(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .mb(px(20.0))
                    .child(SharedString::from("Appearance")),
            )
            // Theme selection
            .child(
                div()
                    .mb(px(24.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Theme")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(self.render_theme_option_interactive(
                                "Dark",
                                ThemeMode::Dark,
                                cx,
                            ))
                            .child(self.render_theme_option_interactive(
                                "Light",
                                ThemeMode::Light,
                                cx,
                            ))
                            .child(self.render_theme_option_interactive(
                                "System",
                                ThemeMode::System,
                                cx,
                            )),
                    ),
            )
            // Font size
            .child(
                div()
                    .mb(px(24.0))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Font Size")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(self.render_font_size_option_interactive(
                                "Small",
                                FontSize::Small,
                                cx,
                            ))
                            .child(self.render_font_size_option_interactive(
                                "Medium",
                                FontSize::Medium,
                                cx,
                            ))
                            .child(self.render_font_size_option_interactive(
                                "Large",
                                FontSize::Large,
                                cx,
                            )),
                    ),
            )
            // Density
            .child(
                div()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.text_primary)
                            .mb(px(12.0))
                            .child(SharedString::from("Display Density")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(self.render_density_option_interactive(
                                "Compact",
                                DisplayDensity::Compact,
                                cx,
                            ))
                            .child(self.render_density_option_interactive(
                                "Comfortable",
                                DisplayDensity::Comfortable,
                                cx,
                            ))
                            .child(self.render_density_option_interactive(
                                "Spacious",
                                DisplayDensity::Spacious,
                                cx,
                            )),
                    ),
            )
    }

    fn render_theme_option_interactive(
        &self,
        label: &str,
        mode: ThemeMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let selected = self.settings_theme_mode == mode;
        let accent = colors.accent;
        let surface_elevated = colors.surface_elevated;
        let border = colors.border;
        let text_primary = colors.text_primary;

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.settings_theme_mode = mode;
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("theme-{:?}", mode)))
            .px(px(16.0))
            .py(px(10.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .bg(if selected { accent } else { surface_elevated })
            .hover(
                move |style| {
                    if !selected {
                        style.bg(border)
                    } else {
                        style
                    }
                },
            )
            .text_sm()
            .text_color(text_primary)
            .on_click(handler)
            .child(SharedString::from(label.to_string()))
    }

    fn render_font_size_option_interactive(
        &self,
        label: &str,
        size: FontSize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let selected = self.settings_font_size == size;
        let accent = colors.accent;
        let border = colors.border;
        let surface_elevated = colors.surface_elevated;
        let text_primary = colors.text_primary;

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.settings_font_size = size;
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("font-size-{:?}", size)))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .border_1()
            .border_color(if selected { accent } else { border })
            .bg(if selected {
                surface_elevated
            } else {
                gpui::Hsla::transparent_black()
            })
            .text_sm()
            .text_color(text_primary)
            .on_click(handler)
            .child(SharedString::from(label.to_string()))
    }

    fn render_density_option_interactive(
        &self,
        label: &str,
        density: DisplayDensity,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let selected = self.settings_display_density == density;
        let accent = colors.accent;
        let border = colors.border;
        let surface_elevated = colors.surface_elevated;
        let text_primary = colors.text_primary;

        let handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.settings_display_density = density;
            cx.notify();
        });

        div()
            .id(SharedString::from(format!("density-{:?}", density)))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .border_1()
            .border_color(if selected { accent } else { border })
            .bg(if selected {
                surface_elevated
            } else {
                gpui::Hsla::transparent_black()
            })
            .text_sm()
            .text_color(text_primary)
            .on_click(handler)
            .child(SharedString::from(label.to_string()))
    }

    #[allow(dead_code)]
    fn render_theme_option(&self, label: &str, selected: bool) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .px(px(16.0))
            .py(px(10.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .bg(if selected {
                colors.accent
            } else {
                colors.surface_elevated
            })
            .hover(move |style| {
                if !selected {
                    style.bg(colors.border)
                } else {
                    style
                }
            })
            .text_sm()
            .text_color(colors.text_primary)
            .child(SharedString::from(label.to_string()))
    }

    #[allow(dead_code)]
    fn render_font_size_option(&self, label: &str, selected: bool) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .border_1()
            .border_color(if selected {
                colors.accent
            } else {
                colors.border
            })
            .bg(if selected {
                colors.surface_elevated
            } else {
                gpui::Hsla::transparent_black()
            })
            .text_sm()
            .text_color(colors.text_primary)
            .child(SharedString::from(label.to_string()))
    }

    #[allow(dead_code)]
    fn render_density_option(&self, label: &str, selected: bool) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .border_1()
            .border_color(if selected {
                colors.accent
            } else {
                colors.border
            })
            .bg(if selected {
                colors.surface_elevated
            } else {
                gpui::Hsla::transparent_black()
            })
            .text_sm()
            .text_color(colors.text_primary)
            .child(SharedString::from(label.to_string()))
    }

    fn render_composer_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let active_field = self.composer_active_field;

        let dismiss_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        let send_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.send_email(cx);
        });

        let discard_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        let close_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        let toggle_cc = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_show_cc = !this.composer_show_cc;
            if this.composer_show_cc {
                this.composer_active_field = ComposerField::Cc;
            }
            cx.notify();
        });

        let toggle_bcc = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_show_bcc = !this.composer_show_bcc;
            if this.composer_show_bcc {
                this.composer_active_field = ComposerField::Bcc;
            }
            cx.notify();
        });

        // Click handlers to focus specific fields
        let focus_to = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_active_field = ComposerField::To;
            cx.notify();
        });
        let focus_cc = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_active_field = ComposerField::Cc;
            cx.notify();
        });
        let focus_bcc = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_active_field = ComposerField::Bcc;
            cx.notify();
        });
        let focus_subject = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_active_field = ComposerField::Subject;
            cx.notify();
        });
        let focus_body = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.composer_active_field = ComposerField::Body;
            cx.notify();
        });

        // Field styling helpers
        let field_bg = |is_active: bool| {
            if is_active {
                colors.surface_elevated
            } else {
                gpui::Hsla::transparent_black()
            }
        };

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        // Block mouse events from passing through the backdrop
        let block_mouse_move = cx.listener(|_, _: &MouseMoveEvent, _, cx| {
            cx.stop_propagation();
        });
        let block_mouse_down = cx.listener(|_, _: &MouseDownEvent, _, cx| {
            cx.stop_propagation();
        });

        div()
            .id("composer-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .on_click(dismiss_handler)
            .on_mouse_move(block_mouse_move)
            .on_mouse_down(MouseButton::Left, block_mouse_down)
            .child(
                div()
                    .id("composer-content")
                    .w(px(680.0))
                    .h(px(520.0))
                    .rounded(px(8.0))
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .on_click(content_click)
                    // Header
                    .child(
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .border_b_1()
                            .border_color(colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from("New Message")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(colors.text_muted)
                                            .child(SharedString::from("Cmd+Enter to send")),
                                    )
                                    .child(
                                        div()
                                            .id("close-composer")
                                            .px(px(8.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .hover(move |style| style.bg(colors.surface_elevated))
                                            .text_color(colors.text_muted)
                                            .on_click(close_handler)
                                            .child(SharedString::from("X")),
                                    ),
                            ),
                    )
                    // To field
                    .child(
                        div()
                            .id("to-field")
                            .px(px(16.0))
                            .py(px(10.0))
                            .bg(field_bg(active_field == ComposerField::To))
                            .border_b_1()
                            .border_color(colors.border)
                            .cursor_text()
                            .on_click(focus_to)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .w(px(60.0))
                                            .text_sm()
                                            .text_color(colors.text_muted)
                                            .child(SharedString::from("To:")),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_color(if self.composer_to.is_empty() {
                                                colors.text_muted
                                            } else {
                                                colors.text_primary
                                            })
                                            .child(SharedString::from(
                                                if self.composer_to.is_empty() {
                                                    "Recipients".to_string()
                                                } else {
                                                    self.composer_to.text().to_string()
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .when(!self.composer_show_cc, |this| {
                                                this.child(
                                                    div()
                                                        .id("add-cc")
                                                        .px(px(6.0))
                                                        .py(px(2.0))
                                                        .rounded(px(4.0))
                                                        .cursor_pointer()
                                                        .hover(move |style| {
                                                            style.bg(colors.surface_elevated)
                                                        })
                                                        .text_xs()
                                                        .text_color(colors.text_muted)
                                                        .on_click(toggle_cc)
                                                        .child(SharedString::from("Cc")),
                                                )
                                            })
                                            .when(!self.composer_show_bcc, |this| {
                                                this.child(
                                                    div()
                                                        .id("add-bcc")
                                                        .px(px(6.0))
                                                        .py(px(2.0))
                                                        .rounded(px(4.0))
                                                        .cursor_pointer()
                                                        .hover(move |style| {
                                                            style.bg(colors.surface_elevated)
                                                        })
                                                        .text_xs()
                                                        .text_color(colors.text_muted)
                                                        .on_click(toggle_bcc)
                                                        .child(SharedString::from("Bcc")),
                                                )
                                            }),
                                    ),
                            ),
                    )
                    // Cc field (conditional)
                    .when(self.composer_show_cc, |this| {
                        this.child(
                            div()
                                .id("cc-field")
                                .px(px(16.0))
                                .py(px(10.0))
                                .bg(field_bg(active_field == ComposerField::Cc))
                                .border_b_1()
                                .border_color(colors.border)
                                .cursor_text()
                                .on_click(focus_cc)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .w(px(60.0))
                                                .text_sm()
                                                .text_color(colors.text_muted)
                                                .child(SharedString::from("Cc:")),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_color(if self.composer_cc.is_empty() {
                                                    colors.text_muted
                                                } else {
                                                    colors.text_primary
                                                })
                                                .child(SharedString::from(
                                                    if self.composer_cc.is_empty() {
                                                        "Carbon copy".to_string()
                                                    } else {
                                                        self.composer_cc.text().to_string()
                                                    },
                                                )),
                                        ),
                                ),
                        )
                    })
                    // Bcc field (conditional)
                    .when(self.composer_show_bcc, |this| {
                        this.child(
                            div()
                                .id("bcc-field")
                                .px(px(16.0))
                                .py(px(10.0))
                                .bg(field_bg(active_field == ComposerField::Bcc))
                                .border_b_1()
                                .border_color(colors.border)
                                .cursor_text()
                                .on_click(focus_bcc)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .w(px(60.0))
                                                .text_sm()
                                                .text_color(colors.text_muted)
                                                .child(SharedString::from("Bcc:")),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_color(if self.composer_bcc.is_empty() {
                                                    colors.text_muted
                                                } else {
                                                    colors.text_primary
                                                })
                                                .child(SharedString::from(
                                                    if self.composer_bcc.is_empty() {
                                                        "Blind carbon copy".to_string()
                                                    } else {
                                                        self.composer_bcc.text().to_string()
                                                    },
                                                )),
                                        ),
                                ),
                        )
                    })
                    // Subject field
                    .child(
                        div()
                            .id("subject-field")
                            .px(px(16.0))
                            .py(px(10.0))
                            .bg(field_bg(active_field == ComposerField::Subject))
                            .border_b_1()
                            .border_color(colors.border)
                            .cursor_text()
                            .on_click(focus_subject)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .w(px(60.0))
                                            .text_sm()
                                            .text_color(colors.text_muted)
                                            .child(SharedString::from("Subject:")),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_color(if self.composer_subject.is_empty() {
                                                colors.text_muted
                                            } else {
                                                colors.text_primary
                                            })
                                            .child(SharedString::from(
                                                if self.composer_subject.is_empty() {
                                                    "Enter subject".to_string()
                                                } else {
                                                    self.composer_subject.text().to_string()
                                                },
                                            )),
                                    ),
                            ),
                    )
                    // Body
                    .child(
                        div()
                            .id("body-field")
                            .flex_1()
                            .p(px(16.0))
                            .bg(field_bg(active_field == ComposerField::Body))
                            .cursor_text()
                            .on_click(focus_body)
                            .overflow_y_scroll()
                            .child(
                                div()
                                    .text_color(if self.composer_body.is_empty() {
                                        colors.text_muted
                                    } else {
                                        colors.text_primary
                                    })
                                    .child(SharedString::from(if self.composer_body.is_empty() {
                                        "Compose your message...".to_string()
                                    } else {
                                        self.composer_body.text().to_string()
                                    })),
                            ),
                    )
                    // Footer
                    .child(
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .border_t_1()
                            .border_color(colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .id("send-btn")
                                            .px(px(16.0))
                                            .py(px(8.0))
                                            .rounded(px(6.0))
                                            .bg(colors.accent)
                                            .text_color(colors.text_primary)
                                            .cursor_pointer()
                                            .on_click(send_handler)
                                            .child(SharedString::from("Send")),
                                    )
                                    .child(
                                        div()
                                            .id("discard-btn")
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .rounded(px(6.0))
                                            .cursor_pointer()
                                            .hover(move |style| style.bg(colors.surface_elevated))
                                            .text_color(colors.text_muted)
                                            .on_click(discard_handler)
                                            .child(SharedString::from("Discard")),
                                    ),
                            )
                            .child(
                                div().flex().items_center().gap(px(12.0)).child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.text_muted)
                                        .child(SharedString::from("Tab to navigate")),
                                ),
                            ),
                    ),
            )
    }

    fn render_account_setup_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dismiss_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dismiss_overlay(cx);
        });

        // Block mouse events from passing through the backdrop
        let block_mouse_move = cx.listener(|_, _: &MouseMoveEvent, _, cx| {
            cx.stop_propagation();
        });
        let block_mouse_down = cx.listener(|_, _: &MouseDownEvent, _, cx| {
            cx.stop_propagation();
        });

        div()
            .id("account-setup-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .on_click(dismiss_handler)
            .on_mouse_move(block_mouse_move)
            .on_mouse_down(MouseButton::Left, block_mouse_down)
            .child(match self.account_setup_mode {
                AccountSetupMode::Selection => self.render_account_selection(cx).into_any_element(),
                AccountSetupMode::Gmail => self.render_gmail_setup(cx).into_any_element(),
                AccountSetupMode::Imap => self.render_imap_setup(cx).into_any_element(),
            })
    }

    fn render_account_selection(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        let select_gmail = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.account_setup_mode = AccountSetupMode::Gmail;
            cx.notify();
        });

        let select_imap = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.account_setup_mode = AccountSetupMode::Imap;
            cx.notify();
        });

        div()
            .id("account-selection-content")
            .w(px(500.0))
            .rounded(px(8.0))
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .on_click(content_click)
            // Header
            .child(
                div()
                    .px(px(24.0))
                    .py(px(16.0))
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_primary)
                            .child(SharedString::from("Add Email Account")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_secondary)
                            .mt(px(4.0))
                            .child(SharedString::from(
                                "Choose your email provider to get started",
                            )),
                    ),
            )
            // Options
            .child(
                div()
                    .p(px(24.0))
                    .child(
                        div()
                            .id("gmail-option")
                            .p(px(16.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(colors.border)
                            .cursor_pointer()
                            .hover(move |style| style.bg(colors.surface_elevated))
                            .on_click(select_gmail)
                            .mb(px(12.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(16.0))
                                    .child(
                                        div()
                                            .size(px(48.0))
                                            .rounded(px(8.0))
                                            .bg(colors.surface_elevated)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_lg()
                                            .text_color(colors.text_primary)
                                            .child(SharedString::from("G")),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(colors.text_primary)
                                                    .child(SharedString::from("Google / Gmail")),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from(
                                                        "Sign in with your Google account",
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .bg(colors.success)
                                            .text_xs()
                                            .text_color(colors.text_primary)
                                            .child(SharedString::from("Recommended")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("imap-option")
                            .p(px(16.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(colors.border)
                            .cursor_pointer()
                            .hover(move |style| style.bg(colors.surface_elevated))
                            .on_click(select_imap)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(16.0))
                                    .child(
                                        div()
                                            .size(px(48.0))
                                            .rounded(px(8.0))
                                            .bg(colors.surface_elevated)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_lg()
                                            .text_color(colors.text_primary)
                                            .child(SharedString::from("@")),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(colors.text_primary)
                                                    .child(SharedString::from("IMAP / SMTP")),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(colors.text_muted)
                                                    .child(SharedString::from(
                                                        "Connect any email provider with IMAP",
                                                    )),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn render_gmail_setup(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        let back_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.account_setup_mode = AccountSetupMode::Selection;
            cx.notify();
        });

        let connect_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            tracing::info!("Starting Gmail OAuth flow...");
            // TODO: Trigger actual OAuth flow
            this.dismiss_overlay(cx);
        });

        div()
            .id("gmail-setup-content")
            .w(px(480.0))
            .rounded(px(8.0))
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .on_click(content_click)
            // Header
            .child(
                div()
                    .px(px(24.0))
                    .py(px(16.0))
                    .border_b_1()
                    .border_color(colors.border)
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .id("back-btn")
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(move |style| style.bg(colors.surface_elevated))
                            .text_color(colors.text_muted)
                            .on_click(back_handler)
                            .child(SharedString::from("<")),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_primary)
                            .child(SharedString::from("Connect Gmail")),
                    ),
            )
            // Content
            .child(
                div()
                    .p(px(24.0))
                    .child(
                        div()
                            .text_center()
                            .child(
                                div()
                                    .size(px(64.0))
                                    .rounded_full()
                                    .bg(colors.surface_elevated)
                                    .mx_auto()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_2xl()
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from("G")),
                            )
                            .child(
                                div()
                                    .mt(px(16.0))
                                    .text_color(colors.text_primary)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(SharedString::from("Sign in with Google")),
                            )
                            .child(
                                div()
                                    .mt(px(8.0))
                                    .text_sm()
                                    .text_color(colors.text_secondary)
                                    .child(SharedString::from(
                                        "You'll be redirected to Google to authorize The Heap to access your email.",
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(24.0))
                            .p(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.background)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_secondary)
                                    .child(SharedString::from("The Heap will request permission to:")),
                            )
                            .child(
                                div()
                                    .mt(px(8.0))
                                    .text_sm()
                                    .text_color(colors.text_muted)
                                    .child(SharedString::from("- Read, send, and manage your email")),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_muted)
                                    .child(SharedString::from("- Manage labels and filters")),
                            ),
                    ),
            )
            // Footer
            .child(
                div()
                    .px(px(24.0))
                    .py(px(16.0))
                    .border_t_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .id("connect-gmail-btn")
                            .w_full()
                            .py(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.accent)
                            .text_color(colors.text_primary)
                            .text_center()
                            .cursor_pointer()
                            .on_click(connect_handler)
                            .child(SharedString::from("Continue with Google")),
                    ),
            )
    }

    fn render_imap_setup(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let active_field = self.imap_active_field;

        let content_click = cx.listener(|_, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        });

        let back_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.account_setup_mode = AccountSetupMode::Selection;
            cx.notify();
        });

        let connect_handler = cx.listener(|this, _: &ClickEvent, _, cx| {
            let server = this.imap_server.text().to_string();
            let port = this.imap_port.text().to_string();
            let username = this.imap_username.text().to_string();

            if server.is_empty() || username.is_empty() {
                tracing::warn!("IMAP server and username are required");
                return;
            }

            tracing::info!(
                "Connecting to IMAP server: {}:{} as {}",
                server,
                port,
                username
            );
            // TODO: Actually connect to IMAP server
            this.dismiss_overlay(cx);
        });

        // Click handlers for each field
        let focus_imap_server = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.imap_active_field = ImapField::ImapServer;
            cx.notify();
        });
        let focus_imap_port = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.imap_active_field = ImapField::ImapPort;
            cx.notify();
        });
        let focus_smtp_server = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.imap_active_field = ImapField::SmtpServer;
            cx.notify();
        });
        let focus_smtp_port = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.imap_active_field = ImapField::SmtpPort;
            cx.notify();
        });
        let focus_username = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.imap_active_field = ImapField::Username;
            cx.notify();
        });
        let focus_password = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.imap_active_field = ImapField::Password;
            cx.notify();
        });

        div()
            .id("imap-setup-content")
            .w(px(520.0))
            .rounded(px(8.0))
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .on_click(content_click)
            // Header
            .child(
                div()
                    .px(px(24.0))
                    .py(px(16.0))
                    .border_b_1()
                    .border_color(colors.border)
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .id("back-btn-imap")
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(move |style| style.bg(colors.surface_elevated))
                            .text_color(colors.text_muted)
                            .on_click(back_handler)
                            .child(SharedString::from("<")),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_primary)
                            .child(SharedString::from("IMAP Configuration")),
                    ),
            )
            // Form
            .child(
                div()
                    .id("account-setup-scroll")
                    .p(px(24.0))
                    .max_h(px(400.0))
                    .overflow_y_scroll()
                    // IMAP Server section
                    .child(
                        div()
                            .mb(px(20.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.text_primary)
                                    .mb(px(12.0))
                                    .child(SharedString::from("Incoming Mail (IMAP)")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(12.0))
                                    .child(
                                        self.render_imap_field_with_handler(
                                            "imap-server",
                                            "Server",
                                            &self.imap_server,
                                            "imap.example.com",
                                            true,
                                            active_field == ImapField::ImapServer,
                                            focus_imap_server,
                                        ),
                                    )
                                    .child(
                                        self.render_imap_field_with_handler(
                                            "imap-port",
                                            "Port",
                                            &self.imap_port,
                                            "993",
                                            false,
                                            active_field == ImapField::ImapPort,
                                            focus_imap_port,
                                        ),
                                    ),
                            ),
                    )
                    // SMTP Server section
                    .child(
                        div()
                            .mb(px(20.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.text_primary)
                                    .mb(px(12.0))
                                    .child(SharedString::from("Outgoing Mail (SMTP)")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(12.0))
                                    .child(
                                        self.render_imap_field_with_handler(
                                            "smtp-server",
                                            "Server",
                                            &self.smtp_server,
                                            "smtp.example.com",
                                            true,
                                            active_field == ImapField::SmtpServer,
                                            focus_smtp_server,
                                        ),
                                    )
                                    .child(
                                        self.render_imap_field_with_handler(
                                            "smtp-port",
                                            "Port",
                                            &self.smtp_port,
                                            "587",
                                            false,
                                            active_field == ImapField::SmtpPort,
                                            focus_smtp_port,
                                        ),
                                    ),
                            ),
                    )
                    // Credentials section
                    .child(
                        div()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.text_primary)
                                    .mb(px(12.0))
                                    .child(SharedString::from("Credentials")),
                            )
                            .child(
                                div()
                                    .mb(px(12.0))
                                    .child(self.render_imap_field_with_handler(
                                        "username",
                                        "Email / Username",
                                        &self.imap_username,
                                        "you@example.com",
                                        true,
                                        active_field == ImapField::Username,
                                        focus_username,
                                    )),
                            )
                            .child(
                                div()
                                    .child(self.render_imap_field_with_handler(
                                        "password",
                                        "Password",
                                        &self.imap_password,
                                        "App password",
                                        true,
                                        active_field == ImapField::Password,
                                        focus_password,
                                    )),
                            ),
                    )
                    // Help text
                    .child(
                        div()
                            .mt(px(16.0))
                            .p(px(12.0))
                            .rounded(px(6.0))
                            .bg(colors.background)
                            .text_sm()
                            .text_color(colors.text_muted)
                            .child(SharedString::from(
                                "For Gmail, use an App Password instead of your regular password. You can create one in your Google Account security settings.",
                            )),
                    ),
            )
            // Footer
            .child(
                div()
                    .px(px(24.0))
                    .py(px(16.0))
                    .border_t_1()
                    .border_color(colors.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.text_muted)
                            .child(SharedString::from("Tab to navigate fields")),
                    )
                    .child(
                        div()
                            .id("connect-imap-btn")
                            .px(px(20.0))
                            .py(px(10.0))
                            .rounded(px(6.0))
                            .bg(colors.accent)
                            .text_color(colors.text_primary)
                            .cursor_pointer()
                            .on_click(connect_handler)
                            .child(SharedString::from("Connect Account")),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_imap_field_with_handler(
        &self,
        id: &str,
        label: &str,
        buffer: &TextBuffer,
        placeholder: &str,
        wide: bool,
        is_active: bool,
        handler: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id(SharedString::from(id.to_string()))
            .when(wide, |this| this.flex_1())
            .when(!wide, |this| this.w(px(100.0)))
            .cursor_text()
            .on_click(handler)
            .child(
                div()
                    .text_sm()
                    .text_color(colors.text_secondary)
                    .mb(px(4.0))
                    .child(SharedString::from(label.to_string())),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(10.0))
                    .rounded(px(6.0))
                    .bg(if is_active {
                        colors.surface_elevated
                    } else {
                        colors.background
                    })
                    .border_1()
                    .border_color(if is_active {
                        colors.accent
                    } else {
                        colors.border
                    })
                    .text_sm()
                    .text_color(if buffer.is_empty() {
                        colors.text_muted
                    } else {
                        colors.text_primary
                    })
                    .child(SharedString::from(if buffer.is_empty() {
                        placeholder.to_string()
                    } else if label == "Password" && !buffer.is_empty() {
                        "*".repeat(buffer.text().len())
                    } else {
                        buffer.text().to_string()
                    })),
            )
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or(text);
    if first_line.len() <= max_len {
        first_line.to_string()
    } else {
        format!("{}...", &first_line[..max_len])
    }
}

impl Focusable for MainWindow {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let has_overlay = self.active_overlay != ActiveOverlay::None;

        div()
            .id("main-window")
            .key_context("MainWindow")
            // Only enable single-letter keybindings when no overlay is active
            .when(!has_overlay, |div| div.key_context("EmailActions"))
            .track_focus(&self.focus_handle)
            // Handle text input for overlays
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if matches!(
                    this.active_overlay,
                    ActiveOverlay::CommandPalette
                        | ActiveOverlay::Search
                        | ActiveOverlay::Composer
                        | ActiveOverlay::AccountSetup
                ) {
                    this.handle_overlay_key(event, cx);
                }
            }))
            // Dismiss overlay
            .on_action(cx.listener(|this, _: &Dismiss, _, cx| {
                if this.active_overlay != ActiveOverlay::None {
                    this.dismiss_overlay(cx);
                }
            }))
            // Undo
            .on_action(cx.listener(|this, _: &Undo, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.undo_last(cx);
                }
            }))
            // Navigation
            .on_action(cx.listener(|this, _: &NextMessage, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.focus_next(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &PreviousMessage, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.focus_previous(cx);
                }
            }))
            // Overlays (Cmd+key works always, single-letter only when no overlay)
            .on_action(cx.listener(|this, _: &OpenCommandPalette, _, cx| {
                this.toggle_overlay(ActiveOverlay::CommandPalette, cx);
            }))
            .on_action(cx.listener(|this, _: &Search, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.show_overlay(ActiveOverlay::Search, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| {
                this.toggle_overlay(ActiveOverlay::Settings, cx);
            }))
            .on_action(cx.listener(|this, _: &Compose, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.show_overlay(ActiveOverlay::Composer, cx);
                }
            }))
            // Email actions
            .on_action(cx.listener(|this, _: &Archive, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.archive_selected(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Trash, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.trash_selected(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Star, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.star_selected(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Snooze, _, cx| {
                if this.active_overlay == ActiveOverlay::None && this.selected_thread_id.is_some() {
                    this.snooze_selected_index = 0;
                    this.show_overlay(ActiveOverlay::SnoozePicker, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ApplyLabel, _, cx| {
                if this.active_overlay == ActiveOverlay::None && this.selected_thread_id.is_some() {
                    this.label_picker_selected.clear();
                    this.show_overlay(ActiveOverlay::LabelPicker, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &MarkRead, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.mark_read_selected(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &MarkUnread, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.mark_unread_selected(cx);
                }
            }))
            // View navigation (only when no overlay)
            .on_action(cx.listener(|this, _: &GoToInbox, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Inbox, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &GoToStarred, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Starred, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &GoToDrafts, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Drafts, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &GoToSent, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Sent, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &GoToArchive, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Archive, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &GoToScreener, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Screener, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &GoToStats, _, cx| {
                if this.active_overlay == ActiveOverlay::None {
                    this.navigate_to(ViewType::Stats, cx);
                }
            }))
            // Screener actions (only in screener view with no overlay)
            .on_action(cx.listener(|this, _: &ScreenerApprove, _, cx| {
                if this.active_overlay == ActiveOverlay::None
                    && this.current_view == ViewType::Screener
                {
                    if let Some(entry) = this.screener_entries.get(this.screener_selected_index) {
                        let id = entry.id.clone();
                        this.approve_screener_entry(&id);
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|this, _: &ScreenerReject, _, cx| {
                if this.active_overlay == ActiveOverlay::None
                    && this.current_view == ViewType::Screener
                {
                    if let Some(entry) = this.screener_entries.get(this.screener_selected_index) {
                        let id = entry.id.clone();
                        this.reject_screener_entry(&id);
                        cx.notify();
                    }
                }
            }))
            // Reply/Forward (show composer with context, only when no overlay)
            .on_action(cx.listener(|this, _: &Reply, _, cx| {
                if this.active_overlay == ActiveOverlay::None && this.selected_thread_id.is_some() {
                    this.show_overlay(ActiveOverlay::Composer, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ReplyAll, _, cx| {
                if this.active_overlay == ActiveOverlay::None && this.selected_thread_id.is_some() {
                    this.show_overlay(ActiveOverlay::Composer, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Forward, _, cx| {
                if this.active_overlay == ActiveOverlay::None && this.selected_thread_id.is_some() {
                    this.show_overlay(ActiveOverlay::Composer, cx);
                }
            }))
            .size_full()
            // Handle resize drag - mouse move
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                if let Some(handle) = this.resize_dragging {
                    let current_x = f32::from(event.position.x);
                    let delta = current_x - this.resize_start_x;
                    let new_width = (this.resize_start_width + delta).clamp(150.0, 600.0);

                    match handle {
                        ResizeHandle::Sidebar => this.sidebar_width = new_width,
                        ResizeHandle::MessageList => this.message_list_width = new_width,
                    }
                    cx.notify();
                }
            }))
            // Handle resize drag - mouse up
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                    if this.resize_dragging.is_some() {
                        this.resize_dragging = None;
                        cx.notify();
                    }
                }),
            )
            .flex()
            .flex_col()
            .bg(colors.background)
            .text_color(colors.text_primary)
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(self.render_sidebar(cx))
                    .child(self.render_resize_handle(ResizeHandle::Sidebar, cx))
                    .when(self.current_view == ViewType::Screener, |this| {
                        this.child(self.render_screener_view(cx))
                    })
                    .when(self.current_view == ViewType::Stats, |this| {
                        this.child(self.render_stats_view(cx))
                    })
                    .when(
                        self.current_view != ViewType::Screener
                            && self.current_view != ViewType::Stats,
                        |this| {
                            this.child(self.render_message_list(cx))
                                .child(self.render_resize_handle(ResizeHandle::MessageList, cx))
                                .child(self.render_reading_pane(cx))
                        },
                    ),
            )
            .child(self.render_status_bar())
            // Render toast notification
            .when(self.toast.is_some(), |this| {
                this.child(self.render_toast(cx))
            })
            // Render active overlay on top
            .when(has_overlay, |this| match self.active_overlay {
                ActiveOverlay::CommandPalette => this.child(self.render_command_palette(cx)),
                ActiveOverlay::Search => this.child(self.render_search_overlay(cx)),
                ActiveOverlay::Settings => this.child(self.render_settings_overlay(cx)),
                ActiveOverlay::Composer => this.child(self.render_composer_overlay(cx)),
                ActiveOverlay::AccountSetup => this.child(self.render_account_setup_overlay(cx)),
                ActiveOverlay::SnoozePicker => this.child(self.render_snooze_picker(cx)),
                ActiveOverlay::LabelPicker => this.child(self.render_label_picker(cx)),
                ActiveOverlay::None => this,
            })
    }
}
