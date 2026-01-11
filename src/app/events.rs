//! Event bus for cross-component communication.
//!
//! Provides a publish-subscribe system for domain events that enables
//! loose coupling between components.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::{AccountId, EmailId, LabelId, ThreadId};

/// Domain events for cross-component communication.
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Account events
    /// An account was added.
    AccountAdded(AccountId),
    /// An account was removed.
    AccountRemoved(AccountId),
    /// Sync started for an account.
    AccountSyncStarted(AccountId),
    /// Sync completed for an account.
    AccountSyncCompleted {
        account_id: AccountId,
        new_emails: u32,
        updated: u32,
    },
    /// Sync failed for an account.
    AccountSyncFailed {
        account_id: AccountId,
        error: String,
    },

    // Email events
    /// New emails received.
    EmailsReceived {
        account_id: AccountId,
        email_ids: Vec<EmailId>,
    },
    /// An email was sent.
    EmailSent {
        account_id: AccountId,
        email_id: EmailId,
    },
    /// Threads were archived.
    ThreadsArchived {
        account_id: AccountId,
        thread_ids: Vec<ThreadId>,
    },
    /// Threads were moved to trash.
    ThreadsTrashed {
        account_id: AccountId,
        thread_ids: Vec<ThreadId>,
    },
    /// Thread star state changed.
    ThreadStarred {
        account_id: AccountId,
        thread_id: ThreadId,
        starred: bool,
    },
    /// Thread read state changed.
    ThreadReadStateChanged {
        account_id: AccountId,
        thread_id: ThreadId,
        is_read: bool,
    },
    /// Label applied to threads.
    LabelApplied {
        account_id: AccountId,
        thread_ids: Vec<ThreadId>,
        label_id: LabelId,
    },
    /// Label removed from threads.
    LabelRemoved {
        account_id: AccountId,
        thread_ids: Vec<ThreadId>,
        label_id: LabelId,
    },

    // AI events
    /// AI task started.
    AiTaskStarted {
        task_id: String,
        task_type: AiTaskType,
    },
    /// AI task completed.
    AiTaskCompleted {
        task_id: String,
        result: AiResult,
    },
    /// AI task failed.
    AiTaskFailed { task_id: String, error: String },

    // UI events
    /// Navigate to a view.
    NavigateTo(ViewNavigation),
    /// Select a thread.
    SelectThread(ThreadId),
    /// Open the composer.
    OpenComposer(ComposerTrigger),
    /// Close the composer.
    CloseComposer,
    /// Show a notification.
    ShowNotification(Notification),
    /// Dismiss a notification.
    DismissNotification(String),
    /// Search query changed.
    SearchQueryChanged(String),
    /// Theme changed.
    ThemeChanged(String),
    /// Settings updated.
    SettingsUpdated,
}

/// AI task types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiTaskType {
    /// Summarize a thread.
    Summary,
    /// Draft a reply.
    DraftReply,
    /// Semantic search.
    SemanticSearch,
    /// Categorize email.
    Categorize,
    /// Analyze sender.
    AnalyzeSender,
}

/// Result of an AI task.
#[derive(Debug, Clone)]
pub enum AiResult {
    /// Summary result.
    Summary {
        thread_id: ThreadId,
        text: String,
        key_points: Vec<String>,
        action_items: Vec<String>,
    },
    /// Draft reply result.
    DraftReply {
        thread_id: ThreadId,
        content: String,
        confidence: f32,
    },
    /// Search results.
    SearchResults {
        query: String,
        results: Vec<SearchResultItem>,
    },
    /// Categorization result.
    Categories { email_id: EmailId, categories: Vec<String> },
    /// Sender analysis result.
    SenderAnalysis {
        sender: String,
        sender_type: String,
        suggested_action: String,
    },
}

/// A search result item.
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    pub email_id: EmailId,
    pub score: f32,
    pub snippet: String,
}

/// View navigation targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewNavigation {
    Inbox,
    Starred,
    Sent,
    Drafts,
    Archive,
    Trash,
    Snoozed,
    Label(LabelId),
    Screener,
    Search(String),
    Settings,
    Stats,
    Thread(ThreadId),
}

/// Composer trigger modes.
#[derive(Debug, Clone)]
pub enum ComposerTrigger {
    /// New email.
    New,
    /// Reply to a message.
    Reply {
        thread_id: ThreadId,
        message_id: String,
    },
    /// Reply all to a message.
    ReplyAll {
        thread_id: ThreadId,
        message_id: String,
    },
    /// Forward a message.
    Forward {
        thread_id: ThreadId,
        message_id: String,
    },
    /// Edit a draft.
    EditDraft { draft_id: String },
}

/// A user notification.
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique ID for this notification.
    pub id: String,
    /// Notification title.
    pub title: String,
    /// Notification body.
    pub body: Option<String>,
    /// Notification severity level.
    pub level: NotificationLevel,
    /// Auto-dismiss after duration (milliseconds).
    pub auto_dismiss_ms: Option<u64>,
}

