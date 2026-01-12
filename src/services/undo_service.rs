//! Undo/Redo service for reversible email operations.
//!
//! Supports undoing and redoing common email actions:
//! - Archive/unarchive
//! - Delete/restore
//! - Move between folders
//! - Mark read/unread
//! - Star/unstar
//! - Label changes
//! - Bulk operations

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use crate::domain::{LabelId, ThreadId};

/// Maximum number of actions to keep in history.
const MAX_HISTORY_SIZE: usize = 100;

/// Default undo window duration.
const DEFAULT_UNDO_WINDOW: Duration = Duration::from_secs(30);

/// Types of reversible actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionType {
    /// Archived threads.
    Archive,
    /// Deleted threads (moved to trash).
    Delete,
    /// Permanently deleted threads.
    PermanentDelete,
    /// Moved threads to a folder.
    Move,
    /// Marked threads as read.
    MarkRead,
    /// Marked threads as unread.
    MarkUnread,
    /// Starred threads.
    Star,
    /// Unstarred threads.
    Unstar,
    /// Added labels.
    AddLabels,
    /// Removed labels.
    RemoveLabels,
    /// Reported as spam.
    ReportSpam,
    /// Marked as not spam.
    NotSpam,
    /// Snoozed until later.
    Snooze,
    /// Sent message.
    Send,
}

impl ActionType {
    /// Returns the human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            ActionType::Archive => "archived",
            ActionType::Delete => "deleted",
            ActionType::PermanentDelete => "permanently deleted",
            ActionType::Move => "moved",
            ActionType::MarkRead => "marked as read",
            ActionType::MarkUnread => "marked as unread",
            ActionType::Star => "starred",
            ActionType::Unstar => "unstarred",
            ActionType::AddLabels => "labeled",
            ActionType::RemoveLabels => "unlabeled",
            ActionType::ReportSpam => "reported as spam",
            ActionType::NotSpam => "marked as not spam",
            ActionType::Snooze => "snoozed",
            ActionType::Send => "sent",
        }
    }

    /// Returns whether this action can be undone.
    pub fn is_undoable(&self) -> bool {
        !matches!(self, ActionType::PermanentDelete | ActionType::Send)
    }
}

/// State before an action, used for undoing.
#[derive(Debug, Clone, Default)]
pub struct ActionState {
    /// Thread IDs affected.
    pub thread_ids: Vec<ThreadId>,
    /// Original folder (for move/archive).
    pub original_folder: Option<LabelId>,
    /// Target folder (for move).
    pub target_folder: Option<LabelId>,
    /// Original labels.
    pub original_labels: Vec<LabelId>,
    /// Labels added or removed.
    pub affected_labels: Vec<LabelId>,
    /// Original read state per thread.
    pub read_states: Vec<(ThreadId, bool)>,
    /// Original starred state per thread.
    pub starred_states: Vec<(ThreadId, bool)>,
    /// Snooze until time.
    pub snooze_until: Option<DateTime<Utc>>,
}

impl ActionState {
    /// Creates state for a folder move.
    pub fn folder_move(thread_ids: Vec<ThreadId>, original: LabelId, target: LabelId) -> Self {
        Self {
            thread_ids,
            original_folder: Some(original),
            target_folder: Some(target),
            ..Default::default()
        }
    }

    /// Creates state for an archive action.
    pub fn archive(thread_ids: Vec<ThreadId>, original_folder: LabelId) -> Self {
        Self {
            thread_ids,
            original_folder: Some(original_folder),
            ..Default::default()
        }
    }

    /// Creates state for a delete action.
    pub fn delete(thread_ids: Vec<ThreadId>, original_folder: LabelId) -> Self {
        Self {
            thread_ids,
            original_folder: Some(original_folder),
            ..Default::default()
        }
    }

    /// Creates state for read state changes.
    pub fn read_state(states: Vec<(ThreadId, bool)>) -> Self {
        let thread_ids = states.iter().map(|(id, _)| id.clone()).collect();
        Self {
            thread_ids,
            read_states: states,
            ..Default::default()
        }
    }

