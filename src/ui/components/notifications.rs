//! Notification system for in-app toasts and status updates.
//!
//! Provides:
//! - Toast notifications with auto-dismiss
//! - Notification queue management
//! - Different notification types (success, error, info, warning)
//! - Action buttons on notifications

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, px, rgba, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

/// Type of notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    /// Success notification (green).
    Success,
    /// Error notification (red).
    Error,
    /// Warning notification (yellow).
    Warning,
    /// Informational notification (blue).
    Info,
    /// Loading/progress notification.
    Loading,
}

impl NotificationType {
    /// Returns the background color for this type.
    pub fn bg_color(&self) -> u32 {
        match self {
            NotificationType::Success => 0x22C55E20,
            NotificationType::Error => 0xEF444420,
            NotificationType::Warning => 0xF59E0B20,
            NotificationType::Info => 0x3B82F620,
            NotificationType::Loading => 0x71717A20,
        }
    }

    /// Returns the accent color for this type.
    pub fn accent_color(&self) -> u32 {
        match self {
            NotificationType::Success => 0x22C55EFF,
            NotificationType::Error => 0xEF4444FF,
            NotificationType::Warning => 0xF59E0BFF,
            NotificationType::Info => 0x3B82F6FF,
            NotificationType::Loading => 0x71717AFF,
        }
    }

    /// Returns the icon for this type.
    pub fn icon(&self) -> &'static str {
        match self {
            NotificationType::Success => "check",
            NotificationType::Error => "x",
            NotificationType::Warning => "alert",
            NotificationType::Info => "info",
            NotificationType::Loading => "loading",
        }
    }
}

/// An action button on a notification.
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Action ID.
    pub id: String,
    /// Button label.
    pub label: SharedString,
    /// Whether this is the primary action.
    pub primary: bool,
}

impl NotificationAction {
    /// Creates a new action.
    pub fn new(id: impl Into<String>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            primary: false,
        }
    }

    /// Makes this the primary action.
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }
}

/// A notification to display.
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique ID.
    pub id: String,
    /// Notification type.
    pub notification_type: NotificationType,
    /// Title text.
    pub title: SharedString,
    /// Optional body text.
    pub body: Option<SharedString>,
    /// Actions.
    pub actions: Vec<NotificationAction>,
    /// When the notification was created.
    pub created_at: Instant,
    /// How long to show (None = manual dismiss only).
    pub duration: Option<Duration>,
    /// Whether the notification can be dismissed.
    pub dismissable: bool,
}

impl Notification {
    /// Creates a new notification.
    pub fn new(
        id: impl Into<String>,
        notification_type: NotificationType,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            notification_type,
            title: title.into(),
            body: None,
            actions: Vec::new(),
            created_at: Instant::now(),
            duration: Some(Duration::from_secs(5)),
            dismissable: true,
        }
    }

    /// Creates a success notification.
    pub fn success(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self::new(id, NotificationType::Success, title)
    }

    /// Creates an error notification.
    pub fn error(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self::new(id, NotificationType::Error, title).with_duration(None) // Errors don't auto-dismiss
    }

    /// Creates an info notification.
    pub fn info(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self::new(id, NotificationType::Info, title)
    }

    /// Creates a warning notification.
    pub fn warning(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self::new(id, NotificationType::Warning, title)
    }

    /// Creates a loading notification.
    pub fn loading(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self::new(id, NotificationType::Loading, title)
            .with_duration(None) // Loading doesn't auto-dismiss
            .not_dismissable()
    }

    /// Sets the body text.
    pub fn with_body(mut self, body: impl Into<SharedString>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the duration.
    pub fn with_duration(mut self, duration: Option<Duration>) -> Self {
        self.duration = duration;
        self
    }

    /// Adds an action.
    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Makes the notification not dismissable.
    pub fn not_dismissable(mut self) -> Self {
        self.dismissable = false;
        self
    }

    /// Returns true if the notification has expired.
    pub fn is_expired(&self) -> bool {
        match self.duration {
            Some(d) => self.created_at.elapsed() > d,
            None => false,
        }
    }
}

/// Manages and displays notifications.
pub struct NotificationManager {
    /// Active notifications.
    notifications: VecDeque<Notification>,
    /// Maximum notifications to show at once.
    max_visible: usize,
}

