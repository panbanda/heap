//! Smart Views for intelligent email filtering.
//!
//! Smart Views automatically categorize emails based on AI analysis
//! and user behavior patterns. They provide quick access to:
//! - Needs Reply: emails requiring user response
//! - Waiting For: sent emails awaiting replies
//! - Newsletters: promotional/bulk mail
//! - VIP: important contacts
//! - Follow Up: flagged for later action

use chrono::{DateTime, Duration, Utc};
use gpui::{
    div, prelude::*, px, rgba, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

use crate::domain::ThreadId;

/// Types of smart views available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SmartViewType {
    /// Emails that need a reply from the user.
    NeedsReply,
    /// Emails where user is waiting for a response.
    WaitingFor,
    /// Newsletter/promotional emails.
    Newsletters,
    /// VIP/important contacts.
    Vip,
    /// Flagged for follow-up.
    FollowUp,
    /// Recently read but unactioned.
    RecentlyViewed,
    /// Emails with attachments.
    Attachments,
}

impl SmartViewType {
    /// Returns all smart view types.
    pub fn all() -> &'static [SmartViewType] {
        &[
            SmartViewType::NeedsReply,
            SmartViewType::WaitingFor,
            SmartViewType::Newsletters,
            SmartViewType::Vip,
            SmartViewType::FollowUp,
            SmartViewType::RecentlyViewed,
            SmartViewType::Attachments,
        ]
    }

    /// Returns the display name for this view type.
    pub fn display_name(&self) -> &'static str {
        match self {
            SmartViewType::NeedsReply => "Needs Reply",
            SmartViewType::WaitingFor => "Waiting For",
            SmartViewType::Newsletters => "Newsletters",
            SmartViewType::Vip => "VIP",
            SmartViewType::FollowUp => "Follow Up",
            SmartViewType::RecentlyViewed => "Recently Viewed",
            SmartViewType::Attachments => "Attachments",
        }
    }

    /// Returns the icon name for this view type.
    pub fn icon(&self) -> &'static str {
        match self {
            SmartViewType::NeedsReply => "reply",
            SmartViewType::WaitingFor => "clock",
            SmartViewType::Newsletters => "newspaper",
            SmartViewType::Vip => "star",
            SmartViewType::FollowUp => "flag",
            SmartViewType::RecentlyViewed => "eye",
            SmartViewType::Attachments => "paperclip",
        }
    }

    /// Returns the accent color for this view type.
    pub fn color(&self) -> u32 {
        match self {
            SmartViewType::NeedsReply => 0xEF4444FF,     // Red
            SmartViewType::WaitingFor => 0xF59E0BFF,     // Amber
            SmartViewType::Newsletters => 0x8B5CF6FF,    // Purple
            SmartViewType::Vip => 0xEAB308FF,            // Yellow
            SmartViewType::FollowUp => 0x3B82F6FF,       // Blue
            SmartViewType::RecentlyViewed => 0x71717AFF, // Gray
            SmartViewType::Attachments => 0x22C55EFF,    // Green
        }
    }

    /// Returns the keyboard shortcut for this view.
    pub fn shortcut(&self) -> Option<&'static str> {
        match self {
            SmartViewType::NeedsReply => Some("g r"),
            SmartViewType::WaitingFor => Some("g w"),
            SmartViewType::Newsletters => Some("g n"),
            SmartViewType::Vip => Some("g v"),
            SmartViewType::FollowUp => Some("g f"),
            _ => None,
        }
    }
}

/// Criteria for filtering into a smart view.
#[derive(Debug, Clone)]
pub struct SmartViewCriteria {
    /// Type of smart view.
    pub view_type: SmartViewType,
    /// Maximum age of threads to include.
    pub max_age: Option<Duration>,
    /// Whether to include archived threads.
    pub include_archived: bool,
    /// Custom AI confidence threshold.
    pub confidence_threshold: f32,
}

impl Default for SmartViewCriteria {
    fn default() -> Self {
        Self {
            view_type: SmartViewType::NeedsReply,
            max_age: None,
            include_archived: false,
            confidence_threshold: 0.7,
        }
    }
}

impl SmartViewCriteria {
    /// Creates criteria for needs reply view.
    pub fn needs_reply() -> Self {
        Self {
            view_type: SmartViewType::NeedsReply,
            max_age: Some(Duration::days(30)),
            include_archived: false,
            confidence_threshold: 0.6,
        }
    }

    /// Creates criteria for waiting for view.
    pub fn waiting_for() -> Self {
        Self {
            view_type: SmartViewType::WaitingFor,
            max_age: Some(Duration::days(14)),
            include_archived: false,
            confidence_threshold: 0.7,
        }
    }

    /// Creates criteria for newsletters view.
    pub fn newsletters() -> Self {
        Self {
            view_type: SmartViewType::Newsletters,
            max_age: Some(Duration::days(7)),
            include_archived: true,
            confidence_threshold: 0.8,
        }
    }

    /// Creates criteria for VIP view.
    pub fn vip() -> Self {
        Self {
            view_type: SmartViewType::Vip,
            max_age: None,
            include_archived: false,
            confidence_threshold: 0.9,
        }
    }

