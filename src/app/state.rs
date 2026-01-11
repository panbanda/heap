//! Application state management.
//!
//! Centralized state for the margin application including accounts,
//! current view, selection state, and runtime status.

use std::collections::HashSet;

use crate::config::Settings;
use crate::domain::{AccountId, LabelId, ThreadId};

/// The currently active view in the application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewType {
    /// Main inbox view.
    #[default]
    Inbox,
    /// Starred messages.
    Starred,
    /// Sent messages.
    Sent,
    /// Draft messages.
    Drafts,
    /// Archived messages.
    Archive,
    /// Trash/deleted messages.
    Trash,
    /// Snoozed messages.
    Snoozed,
    /// Messages with a specific label.
    Label(LabelId),
    /// Screener queue for new senders.
    Screener,
    /// Search results.
    Search(String),
    /// Settings panel.
    Settings,
    /// Statistics dashboard.
    Stats,
}

/// Sync status for an account.
#[derive(Debug, Clone, Default)]
pub enum SyncStatus {
    /// Not currently syncing.
    #[default]
    Idle,
    /// Sync in progress.
    Syncing {
        /// Progress percentage (0-100).
        progress: u8,
    },
    /// Sync completed successfully.
    Completed {
        /// Number of new messages.
        new_messages: u32,
    },
    /// Sync failed with error.
    Failed {
        /// Error message.
        error: String,
    },
    /// Offline - cannot sync.
    Offline,
}

/// AI service status.
#[derive(Debug, Clone, Default)]
pub enum AiStatus {
    /// AI is idle.
    #[default]
    Idle,
    /// AI is processing a request.
    Processing {
        /// Description of the current task.
        task: String,
    },
    /// AI completed a task.
    Completed,
    /// AI encountered an error.
    Error {
        /// Error message.
        message: String,
    },
    /// AI is disabled.
    Disabled,
}

/// Global application state.
#[derive(Debug, Clone)]
pub struct AppState {
    /// List of configured account IDs.
    pub account_ids: Vec<AccountId>,
    /// Currently active account, if any.
    pub active_account_id: Option<AccountId>,
    /// Current view being displayed.
    pub active_view: ViewType,
    /// Currently selected thread IDs.
    pub selected_threads: HashSet<ThreadId>,
    /// Currently focused thread (cursor position).
    pub focused_thread: Option<ThreadId>,
    /// Application settings.
    pub settings: Settings,
    /// Per-account sync status.
    pub sync_status: std::collections::HashMap<AccountId, SyncStatus>,
    /// AI service status.
    pub ai_status: AiStatus,
    /// Whether the app is in offline mode.
    pub is_offline: bool,
    /// Whether the command palette is open.
    pub command_palette_open: bool,
    /// Whether the composer is open.
    pub composer_open: bool,
    /// Current search query, if any.
    pub search_query: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            account_ids: Vec::new(),
            active_account_id: None,
            active_view: ViewType::Inbox,
            selected_threads: HashSet::new(),
            focused_thread: None,
            settings: Settings::default(),
            sync_status: std::collections::HashMap::new(),
            ai_status: AiStatus::Idle,
            is_offline: false,
            command_palette_open: false,
            composer_open: false,
            search_query: None,
        }
    }
}

impl AppState {
    /// Create a new application state with the given settings.
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    /// Set the active account.
    pub fn set_active_account(&mut self, account_id: Option<AccountId>) {
        self.active_account_id = account_id;
        self.clear_selection();
    }

    /// Navigate to a view.
    pub fn navigate_to(&mut self, view: ViewType) {
        self.active_view = view;
        self.clear_selection();
    }

    /// Clear thread selection.
    pub fn clear_selection(&mut self) {
        self.selected_threads.clear();
        self.focused_thread = None;
    }

    /// Select a thread.
    pub fn select_thread(&mut self, thread_id: ThreadId) {
        self.selected_threads.insert(thread_id.clone());
        self.focused_thread = Some(thread_id);
    }