impl NotificationManager {
    /// Creates a new notification manager.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            notifications: VecDeque::new(),
            max_visible: 5,
        }
    }

    /// Adds a notification.
    pub fn notify(&mut self, notification: Notification) {
        // Remove any existing notification with the same ID
        self.notifications.retain(|n| n.id != notification.id);
        self.notifications.push_back(notification);

        // Trim to max
        while self.notifications.len() > self.max_visible {
            self.notifications.pop_front();
        }
    }

    /// Shows a success notification.
    pub fn success(&mut self, title: impl Into<SharedString>) {
        let id = format!("success-{}", Instant::now().elapsed().as_nanos());
        self.notify(Notification::success(id, title));
    }

    /// Shows an error notification.
    pub fn error(&mut self, title: impl Into<SharedString>) {
        let id = format!("error-{}", Instant::now().elapsed().as_nanos());
        self.notify(Notification::error(id, title));
    }

    /// Shows an info notification.
    pub fn info(&mut self, title: impl Into<SharedString>) {
        let id = format!("info-{}", Instant::now().elapsed().as_nanos());
        self.notify(Notification::info(id, title));
    }

    /// Dismisses a notification by ID.
    pub fn dismiss(&mut self, id: &str) {
        self.notifications.retain(|n| n.id != id);
    }

    /// Dismisses all notifications.
    pub fn dismiss_all(&mut self) {
        self.notifications.clear();
    }

    /// Removes expired notifications.
    pub fn cleanup_expired(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    /// Updates a notification by ID.
    pub fn update(&mut self, id: &str, f: impl FnOnce(&mut Notification)) {
        if let Some(notification) = self.notifications.iter_mut().find(|n| n.id == id) {
            f(notification);
        }
    }

    /// Returns the number of notifications.
    pub fn count(&self) -> usize {
        self.notifications.len()
    }

    fn render_notification(&self, notification: &Notification) -> impl IntoElement {
        let bg = notification.notification_type.bg_color();
        let accent = notification.notification_type.accent_color();
        let icon = notification.notification_type.icon();
        let title = notification.title.clone();
        let body = notification.body.clone();
        let actions = notification.actions.clone();
        let dismissable = notification.dismissable;
        let notif_id = notification.id.clone();

        div()
            .id(SharedString::from(format!("notif-{}", notif_id)))
            .w(px(360.0))
            .bg(rgba(0x27272AFF))
            .rounded(px(8.0))
            .shadow_lg()
            .border_1()
            .border_color(rgba(bg))
            .overflow_hidden()
            .child(
                div()
                    .p(px(12.0))
                    .flex()
                    .gap(px(12.0))
                    // Icon
                    .child(
                        div()
                            .size(px(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(rgba(bg))
                            .child(div().text_xs().text_color(rgba(accent)).child(icon)),
                    )
                    // Content
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgba(0xF4F4F5FF))
                                    .child(title),
                            )
                            .when_some(body, |d, b| {
                                d.child(div().text_xs().text_color(rgba(0xA1A1AAFF)).child(b))
                            })
                            .when(!actions.is_empty(), |d| {
                                d.child(div().mt(px(8.0)).flex().gap(px(8.0)).children(
                                    actions.iter().map(|action| {
                                        let action_id = action.id.clone();
                                        let label = action.label.clone();
                                        let is_primary = action.primary;
                                        div()
                                            .id(SharedString::from(format!("action-{}", action_id)))
                                            .px(px(10.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .when(is_primary, |d| {
                                                d.bg(rgba(accent))
                                                    .hover(|d| d.bg(rgba(accent & 0xFFFFFFDD)))
                                            })
                                            .when(!is_primary, |d| {
                                                d.bg(rgba(0x3F3F46FF))
                                                    .hover(|d| d.bg(rgba(0x52525BFF)))
                                            })
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .when(is_primary, |d| {
                                                        d.text_color(rgba(0xFFFFFFFF))
                                                    })
                                                    .when(!is_primary, |d| {
                                                        d.text_color(rgba(0xE4E4E7FF))
                                                    })
                                                    .child(label),
                                            )
                                    }),
                                ))
                            }),
                    )
                    // Dismiss button
                    .when(dismissable, |d| {
                        d.child(
                            div()
                                .id(SharedString::from(format!("dismiss-{}", notif_id)))
                                .size(px(20.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                .child(div().text_xs().text_color(rgba(0x71717AFF)).child("x")),
                        )
                    }),
            )
    }
}

impl Render for NotificationManager {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.notifications.is_empty() {
            return div().id("notifications-empty");
        }

        div()
            .id("notifications-container")
            .absolute()
            .bottom(px(20.0))
            .right(px(20.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(
                self.notifications
                    .iter()
                    .rev()
                    .map(|n| self.render_notification(n)),
            )
    }
}

/// Status bar notification display.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    /// Message text.
    pub text: SharedString,
    /// Message type.
    pub message_type: NotificationType,
    /// When it was shown.
    pub shown_at: Instant,
}

impl StatusMessage {
    /// Creates a new status message.
    pub fn new(text: impl Into<SharedString>, message_type: NotificationType) -> Self {
        Self {
            text: text.into(),
            message_type,
            shown_at: Instant::now(),
        }
    }
}

/// Status bar component showing sync status, AI status, etc.
pub struct StatusBar {
    /// Current status message.
    message: Option<StatusMessage>,
    /// Whether syncing.
    syncing: bool,
    /// Whether AI is processing.
    ai_processing: bool,
    /// Offline mode.
    offline: bool,
    /// Unread count.
    unread_count: u32,
}

impl StatusBar {
    /// Creates a new status bar.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            message: None,
            syncing: false,
            ai_processing: false,
            offline: false,
            unread_count: 0,
        }
    }

    /// Sets the status message.
    pub fn set_message(&mut self, message: StatusMessage) {
        self.message = Some(message);
    }

    /// Clears the status message.
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Sets syncing state.
    pub fn set_syncing(&mut self, syncing: bool) {
        self.syncing = syncing;
    }

    /// Sets AI processing state.
    pub fn set_ai_processing(&mut self, processing: bool) {
        self.ai_processing = processing;
    }

    /// Sets offline mode.
    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    /// Sets unread count.
    pub fn set_unread_count(&mut self, count: u32) {
        self.unread_count = count;
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("status-bar")
            .h(px(24.0))
            .w_full()
            .px(px(12.0))
            .bg(rgba(0x18181BFF))
            .border_t_1()
            .border_color(rgba(0x27272AFF))
            .flex()
            .items_center()
            .justify_between()
            // Left side: status indicators
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    // Offline indicator
                    .when(self.offline, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(div().size(px(6.0)).rounded_full().bg(rgba(0xF59E0BFF)))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgba(0xF59E0BFF))
                                        .child("Offline"),
                                ),
                        )
                    })
                    // Sync indicator
                    .when(self.syncing, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(div().text_xs().text_color(rgba(0x3B82F6FF)).child("sync"))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgba(0x71717AFF))
                                        .child("Syncing..."),
                                ),
                        )
                    })
                    // AI indicator
                    .when(self.ai_processing, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(div().text_xs().text_color(rgba(0xA855F7FF)).child("ai"))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgba(0x71717AFF))
                                        .child("Processing..."),
                                ),
                        )
                    })
                    // Message
                    .when_some(self.message.clone(), |d, msg| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(rgba(msg.message_type.accent_color()))
                                .child(msg.text),
                        )
                    }),
            )
            // Right side: counts and info
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .when(self.unread_count > 0, |d| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(rgba(0x71717AFF))
                                .child(format!("{} unread", self.unread_count)),
                        )
                    })
                    .child(div().text_xs().text_color(rgba(0x52525BFF)).child("margin")),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_type_colors() {
        assert_ne!(
            NotificationType::Success.bg_color(),
            NotificationType::Error.bg_color()
        );
        assert_ne!(
            NotificationType::Success.accent_color(),
            NotificationType::Error.accent_color()
        );
    }

    #[test]
    fn notification_builder() {
        let notif = Notification::success("test-1", "Test notification")
            .with_body("This is a test")
            .with_action(NotificationAction::new("undo", "Undo").primary());

        assert_eq!(notif.id, "test-1");
        assert_eq!(notif.notification_type, NotificationType::Success);
        assert!(notif.body.is_some());
        assert_eq!(notif.actions.len(), 1);
        assert!(notif.actions[0].primary);
    }

    #[test]
    fn notification_expiry() {
        let notif =
            Notification::info("test", "Test").with_duration(Some(Duration::from_millis(1)));

        // Wait a bit
        std::thread::sleep(Duration::from_millis(10));

        assert!(notif.is_expired());
    }

    #[test]
    fn notification_no_expiry() {
        let notif = Notification::error("test", "Error").with_duration(None);

        assert!(!notif.is_expired());
    }
}