    /// Creates criteria for follow up view.
    pub fn follow_up() -> Self {
        Self {
            view_type: SmartViewType::FollowUp,
            max_age: None,
            include_archived: false,
            confidence_threshold: 0.5,
        }
    }
}

/// Result of smart view classification for a thread.
#[derive(Debug, Clone)]
pub struct SmartViewMatch {
    /// Thread ID.
    pub thread_id: ThreadId,
    /// Type of smart view matched.
    pub view_type: SmartViewType,
    /// AI confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Reason for classification.
    pub reason: String,
    /// When this classification was made.
    pub classified_at: DateTime<Utc>,
    /// Whether manually assigned by user.
    pub manual: bool,
}

impl SmartViewMatch {
    /// Creates a new AI-classified match.
    pub fn ai_classified(
        thread_id: ThreadId,
        view_type: SmartViewType,
        confidence: f32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            thread_id,
            view_type,
            confidence,
            reason: reason.into(),
            classified_at: Utc::now(),
            manual: false,
        }
    }

    /// Creates a manually assigned match.
    pub fn manual(
        thread_id: ThreadId,
        view_type: SmartViewType,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            thread_id,
            view_type,
            confidence: 1.0,
            reason: reason.into(),
            classified_at: Utc::now(),
            manual: true,
        }
    }
}

/// Smart view sidebar entry with count.
#[derive(Debug, Clone)]
pub struct SmartViewEntry {
    /// View type.
    pub view_type: SmartViewType,
    /// Number of threads in this view.
    pub count: u32,
    /// Number of unread threads.
    pub unread_count: u32,
    /// Whether this view is selected.
    pub is_selected: bool,
}

impl SmartViewEntry {
    /// Creates a new entry.
    pub fn new(view_type: SmartViewType) -> Self {
        Self {
            view_type,
            count: 0,
            unread_count: 0,
            is_selected: false,
        }
    }

    /// Updates the counts.
    pub fn with_counts(mut self, count: u32, unread: u32) -> Self {
        self.count = count;
        self.unread_count = unread;
        self
    }

    /// Marks as selected.
    pub fn selected(mut self) -> Self {
        self.is_selected = true;
        self
    }
}

/// Smart Views panel component for the sidebar.
pub struct SmartViewsPanel {
    /// Available smart views.
    entries: Vec<SmartViewEntry>,
    /// Currently selected view.
    selected: Option<SmartViewType>,
    /// Whether the panel is expanded.
    expanded: bool,
}

impl SmartViewsPanel {
    /// Creates a new smart views panel.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let entries = SmartViewType::all()
            .iter()
            .map(|&vt| SmartViewEntry::new(vt))
            .collect();

        Self {
            entries,
            selected: None,
            expanded: true,
        }
    }

    /// Updates the counts for all views.
    pub fn update_counts(&mut self, counts: &[(SmartViewType, u32, u32)]) {
        for (view_type, count, unread) in counts {
            if let Some(entry) = self.entries.iter_mut().find(|e| e.view_type == *view_type) {
                entry.count = *count;
                entry.unread_count = *unread;
            }
        }
    }

    /// Selects a smart view.
    pub fn select(&mut self, view_type: SmartViewType) {
        self.selected = Some(view_type);
        for entry in &mut self.entries {
            entry.is_selected = entry.view_type == view_type;
        }
    }

    /// Clears selection.
    pub fn clear_selection(&mut self) {
        self.selected = None;
        for entry in &mut self.entries {
            entry.is_selected = false;
        }
    }

    /// Toggles the panel expansion.
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Returns the selected view type.
    pub fn selected_view(&self) -> Option<SmartViewType> {
        self.selected
    }

    fn render_entry(&self, entry: &SmartViewEntry) -> impl IntoElement {
        let name = entry.view_type.display_name();
        let icon = entry.view_type.icon();
        let color = entry.view_type.color();
        let count = entry.count;
        let unread = entry.unread_count;
        let is_selected = entry.is_selected;
        let shortcut = entry.view_type.shortcut();

        div()
            .id(SharedString::from(format!(
                "smart-view-{:?}",
                entry.view_type
            )))
            .px(px(8.0))
            .py(px(6.0))
            .mx(px(4.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .when(is_selected, |d| d.bg(rgba(0x3F3F46FF)))
            .when(!is_selected, |d| d.hover(|d| d.bg(rgba(0x27272A80))))
            .flex()
            .items_center()
            .gap(px(8.0))
            // Icon
            .child(
                div()
                    .size(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().text_xs().text_color(rgba(color)).child(icon)),
            )
            // Name
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .when(is_selected, |d| d.text_color(rgba(0xF4F4F5FF)))
                    .when(!is_selected, |d| d.text_color(rgba(0xA1A1AAFF)))
                    .child(name),
            )
            // Shortcut hint
            .when_some(shortcut, |d, sc| {
                d.child(div().text_xs().text_color(rgba(0x52525BFF)).child(sc))
            })
            // Count badge
            .when(count > 0, |d| {
                d.child(
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(10.0))
                        .when(unread > 0, |d| d.bg(rgba(color & 0xFFFFFF40)))
                        .when(unread == 0, |d| d.bg(rgba(0x27272AFF)))
                        .child(
                            div()
                                .text_xs()
                                .when(unread > 0, |d| {
                                    d.text_color(rgba(color))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                })
                                .when(unread == 0, |d| d.text_color(rgba(0x71717AFF)))
                                .child(format!("{}", count)),
                        ),
                )
            })
    }
}