    /// Toggle thread selection.
    pub fn toggle_thread_selection(&mut self, thread_id: &ThreadId) {
        if self.selected_threads.contains(thread_id) {
            self.selected_threads.remove(thread_id);
        } else {
            self.selected_threads.insert(thread_id.clone());
        }
    }

    /// Check if a thread is selected.
    pub fn is_thread_selected(&self, thread_id: &ThreadId) -> bool {
        self.selected_threads.contains(thread_id)
    }

    /// Get the number of selected threads.
    pub fn selection_count(&self) -> usize {
        self.selected_threads.len()
    }

    /// Check if we have any selection.
    pub fn has_selection(&self) -> bool {
        !self.selected_threads.is_empty()
    }

    /// Update sync status for an account.
    pub fn set_sync_status(&mut self, account_id: AccountId, status: SyncStatus) {
        self.sync_status.insert(account_id, status);
    }

    /// Get sync status for an account.
    pub fn get_sync_status(&self, account_id: &AccountId) -> &SyncStatus {
        self.sync_status.get(account_id).unwrap_or(&SyncStatus::Idle)
    }

    /// Check if any account is currently syncing.
    pub fn is_any_syncing(&self) -> bool {
        self.sync_status
            .values()
            .any(|s| matches!(s, SyncStatus::Syncing { .. }))
    }

    /// Set AI status.
    pub fn set_ai_status(&mut self, status: AiStatus) {
        self.ai_status = status;
    }

    /// Check if AI is currently processing.
    pub fn is_ai_processing(&self) -> bool {
        matches!(self.ai_status, AiStatus::Processing { .. })
    }

    /// Open the command palette.
    pub fn open_command_palette(&mut self) {
        self.command_palette_open = true;
    }

    /// Close the command palette.
    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
    }

    /// Toggle the command palette.
    pub fn toggle_command_palette(&mut self) {
        self.command_palette_open = !self.command_palette_open;
    }

    /// Open the composer.
    pub fn open_composer(&mut self) {
        self.composer_open = true;
    }

    /// Close the composer.
    pub fn close_composer(&mut self) {
        self.composer_open = false;
    }
}

/// State for the message list view.
#[derive(Debug, Clone, Default)]
pub struct MessageListState {
    /// Index of the focused item.
    pub focused_index: usize,
    /// Scroll offset in pixels.
    pub scroll_offset: f32,
    /// Total number of items.
    pub total_items: usize,
    /// Whether we're loading more items.
    pub loading: bool,
    /// Whether there are more items to load.
    pub has_more: bool,
}

impl MessageListState {
    /// Move focus to the next item.
    pub fn focus_next(&mut self) {
        if self.focused_index + 1 < self.total_items {
            self.focused_index += 1;
        }
    }

    /// Move focus to the previous item.
    pub fn focus_previous(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
        }
    }

    /// Move focus to the first item.
    pub fn focus_first(&mut self) {
        self.focused_index = 0;
    }

    /// Move focus to the last item.
    pub fn focus_last(&mut self) {
        if self.total_items > 0 {
            self.focused_index = self.total_items - 1;
        }
    }

    /// Check if the focused item is the first.
    pub fn is_at_start(&self) -> bool {
        self.focused_index == 0
    }

    /// Check if the focused item is the last.
    pub fn is_at_end(&self) -> bool {
        self.total_items == 0 || self.focused_index == self.total_items - 1
    }
}

/// State for the reading pane.
#[derive(Debug, Clone, Default)]
pub struct ReadingPaneState {
    /// Currently displayed thread ID.
    pub thread_id: Option<ThreadId>,
    /// IDs of expanded messages in the thread.
    pub expanded_messages: HashSet<String>,
    /// Whether the inline composer is visible.
    pub composer_visible: bool,
    /// Current scroll position.
    pub scroll_offset: f32,
}