    /// Creates state for starred state changes.
    pub fn starred_state(states: Vec<(ThreadId, bool)>) -> Self {
        let thread_ids = states.iter().map(|(id, _)| id.clone()).collect();
        Self {
            thread_ids,
            starred_states: states,
            ..Default::default()
        }
    }

    /// Creates state for label changes.
    pub fn labels(
        thread_ids: Vec<ThreadId>,
        original: Vec<LabelId>,
        affected: Vec<LabelId>,
    ) -> Self {
        Self {
            thread_ids,
            original_labels: original,
            affected_labels: affected,
            ..Default::default()
        }
    }
}

/// A recorded action that can be undone.
#[derive(Debug, Clone)]
pub struct UndoableAction {
    /// Unique ID for this action.
    pub id: String,
    /// Type of action.
    pub action_type: ActionType,
    /// State before the action.
    pub before_state: ActionState,
    /// When the action was performed.
    pub performed_at: Instant,
    /// Human-readable description.
    pub description: String,
    /// Number of items affected.
    pub item_count: usize,
}

impl UndoableAction {
    /// Creates a new undoable action.
    pub fn new(action_type: ActionType, before_state: ActionState) -> Self {
        let item_count = before_state.thread_ids.len();
        let desc = Self::format_description(&action_type, item_count);
        let id = format!("{:?}-{}", action_type, Utc::now().timestamp_millis());

        Self {
            id,
            action_type,
            before_state,
            performed_at: Instant::now(),
            description: desc,
            item_count,
        }
    }

    fn format_description(action_type: &ActionType, count: usize) -> String {
        let noun = if count == 1 {
            "conversation"
        } else {
            "conversations"
        };
        format!("{} {} {}", count, noun, action_type.description())
    }

    /// Returns whether this action is within the undo window.
    pub fn is_within_window(&self, window: Duration) -> bool {
        self.performed_at.elapsed() < window
    }

    /// Returns whether this action can still be undone.
    pub fn can_undo(&self) -> bool {
        self.action_type.is_undoable() && self.is_within_window(DEFAULT_UNDO_WINDOW)
    }

    /// Returns time remaining in the undo window.
    pub fn time_remaining(&self) -> Duration {
        let elapsed = self.performed_at.elapsed();
        if elapsed >= DEFAULT_UNDO_WINDOW {
            Duration::ZERO
        } else {
            DEFAULT_UNDO_WINDOW - elapsed
        }
    }
}

/// Result of executing or undoing an action.
#[derive(Debug)]
pub struct ActionResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Action that was executed or undone.
    pub action: UndoableAction,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ActionResult {
    /// Creates a successful result.
    pub fn success(action: UndoableAction) -> Self {
        Self {
            success: true,
            action,
            error: None,
        }
    }

    /// Creates a failed result.
    pub fn failure(action: UndoableAction, error: impl Into<String>) -> Self {
        Self {
            success: false,
            action,
            error: Some(error.into()),
        }
    }
}

/// Service for managing undo/redo operations.
pub struct UndoService {
    /// Stack of actions that can be undone.
    undo_stack: VecDeque<UndoableAction>,
    /// Stack of actions that can be redone.
    redo_stack: VecDeque<UndoableAction>,
    /// Undo window duration.
    undo_window: Duration,
    /// Whether undo is enabled.
    enabled: bool,
}

