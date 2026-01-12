//! Notification service for managing application notifications.
//!
//! Provides a service layer for:
//! - System notifications (OS-level)
//! - In-app toast notifications
//! - Email arrival notifications
//! - Background task notifications

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use thiserror::Error;

/// Errors that can occur during notification operations.
#[derive(Debug, Error)]
pub enum NotificationError {
    /// Failed to send system notification.
    #[error("system notification error: {0}")]
    SystemError(String),

    /// Notification not found.
    #[error("notification not found: {0}")]
    NotFound(String),

    /// Rate limit exceeded.
    #[error("notification rate limit exceeded")]
    RateLimited,
}

/// Result type for notification operations.
pub type NotificationResult<T> = Result<T, NotificationError>;

/// Priority level for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum NotificationPriority {
    /// Low priority, can be batched or delayed.
    Low,
    /// Normal priority.
    #[default]
    Normal,
    /// High priority, show immediately.
    High,
    /// Critical, interrupt user if necessary.
    Critical,
}

/// Type of notification for categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationCategory {
    /// New email received.
    NewEmail,
    /// Email sent successfully.
    EmailSent,
    /// Sync completed.
    SyncComplete,
    /// Sync error.
    SyncError,
    /// Reminder/snooze wake up.
    Reminder,
    /// Screener queue item.
    Screener,
    /// AI task completed.
    AiComplete,
    /// General information.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// A notification request.
#[derive(Debug, Clone)]
pub struct NotificationRequest {
    /// Unique identifier.
    pub id: String,
    /// Notification category.
    pub category: NotificationCategory,
    /// Title text.
    pub title: String,
    /// Body text.
    pub body: Option<String>,
    /// Priority level.
    pub priority: NotificationPriority,
    /// Whether to show as system notification.
    pub system_notification: bool,
    /// Whether to show as in-app toast.
    pub in_app_toast: bool,
    /// Whether to play a sound.
    pub sound: bool,
    /// Auto-dismiss after this duration.
    pub auto_dismiss: Option<Duration>,
    /// Action URL or identifier.
    pub action: Option<String>,
}

impl NotificationRequest {
    /// Creates a new notification request.
    pub fn new(category: NotificationCategory, title: impl Into<String>) -> Self {
        Self {
            id: format!("notif-{}", uuid::Uuid::new_v4()),
            category,
            title: title.into(),
            body: None,
            priority: NotificationPriority::Normal,
            system_notification: false,
            in_app_toast: true,
            sound: false,
            auto_dismiss: Some(Duration::from_secs(5)),
            action: None,
        }
    }

    /// Sets the body text.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the priority.
    pub fn priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Enables system notification.
    pub fn system(mut self) -> Self {
        self.system_notification = true;
        self
    }

    /// Disables in-app toast.
    pub fn no_toast(mut self) -> Self {
        self.in_app_toast = false;
        self
    }

    /// Enables sound.
    pub fn with_sound(mut self) -> Self {
        self.sound = true;
        self
    }

    /// Sets auto-dismiss duration.
    pub fn dismiss_after(mut self, duration: Duration) -> Self {
        self.auto_dismiss = Some(duration);
        self
    }

    /// Disables auto-dismiss.
    pub fn persistent(mut self) -> Self {
        self.auto_dismiss = None;
        self
    }

    /// Sets an action.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Creates a new email notification.
    pub fn new_email(sender: &str, subject: &str) -> Self {
        Self::new(
            NotificationCategory::NewEmail,
            format!("New email from {}", sender),
        )
        .body(subject)
        .system()
        .with_sound()
    }

    /// Creates an email sent notification.
    pub fn email_sent() -> Self {
        Self::new(NotificationCategory::EmailSent, "Email sent")
            .dismiss_after(Duration::from_secs(3))
    }

    /// Creates a sync complete notification.
    pub fn sync_complete(count: u32) -> Self {
        let title = if count == 0 {
            "Sync complete".to_string()
        } else if count == 1 {
            "1 new email".to_string()
        } else {
            format!("{} new emails", count)
        };
        Self::new(NotificationCategory::SyncComplete, title)
    }

    /// Creates a sync error notification.
    pub fn sync_error(message: &str) -> Self {
        Self::new(NotificationCategory::SyncError, "Sync failed")
            .body(message)
            .priority(NotificationPriority::High)
            .persistent()
    }

    /// Creates a reminder notification.
    pub fn reminder(subject: &str) -> Self {
        Self::new(NotificationCategory::Reminder, "Reminder")
            .body(subject)
            .system()
            .with_sound()
            .priority(NotificationPriority::High)
    }