impl ReadingPaneState {
    /// Set the current thread.
    pub fn set_thread(&mut self, thread_id: Option<ThreadId>) {
        self.thread_id = thread_id;
        self.expanded_messages.clear();
        self.composer_visible = false;
        self.scroll_offset = 0.0;
    }

    /// Toggle message expansion.
    pub fn toggle_message(&mut self, message_id: &str) {
        if self.expanded_messages.contains(message_id) {
            self.expanded_messages.remove(message_id);
        } else {
            self.expanded_messages.insert(message_id.to_string());
        }
    }

    /// Expand a message.
    pub fn expand_message(&mut self, message_id: &str) {
        self.expanded_messages.insert(message_id.to_string());
    }

    /// Collapse a message.
    pub fn collapse_message(&mut self, message_id: &str) {
        self.expanded_messages.remove(message_id);
    }

    /// Expand all messages.
    pub fn expand_all(&mut self, message_ids: impl IntoIterator<Item = String>) {
        self.expanded_messages.extend(message_ids);
    }

    /// Collapse all messages.
    pub fn collapse_all(&mut self) {
        self.expanded_messages.clear();
    }

    /// Check if a message is expanded.
    pub fn is_expanded(&self, message_id: &str) -> bool {
        self.expanded_messages.contains(message_id)
    }

    /// Show the inline composer.
    pub fn show_composer(&mut self) {
        self.composer_visible = true;
    }

    /// Hide the inline composer.
    pub fn hide_composer(&mut self) {
        self.composer_visible = false;
    }
}

/// State for the composer.
#[derive(Debug, Clone, Default)]
pub struct ComposerState {
    /// Composer mode (new, reply, forward).
    pub mode: ComposerMode,
    /// Thread being replied to, if any.
    pub reply_to_thread_id: Option<ThreadId>,
    /// Message being replied to, if any.
    pub reply_to_message_id: Option<String>,
    /// To recipients.
    pub to: Vec<String>,
    /// CC recipients.
    pub cc: Vec<String>,
    /// BCC recipients.
    pub bcc: Vec<String>,
    /// Email subject.
    pub subject: String,
    /// Email body content.
    pub body: String,
    /// Attached file paths.
    pub attachments: Vec<String>,
    /// Whether the composer has unsaved changes.
    pub is_dirty: bool,
    /// Whether we're currently sending.
    pub is_sending: bool,
    /// AI draft suggestion, if any.
    pub ai_suggestion: Option<String>,
}

/// Composer mode.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ComposerMode {
    /// New email composition.
    #[default]
    New,
    /// Reply to a message.
    Reply,
    /// Reply all to a message.
    ReplyAll,
    /// Forward a message.
    Forward,
    /// Edit a draft.
    EditDraft,
}

impl ComposerState {
    /// Create a new composer for a fresh email.
    pub fn new_email() -> Self {
        Self {
            mode: ComposerMode::New,
            ..Default::default()
        }
    }

    /// Create a composer for replying to a message.
    pub fn reply(thread_id: ThreadId, message_id: String, to: String, subject: String) -> Self {
        Self {
            mode: ComposerMode::Reply,
            reply_to_thread_id: Some(thread_id),
            reply_to_message_id: Some(message_id),
            to: vec![to],
            subject: if subject.starts_with("Re:") {
                subject
            } else {
                format!("Re: {}", subject)
            },
            ..Default::default()
        }
    }

    /// Create a composer for forwarding a message.
    pub fn forward(thread_id: ThreadId, message_id: String, subject: String, body: String) -> Self {
        Self {
            mode: ComposerMode::Forward,
            reply_to_thread_id: Some(thread_id),
            reply_to_message_id: Some(message_id),
            subject: if subject.starts_with("Fwd:") {
                subject
            } else {
                format!("Fwd: {}", subject)
            },
            body,
            ..Default::default()
        }
    }

    /// Check if the composer can be sent.
    pub fn can_send(&self) -> bool {
        !self.to.is_empty() && !self.is_sending
    }