impl Render for SmartViewsPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("smart-views-panel")
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .id("smart-views-header")
                    .px(px(12.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgba(0x71717AFF))
                                    .child(if self.expanded { "v" } else { ">" }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgba(0x71717AFF))
                                    .child("SMART VIEWS"),
                            ),
                    ),
            )
            // Entries
            .when(self.expanded, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .children(self.entries.iter().map(|entry| self.render_entry(entry))),
                )
            })
    }
}

/// Manager for computing and caching smart view classifications.
pub struct SmartViewManager {
    /// Cached classifications.
    classifications: Vec<SmartViewMatch>,
    /// When cache was last updated.
    last_updated: Option<DateTime<Utc>>,
    /// Whether a refresh is in progress.
    refreshing: bool,
}

impl SmartViewManager {
    /// Creates a new manager.
    pub fn new() -> Self {
        Self {
            classifications: Vec::new(),
            last_updated: None,
            refreshing: false,
        }
    }

    /// Returns threads matching a smart view type.
    pub fn threads_for_view(&self, view_type: SmartViewType) -> Vec<&SmartViewMatch> {
        self.classifications
            .iter()
            .filter(|m| m.view_type == view_type)
            .collect()
    }

    /// Returns the count for a view type.
    pub fn count_for_view(&self, view_type: SmartViewType) -> u32 {
        self.classifications
            .iter()
            .filter(|m| m.view_type == view_type)
            .count() as u32
    }

    /// Adds a classification.
    pub fn add_classification(&mut self, match_entry: SmartViewMatch) {
        // Remove existing classification for this thread/view combo
        self.classifications.retain(|m| {
            m.thread_id != match_entry.thread_id || m.view_type != match_entry.view_type
        });
        self.classifications.push(match_entry);
    }

    /// Removes classifications for a thread.
    pub fn remove_thread(&mut self, thread_id: &ThreadId) {
        self.classifications.retain(|m| m.thread_id != *thread_id);
    }

    /// Manually assigns a thread to a smart view.
    pub fn assign_manual(
        &mut self,
        thread_id: ThreadId,
        view_type: SmartViewType,
        reason: impl Into<String>,
    ) {
        self.add_classification(SmartViewMatch::manual(thread_id, view_type, reason));
    }

    /// Sets refresh state.
    pub fn set_refreshing(&mut self, refreshing: bool) {
        self.refreshing = refreshing;
        if !refreshing {
            self.last_updated = Some(Utc::now());
        }
    }

    /// Returns whether refresh is needed.
    pub fn needs_refresh(&self) -> bool {
        match self.last_updated {
            None => true,
            Some(t) => Utc::now() - t > Duration::minutes(5),
        }
    }

    /// Returns all counts for sidebar display.
    pub fn all_counts(&self) -> Vec<(SmartViewType, u32, u32)> {
        SmartViewType::all()
            .iter()
            .map(|&vt| {
                let count = self.count_for_view(vt);
                // TODO: track unread per classification
                (vt, count, 0)
            })
            .collect()
    }
}

impl Default for SmartViewManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_view_type_properties() {
        for vt in SmartViewType::all() {
            assert!(!vt.display_name().is_empty());
            assert!(!vt.icon().is_empty());
            assert!(vt.color() != 0);
        }
    }

    #[test]
    fn smart_view_criteria_defaults() {
        let needs_reply = SmartViewCriteria::needs_reply();
        assert_eq!(needs_reply.view_type, SmartViewType::NeedsReply);
        assert!(needs_reply.max_age.is_some());
        assert!(!needs_reply.include_archived);

        let newsletters = SmartViewCriteria::newsletters();
        assert_eq!(newsletters.view_type, SmartViewType::Newsletters);
        assert!(newsletters.include_archived); // Newsletters can be archived
    }

    #[test]
    fn smart_view_manager_classifications() {
        let mut manager = SmartViewManager::new();

        let thread_id = ThreadId::from("test-thread".to_string());
        manager.add_classification(SmartViewMatch::ai_classified(
            thread_id.clone(),
            SmartViewType::NeedsReply,
            0.85,
            "User was last recipient",
        ));

        assert_eq!(manager.count_for_view(SmartViewType::NeedsReply), 1);
        assert_eq!(manager.count_for_view(SmartViewType::WaitingFor), 0);

        manager.remove_thread(&thread_id);
        assert_eq!(manager.count_for_view(SmartViewType::NeedsReply), 0);
    }

    #[test]
    fn smart_view_entry_counts() {
        let entry = SmartViewEntry::new(SmartViewType::Vip)
            .with_counts(42, 5)
            .selected();

        assert_eq!(entry.count, 42);
        assert_eq!(entry.unread_count, 5);
        assert!(entry.is_selected);
    }
}