    /// Creates an info notification.
    pub fn info(message: &str) -> Self {
        Self::new(NotificationCategory::Info, message)
    }

    /// Creates a warning notification.
    pub fn warning(message: &str) -> Self {
        Self::new(NotificationCategory::Warning, message)
            .priority(NotificationPriority::High)
            .dismiss_after(Duration::from_secs(10))
    }

    /// Creates an error notification.
    pub fn error(message: &str) -> Self {
        Self::new(NotificationCategory::Error, message)
            .priority(NotificationPriority::Critical)
            .persistent()
    }
}

/// A sent notification with tracking info.
#[derive(Debug, Clone)]
pub struct SentNotification {
    /// The original request.
    pub request: NotificationRequest,
    /// When it was sent.
    pub sent_at: Instant,
    /// Whether it has been dismissed.
    pub dismissed: bool,
    /// Whether the action was taken.
    pub action_taken: bool,
}

impl SentNotification {
    /// Creates a new sent notification.
    pub fn new(request: NotificationRequest) -> Self {
        Self {
            request,
            sent_at: Instant::now(),
            dismissed: false,
            action_taken: false,
        }
    }

    /// Returns whether this notification should auto-dismiss.
    pub fn should_auto_dismiss(&self) -> bool {
        if self.dismissed {
            return false;
        }
        if let Some(duration) = self.request.auto_dismiss {
            self.sent_at.elapsed() >= duration
        } else {
            false
        }
    }
}

/// Settings for notification behavior.
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    /// Enable system notifications.
    pub system_notifications_enabled: bool,
    /// Enable in-app toasts.
    pub toasts_enabled: bool,
    /// Enable sounds.
    pub sounds_enabled: bool,
    /// Do not disturb mode.
    pub do_not_disturb: bool,
    /// Maximum notifications to show at once.
    pub max_visible: usize,
    /// Minimum time between notifications of the same type.
    pub rate_limit: Duration,
    /// Categories to mute.
    pub muted_categories: Vec<NotificationCategory>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            system_notifications_enabled: true,
            toasts_enabled: true,
            sounds_enabled: true,
            do_not_disturb: false,
            max_visible: 5,
            rate_limit: Duration::from_secs(1),
            muted_categories: Vec::new(),
        }
    }
}

impl NotificationSettings {
    /// Creates settings with do not disturb enabled.
    pub fn do_not_disturb() -> Self {
        Self {
            do_not_disturb: true,
            ..Default::default()
        }
    }

    /// Checks if a category is muted.
    pub fn is_muted(&self, category: NotificationCategory) -> bool {
        self.muted_categories.contains(&category)
    }
}

/// Service for managing notifications.
pub struct NotificationService {
    settings: NotificationSettings,
    sent: VecDeque<SentNotification>,
    last_by_category: std::collections::HashMap<NotificationCategory, Instant>,
}

impl NotificationService {
    /// Creates a new notification service.
    pub fn new(settings: NotificationSettings) -> Self {
        Self {
            settings,
            sent: VecDeque::new(),
            last_by_category: std::collections::HashMap::new(),
        }
    }

    /// Creates a service with default settings.
    pub fn with_defaults() -> Self {
        Self::new(NotificationSettings::default())
    }

    /// Updates settings.
    pub fn set_settings(&mut self, settings: NotificationSettings) {
        self.settings = settings;
    }

    /// Returns current settings.
    pub fn settings(&self) -> &NotificationSettings {
        &self.settings
    }

    /// Sends a notification.
    pub fn notify(&mut self, request: NotificationRequest) -> NotificationResult<()> {
        // Check do not disturb
        if self.settings.do_not_disturb && request.priority < NotificationPriority::Critical {
            return Ok(());
        }

        // Check if category is muted
        if self.settings.is_muted(request.category) {
            return Ok(());
        }

        // Check rate limiting
        if let Some(last) = self.last_by_category.get(&request.category) {
            if last.elapsed() < self.settings.rate_limit {
                return Err(NotificationError::RateLimited);
            }
        }

        // Send system notification if enabled
        if request.system_notification && self.settings.system_notifications_enabled {
            self.send_system_notification(&request)?;
        }

        // Add to in-app queue if enabled
        if request.in_app_toast && self.settings.toasts_enabled {
            self.sent.push_back(SentNotification::new(request.clone()));

            // Trim to max visible
            while self.sent.len() > self.settings.max_visible {
                self.sent.pop_front();
            }
        }

        // Update rate limit tracking
        self.last_by_category
            .insert(request.category, Instant::now());

        Ok(())
    }