    /// Mark the composer as dirty.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Clear the dirty flag.
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Set AI suggestion.
    pub fn set_ai_suggestion(&mut self, suggestion: String) {
        self.ai_suggestion = Some(suggestion);
    }

    /// Accept AI suggestion into body.
    pub fn accept_ai_suggestion(&mut self) {
        if let Some(suggestion) = self.ai_suggestion.take() {
            self.body = suggestion;
            self.mark_dirty();
        }
    }

    /// Reject AI suggestion.
    pub fn reject_ai_suggestion(&mut self) {
        self.ai_suggestion = None;
    }

    /// Add an attachment.
    pub fn add_attachment(&mut self, path: String) {
        self.attachments.push(path);
        self.mark_dirty();
    }

    /// Remove an attachment.
    pub fn remove_attachment(&mut self, index: usize) {
        if index < self.attachments.len() {
            self.attachments.remove(index);
            self.mark_dirty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.account_ids.is_empty());
        assert!(state.active_account_id.is_none());
        assert_eq!(state.active_view, ViewType::Inbox);
        assert!(!state.has_selection());
    }

    #[test]
    fn test_thread_selection() {
        let mut state = AppState::default();
        let thread_id = ThreadId::from("thread-1");

        state.select_thread(thread_id.clone());
        assert!(state.is_thread_selected(&thread_id));
        assert_eq!(state.selection_count(), 1);

        state.toggle_thread_selection(&thread_id);
        assert!(!state.is_thread_selected(&thread_id));
        assert_eq!(state.selection_count(), 0);
    }

    #[test]
    fn test_navigation() {
        let mut state = AppState::default();
        let thread_id = ThreadId::from("thread-1");

        state.select_thread(thread_id.clone());
        assert!(state.has_selection());

        state.navigate_to(ViewType::Starred);
        assert_eq!(state.active_view, ViewType::Starred);
        assert!(!state.has_selection());
    }

    #[test]
    fn test_message_list_state() {
        let mut state = MessageListState {
            total_items: 10,
            ..Default::default()
        };

        assert!(state.is_at_start());
        state.focus_next();
        assert_eq!(state.focused_index, 1);
        assert!(!state.is_at_start());

        state.focus_last();
        assert_eq!(state.focused_index, 9);
        assert!(state.is_at_end());

        state.focus_previous();
        assert_eq!(state.focused_index, 8);
    }

    #[test]
    fn test_reading_pane_state() {
        let mut state = ReadingPaneState::default();

        state.expand_message("msg-1");
        assert!(state.is_expanded("msg-1"));

        state.toggle_message("msg-1");
        assert!(!state.is_expanded("msg-1"));

        state.expand_all(vec!["msg-1".to_string(), "msg-2".to_string()]);
        assert!(state.is_expanded("msg-1"));
        assert!(state.is_expanded("msg-2"));

        state.collapse_all();
        assert!(!state.is_expanded("msg-1"));
    }

    #[test]
    fn test_composer_state() {
        let state = ComposerState::new_email();
        assert_eq!(state.mode, ComposerMode::New);
        assert!(!state.can_send());

        let reply = ComposerState::reply(
            ThreadId::from("thread-1"),
            "msg-1".to_string(),
            "user@example.com".to_string(),
            "Test Subject".to_string(),
        );
        assert_eq!(reply.mode, ComposerMode::Reply);
        assert_eq!(reply.subject, "Re: Test Subject");
        assert!(reply.can_send());
    }

    #[test]
    fn test_composer_ai_suggestion() {
        let mut state = ComposerState::new_email();
        state.set_ai_suggestion("AI generated reply".to_string());
        assert!(state.ai_suggestion.is_some());

        state.accept_ai_suggestion();
        assert_eq!(state.body, "AI generated reply");
        assert!(state.ai_suggestion.is_none());
        assert!(state.is_dirty);
    }
}