/// Notification severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl Notification {
    pub fn info(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            level: NotificationLevel::Info,
            auto_dismiss_ms: Some(5000),
        }
    }

    pub fn success(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            level: NotificationLevel::Success,
            auto_dismiss_ms: Some(3000),
        }
    }

    pub fn warning(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            level: NotificationLevel::Warning,
            auto_dismiss_ms: Some(8000),
        }
    }

    pub fn error(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            level: NotificationLevel::Error,
            auto_dismiss_ms: None,
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn persistent(mut self) -> Self {
        self.auto_dismiss_ms = None;
        self
    }
}

/// Subscriber ID for unsubscribing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(u64);

/// Event handler function type.
pub type EventHandler = Box<dyn Fn(&AppEvent) + Send + Sync>;

/// Event bus for publish-subscribe communication.
///
/// Allows components to publish events and subscribe to events they care about.
/// Thread-safe for use across async boundaries.
pub struct EventBus {
    handlers: Arc<Mutex<HashMap<u64, EventHandler>>>,
    next_id: Arc<Mutex<u64>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Subscribe to all events.
    ///
    /// Returns a subscriber ID that can be used to unsubscribe.
    pub fn subscribe<F>(&self, handler: F) -> SubscriberId
    where
        F: Fn(&AppEvent) + Send + Sync + 'static,
    {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert(id, Box::new(handler));

        SubscriberId(id)
    }

    /// Unsubscribe from events.
    pub fn unsubscribe(&self, subscriber_id: SubscriberId) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.remove(&subscriber_id.0);
    }

    /// Publish an event to all subscribers.
    pub fn publish(&self, event: AppEvent) {
        let handlers = self.handlers.lock().unwrap();
        for handler in handlers.values() {
            handler(&event);
        }
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.handlers.lock().unwrap().len()
    }
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("subscriber_count", &self.subscriber_count())
            .finish()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            next_id: Arc::clone(&self.next_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn subscribe_and_publish() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        let _sub = bus.subscribe(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(AppEvent::AccountAdded(AccountId::from("test")));
        bus.publish(AppEvent::SettingsUpdated);

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn unsubscribe() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        let sub_id = bus.subscribe(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(AppEvent::SettingsUpdated);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        bus.unsubscribe(sub_id);

        bus.publish(AppEvent::SettingsUpdated);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn multiple_subscribers() {
        let bus = EventBus::new();
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&counter1);
        let _sub1 = bus.subscribe(move |_event| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let c2 = Arc::clone(&counter2);
        let _sub2 = bus.subscribe(move |_event| {
            c2.fetch_add(10, Ordering::SeqCst);
        });

        bus.publish(AppEvent::SettingsUpdated);

        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn subscriber_count() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);

        let sub1 = bus.subscribe(|_| {});
        assert_eq!(bus.subscriber_count(), 1);

        let sub2 = bus.subscribe(|_| {});
        assert_eq!(bus.subscriber_count(), 2);

        bus.unsubscribe(sub1);
        assert_eq!(bus.subscriber_count(), 1);

        bus.unsubscribe(sub2);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn event_bus_is_clone() {
        let bus1 = EventBus::new();
        let bus2 = bus1.clone();

        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);
        let _sub = bus1.subscribe(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        bus2.publish(AppEvent::SettingsUpdated);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn notification_builders() {
        let info = Notification::info("id1", "Info message");
        assert_eq!(info.level, NotificationLevel::Info);
        assert_eq!(info.auto_dismiss_ms, Some(5000));

        let success = Notification::success("id2", "Success message");
        assert_eq!(success.level, NotificationLevel::Success);

        let warning = Notification::warning("id3", "Warning message").with_body("Details here");
        assert_eq!(warning.level, NotificationLevel::Warning);
        assert!(warning.body.is_some());

        let error = Notification::error("id4", "Error message").persistent();
        assert_eq!(error.level, NotificationLevel::Error);
        assert_eq!(error.auto_dismiss_ms, None);
    }

    #[test]
    fn app_event_variants() {
        let account_event = AppEvent::AccountAdded(AccountId::from("acc-1"));
        assert!(matches!(account_event, AppEvent::AccountAdded(_)));

        let sync_event = AppEvent::AccountSyncCompleted {
            account_id: AccountId::from("acc-1"),
            new_emails: 5,
            updated: 2,
        };
        assert!(matches!(sync_event, AppEvent::AccountSyncCompleted { .. }));

        let nav_event = AppEvent::NavigateTo(ViewNavigation::Inbox);
        assert!(matches!(nav_event, AppEvent::NavigateTo(ViewNavigation::Inbox)));
    }

    #[test]
    fn ai_task_types() {
        let task = AiTaskType::Summary;
        assert_eq!(task, AiTaskType::Summary);

        let result = AiResult::Summary {
            thread_id: ThreadId::from("thread-1"),
            text: "Summary text".to_string(),
            key_points: vec!["Point 1".to_string()],
            action_items: vec!["Action 1".to_string()],
        };
        assert!(matches!(result, AiResult::Summary { .. }));
    }

    #[test]
    fn composer_trigger_variants() {
        let new = ComposerTrigger::New;
        assert!(matches!(new, ComposerTrigger::New));

        let reply = ComposerTrigger::Reply {
            thread_id: ThreadId::from("thread-1"),
            message_id: "msg-1".to_string(),
        };
        assert!(matches!(reply, ComposerTrigger::Reply { .. }));
    }
}