    /// Dismisses a notification by ID.
    pub fn dismiss(&mut self, id: &str) {
        if let Some(notif) = self.sent.iter_mut().find(|n| n.request.id == id) {
            notif.dismissed = true;
        }
    }

    /// Dismisses all notifications.
    pub fn dismiss_all(&mut self) {
        for notif in self.sent.iter_mut() {
            notif.dismissed = true;
        }
    }

    /// Marks an action as taken.
    pub fn action_taken(&mut self, id: &str) {
        if let Some(notif) = self.sent.iter_mut().find(|n| n.request.id == id) {
            notif.action_taken = true;
            notif.dismissed = true;
        }
    }

    /// Cleans up dismissed and expired notifications.
    pub fn cleanup(&mut self) {
        self.sent
            .retain(|n| !n.dismissed && !n.should_auto_dismiss());
    }

    /// Returns all active notifications.
    pub fn active_notifications(&self) -> Vec<&SentNotification> {
        self.sent
            .iter()
            .filter(|n| !n.dismissed && !n.should_auto_dismiss())
            .collect()
    }

    /// Returns the count of active notifications.
    pub fn active_count(&self) -> usize {
        self.active_notifications().len()
    }

    /// Sends a system notification (platform-specific).
    fn send_system_notification(&self, request: &NotificationRequest) -> NotificationResult<()> {
        // Platform-specific implementation would go here
        // For now, we just log
        tracing::debug!(
            title = %request.title,
            body = ?request.body,
            "System notification"
        );
        Ok(())
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_request_builders() {
        let notif = NotificationRequest::new(NotificationCategory::Info, "Test")
            .body("Body text")
            .priority(NotificationPriority::High)
            .system()
            .with_sound();

        assert_eq!(notif.title, "Test");
        assert_eq!(notif.body, Some("Body text".to_string()));
        assert_eq!(notif.priority, NotificationPriority::High);
        assert!(notif.system_notification);
        assert!(notif.sound);
    }

    #[test]
    fn new_email_notification() {
        let notif = NotificationRequest::new_email("sender@example.com", "Hello World");
        assert_eq!(notif.category, NotificationCategory::NewEmail);
        assert!(notif.title.contains("sender@example.com"));
        assert!(notif.system_notification);
        assert!(notif.sound);
    }

    #[test]
    fn service_notify_and_dismiss() {
        let mut settings = NotificationSettings::default();
        settings.rate_limit = Duration::from_secs(0); // Disable rate limiting for test
        let mut service = NotificationService::new(settings);

        service.notify(NotificationRequest::info("Test 1")).unwrap();
        service.notify(NotificationRequest::info("Test 2")).unwrap();

        assert_eq!(service.active_count(), 2);

        let id = service.sent[0].request.id.clone();
        service.dismiss(&id);

        assert_eq!(service.active_count(), 1);
    }

    #[test]
    fn do_not_disturb() {
        let mut service = NotificationService::new(NotificationSettings::do_not_disturb());

        // Normal notifications should be suppressed
        service.notify(NotificationRequest::info("Test")).unwrap();
        assert_eq!(service.active_count(), 0);

        // Critical notifications should still go through
        service
            .notify(NotificationRequest::error("Critical").priority(NotificationPriority::Critical))
            .unwrap();
        assert_eq!(service.active_count(), 1);
    }

    #[test]
    fn muted_categories() {
        let mut settings = NotificationSettings::default();
        settings
            .muted_categories
            .push(NotificationCategory::SyncComplete);

        let mut service = NotificationService::new(settings);

        service
            .notify(NotificationRequest::sync_complete(5))
            .unwrap();
        assert_eq!(service.active_count(), 0);

        service.notify(NotificationRequest::info("Info")).unwrap();
        assert_eq!(service.active_count(), 1);
    }

    #[test]
    fn auto_dismiss() {
        let notif = NotificationRequest::info("Test").dismiss_after(Duration::from_millis(1));
        let sent = SentNotification::new(notif);

        std::thread::sleep(Duration::from_millis(10));

        assert!(sent.should_auto_dismiss());
    }

    #[test]
    fn persistent_notification() {
        let notif = NotificationRequest::error("Error").persistent();
        let sent = SentNotification::new(notif);

        std::thread::sleep(Duration::from_millis(10));

        assert!(!sent.should_auto_dismiss());
    }
}