impl UndoService {
    /// Creates a new undo service.
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            undo_window: DEFAULT_UNDO_WINDOW,
            enabled: true,
        }
    }

    /// Sets the undo window duration.
    pub fn set_undo_window(&mut self, window: Duration) {
        self.undo_window = window;
    }

    /// Enables or disables undo.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Records an action for potential undo.
    pub fn record(&mut self, action: UndoableAction) {
        if !self.enabled || !action.action_type.is_undoable() {
            return;
        }

        // Clear redo stack when new action is recorded
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push_back(action);

        // Trim to max size
        while self.undo_stack.len() > MAX_HISTORY_SIZE {
            self.undo_stack.pop_front();
        }

        // Clean up expired actions
        self.cleanup_expired();
    }

    /// Records an action with the given type and state.
    pub fn record_action(&mut self, action_type: ActionType, before_state: ActionState) {
        self.record(UndoableAction::new(action_type, before_state));
    }

    /// Returns the most recent undoable action.
    pub fn peek_undo(&self) -> Option<&UndoableAction> {
        self.undo_stack
            .back()
            .filter(|a| a.is_within_window(self.undo_window))
    }

    /// Returns the most recent redoable action.
    pub fn peek_redo(&self) -> Option<&UndoableAction> {
        self.redo_stack.back()
    }

    /// Pops the most recent action for undoing.
    /// Returns None if no action is available or undo window expired.
    pub fn pop_undo(&mut self) -> Option<UndoableAction> {
        self.cleanup_expired();
        self.undo_stack.pop_back().filter(|a| a.can_undo())
    }

    /// Pops the most recent action for redoing.
    pub fn pop_redo(&mut self) -> Option<UndoableAction> {
        self.redo_stack.pop_back()
    }

    /// Pushes an action back onto the redo stack after undoing.
    pub fn push_to_redo(&mut self, action: UndoableAction) {
        self.redo_stack.push_back(action);
        while self.redo_stack.len() > MAX_HISTORY_SIZE {
            self.redo_stack.pop_front();
        }
    }

    /// Returns whether undo is available.
    pub fn can_undo(&self) -> bool {
        self.peek_undo().is_some()
    }

    /// Returns whether redo is available.
    pub fn can_redo(&self) -> bool {
        self.peek_redo().is_some()
    }

    /// Returns the description of the next undo action.
    pub fn undo_description(&self) -> Option<String> {
        self.peek_undo().map(|a| format!("Undo: {}", a.description))
    }

    /// Returns the description of the next redo action.
    pub fn redo_description(&self) -> Option<String> {
        self.peek_redo().map(|a| format!("Redo: {}", a.description))
    }

    /// Returns remaining time for the current undo action.
    pub fn undo_time_remaining(&self) -> Option<Duration> {
        self.peek_undo().map(|a| a.time_remaining())
    }

    /// Returns recent actions (for history view).
    pub fn recent_actions(&self, limit: usize) -> Vec<&UndoableAction> {
        self.undo_stack.iter().rev().take(limit).collect()
    }

    /// Clears all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Removes expired actions from the undo stack.
    fn cleanup_expired(&mut self) {
        let window = self.undo_window;
        self.undo_stack.retain(|a| a.is_within_window(window));
    }

    /// Returns the number of undoable actions.
    pub fn undo_count(&self) -> usize {
        self.cleanup_expired_count()
    }

    /// Returns the number of redoable actions.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    fn cleanup_expired_count(&self) -> usize {
        let window = self.undo_window;
        self.undo_stack
            .iter()
            .filter(|a| a.is_within_window(window))
            .count()
    }
}

impl Default for UndoService {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating undoable actions with batch operations.
pub struct ActionBuilder {
    action_type: ActionType,
    thread_ids: Vec<ThreadId>,
    original_folder: Option<LabelId>,
    target_folder: Option<LabelId>,
    labels: Vec<LabelId>,
    read_states: Vec<(ThreadId, bool)>,
    starred_states: Vec<(ThreadId, bool)>,
}

impl ActionBuilder {
    /// Creates a new action builder.
    pub fn new(action_type: ActionType) -> Self {
        Self {
            action_type,
            thread_ids: Vec::new(),
            original_folder: None,
            target_folder: None,
            labels: Vec::new(),
            read_states: Vec::new(),
            starred_states: Vec::new(),
        }
    }

    /// Adds thread IDs.
    pub fn threads(mut self, ids: impl IntoIterator<Item = ThreadId>) -> Self {
        self.thread_ids.extend(ids);
        self
    }

    /// Sets the original folder.
    pub fn from_folder(mut self, folder: LabelId) -> Self {
        self.original_folder = Some(folder);
        self
    }

    /// Sets the target folder.
    pub fn to_folder(mut self, folder: LabelId) -> Self {
        self.target_folder = Some(folder);
        self
    }

    /// Adds labels.
    pub fn labels(mut self, labels: impl IntoIterator<Item = LabelId>) -> Self {
        self.labels.extend(labels);
        self
    }

    /// Records original read states.
    pub fn read_states(mut self, states: impl IntoIterator<Item = (ThreadId, bool)>) -> Self {
        self.read_states.extend(states);
        self
    }

    /// Records original starred states.
    pub fn starred_states(mut self, states: impl IntoIterator<Item = (ThreadId, bool)>) -> Self {
        self.starred_states.extend(states);
        self
    }

    /// Builds the undoable action.
    pub fn build(self) -> UndoableAction {
        let state = ActionState {
            thread_ids: self.thread_ids,
            original_folder: self.original_folder,
            target_folder: self.target_folder,
            original_labels: Vec::new(),
            affected_labels: self.labels,
            read_states: self.read_states,
            starred_states: self.starred_states,
            snooze_until: None,
        };
        UndoableAction::new(self.action_type, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_thread_id(s: &str) -> ThreadId {
        ThreadId::from(s.to_string())
    }

    fn make_label_id(s: &str) -> LabelId {
        LabelId::from(s.to_string())
    }

    #[test]
    fn action_type_properties() {
        assert!(ActionType::Archive.is_undoable());
        assert!(ActionType::Delete.is_undoable());
        assert!(!ActionType::PermanentDelete.is_undoable());
        assert!(!ActionType::Send.is_undoable());
    }

    #[test]
    fn action_description() {
        let state = ActionState::archive(
            vec![make_thread_id("t1"), make_thread_id("t2")],
            make_label_id("inbox"),
        );
        let action = UndoableAction::new(ActionType::Archive, state);

        assert!(action.description.contains("2"));
        assert!(action.description.contains("conversations"));
        assert!(action.description.contains("archived"));
    }

    #[test]
    fn undo_service_basic() {
        let mut service = UndoService::new();

        let state = ActionState::archive(vec![make_thread_id("t1")], make_label_id("inbox"));
        service.record(UndoableAction::new(ActionType::Archive, state));

        assert!(service.can_undo());
        assert!(!service.can_redo());

        let action = service.pop_undo().unwrap();
        assert_eq!(action.action_type, ActionType::Archive);

        service.push_to_redo(action);
        assert!(service.can_redo());
    }

    #[test]
    fn undo_clears_redo_on_new_action() {
        let mut service = UndoService::new();

        // Record first action
        let state1 = ActionState::archive(vec![make_thread_id("t1")], make_label_id("inbox"));
        service.record(UndoableAction::new(ActionType::Archive, state1));

        // Undo and push to redo
        let action = service.pop_undo().unwrap();
        service.push_to_redo(action);
        assert!(service.can_redo());

        // Record new action
        let state2 = ActionState::delete(vec![make_thread_id("t2")], make_label_id("inbox"));
        service.record(UndoableAction::new(ActionType::Delete, state2));

        // Redo should be cleared
        assert!(!service.can_redo());
    }

    #[test]
    fn action_builder() {
        let action = ActionBuilder::new(ActionType::Move)
            .threads([make_thread_id("t1"), make_thread_id("t2")])
            .from_folder(make_label_id("inbox"))
            .to_folder(make_label_id("archive"))
            .build();

        assert_eq!(action.action_type, ActionType::Move);
        assert_eq!(action.before_state.thread_ids.len(), 2);
        assert!(action.before_state.original_folder.is_some());
        assert!(action.before_state.target_folder.is_some());
    }

    #[test]
    fn undo_window_expiry() {
        let mut service = UndoService::new();
        service.set_undo_window(Duration::from_millis(1));

        let state = ActionState::archive(vec![make_thread_id("t1")], make_label_id("inbox"));
        service.record(UndoableAction::new(ActionType::Archive, state));

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(10));

        assert!(!service.can_undo());
    }

    #[test]
    fn non_undoable_actions_not_recorded() {
        let mut service = UndoService::new();

        let state = ActionState {
            thread_ids: vec![make_thread_id("t1")],
            ..Default::default()
        };
        service.record(UndoableAction::new(ActionType::PermanentDelete, state));

        assert!(!service.can_undo());
    }
}
