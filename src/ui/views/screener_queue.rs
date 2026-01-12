//! Screener queue component.
//!
//! Displays unknown senders awaiting review, with AI-powered analysis
//! and one-click approve/reject actions.

use chrono::{DateTime, Utc};
use gpui::{
    div, prelude::*, px, rgba, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

use crate::domain::{ScreenerAction, SenderType};

/// A sender entry in the screener queue.
#[derive(Debug, Clone)]
pub struct ScreenerEntry {
    /// Unique ID.
    pub id: String,
    /// Sender email address.
    pub email: String,
    /// Sender name if known.
    pub name: Option<String>,
    /// Subject of first email.
    pub first_email_subject: Option<String>,
    /// Preview of first email.
    pub first_email_preview: String,
    /// When the first email was received.
    pub received_at: DateTime<Utc>,
    /// AI-determined sender type.
    pub ai_sender_type: Option<SenderType>,
    /// AI reasoning.
    pub ai_reasoning: Option<String>,
    /// AI-suggested action.
    pub ai_suggested_action: Option<ScreenerAction>,
    /// Whether this entry is selected.
    pub is_selected: bool,
}

impl ScreenerEntry {
    /// Creates a new screener entry.
    pub fn new(id: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            email: email.into(),
            name: None,
            first_email_subject: None,
            first_email_preview: String::new(),
            received_at: Utc::now(),
            ai_sender_type: None,
            ai_reasoning: None,
            ai_suggested_action: None,
            is_selected: false,
        }
    }

    /// Sets the sender name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the first email details.
    pub fn with_email_preview(
        mut self,
        subject: Option<String>,
        preview: impl Into<String>,
    ) -> Self {
        self.first_email_subject = subject;
        self.first_email_preview = preview.into();
        self
    }

    /// Sets the AI analysis.
    pub fn with_ai_analysis(
        mut self,
        sender_type: SenderType,
        reasoning: impl Into<String>,
        suggested: ScreenerAction,
    ) -> Self {
        self.ai_sender_type = Some(sender_type);
        self.ai_reasoning = Some(reasoning.into());
        self.ai_suggested_action = Some(suggested);
        self
    }

    /// Returns the display name (name or email).
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.email)
    }

    /// Returns the sender type badge text.
    pub fn sender_type_badge(&self) -> Option<&'static str> {
        self.ai_sender_type.map(|t| match t {
            SenderType::KnownContact => "Contact",
            SenderType::Newsletter => "Newsletter",
            SenderType::Marketing => "Marketing",
            SenderType::Recruiter => "Recruiter",
            SenderType::Support => "Support",
            SenderType::Unknown => "Unknown",
        })
    }

    /// Returns the suggested action text.
    pub fn suggested_action_text(&self) -> Option<&'static str> {
        self.ai_suggested_action.map(|a| match a {
            ScreenerAction::Approve => "Approve",
            ScreenerAction::Reject => "Reject",
            ScreenerAction::Review => "Review",
        })
    }
}

/// The screener queue view component.
pub struct ScreenerQueue {
    /// Whether the queue is visible.
    visible: bool,
    /// Entries in the queue.
    entries: Vec<ScreenerEntry>,
    /// Currently selected entry index.
    selected_index: Option<usize>,
    /// Whether AI analysis is loading.
    #[allow(dead_code)]
    ai_loading: bool,
    /// Filter by sender type.
    filter: Option<SenderType>,
}

impl ScreenerQueue {
    /// Creates a new screener queue.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
            selected_index: None,
            ai_loading: false,
            filter: None,
        }
    }

    /// Opens the screener queue.
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// Closes the screener queue.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Returns whether the queue is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Sets the entries.
    pub fn set_entries(&mut self, entries: Vec<ScreenerEntry>) {
        self.entries = entries;
        self.selected_index = if self.entries.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Adds an entry.
    pub fn add_entry(&mut self, entry: ScreenerEntry) {
        self.entries.push(entry);
    }

    /// Removes an entry by ID.
    pub fn remove_entry(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
        if let Some(idx) = self.selected_index {
            if idx >= self.entries.len() {
                self.selected_index = if self.entries.is_empty() {
                    None
                } else {
                    Some(self.entries.len() - 1)
                };
            }
        }
    }

    /// Returns the number of entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Selects the next entry.
    pub fn select_next(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.entries.len() - 1 {
                self.selected_index = Some(idx + 1);
            }
        }
    }

    /// Selects the previous entry.
    pub fn select_previous(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx > 0 {
                self.selected_index = Some(idx - 1);
            }
        }
    }

    /// Returns the selected entry.
    pub fn selected_entry(&self) -> Option<&ScreenerEntry> {
        self.selected_index.and_then(|i| self.entries.get(i))
    }

    /// Approves the selected entry.
    pub fn approve_selected(&mut self) -> Option<String> {
        let id = self.selected_entry()?.id.clone();
        self.remove_entry(&id);
        Some(id)
    }

    /// Rejects the selected entry.
    pub fn reject_selected(&mut self) -> Option<String> {
        let id = self.selected_entry()?.id.clone();
        self.remove_entry(&id);
        Some(id)
    }

    /// Approves all entries with AI-suggested approval.
    pub fn approve_all_suggested(&mut self) -> Vec<String> {
        let ids: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.ai_suggested_action == Some(ScreenerAction::Approve))
            .map(|e| e.id.clone())
            .collect();

        for id in &ids {
            self.remove_entry(id);
        }

        ids
    }

    /// Sets the filter.
    pub fn set_filter(&mut self, filter: Option<SenderType>) {
        self.filter = filter;
    }

    /// Returns filtered entries.
    fn filtered_entries(&self) -> Vec<&ScreenerEntry> {
        match self.filter {
            Some(filter_type) => self
                .entries
                .iter()
                .filter(|e| e.ai_sender_type == Some(filter_type))
                .collect(),
            None => self.entries.iter().collect(),
        }
    }

    fn render_entry(&self, entry: &ScreenerEntry, index: usize) -> impl IntoElement {
        let is_selected = self.selected_index == Some(index);
        let display_name = entry.display_name().to_string();
        let email = entry.email.clone();
        let first_char = email
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        let badge = entry.sender_type_badge();
        let subject = entry.first_email_subject.clone();
        let preview = entry.first_email_preview.clone();
        let reasoning = entry.ai_reasoning.clone();
        let entry_id = entry.id.clone();

        div()
            .id(SharedString::from(format!("entry-{}", entry_id)))
            .px(px(16.0))
            .py(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .cursor_pointer()
            .border_b_1()
            .border_color(rgba(0x27272AFF))
            .when(is_selected, |d| d.bg(rgba(0x3B82F610)))
            .when(!is_selected, |d| d.hover(|d| d.bg(rgba(0xFFFFFF05))))
            // Header row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // Avatar
                    .child(
                        div()
                            .size(px(36.0))
                            .rounded_full()
                            .bg(rgba(0x71717AFF))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(rgba(0xFFFFFFFF))
                            .child(first_char),
                    )
                    // Name and email
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgba(0xF4F4F5FF))
                                    .truncate()
                                    .child(display_name),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgba(0x71717AFF))
                                    .truncate()
                                    .child(email),
                            ),
                    )
                    // Type badge
                    .when_some(badge, |d, b| {
                        d.child(
                            div()
                                .px(px(8.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(rgba(0x3F3F46FF))
                                .text_xs()
                                .text_color(rgba(0xA1A1AAFF))
                                .child(b),
                        )
                    }),
            )
            // Email preview
            .child(
                div()
                    .pl(px(44.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .when_some(subject, |d, s| {
                        d.child(
                            div()
                                .text_sm()
                                .text_color(rgba(0xE4E4E7FF))
                                .truncate()
                                .child(s),
                        )
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0x71717AFF))
                            .truncate()
                            .child(preview),
                    ),
            )
            // AI reasoning
            .when_some(reasoning, |d, r| {
                d.child(
                    div()
                        .pl(px(44.0))
                        .mt(px(4.0))
                        .p(px(8.0))
                        .bg(rgba(0x1E3A5F40))
                        .rounded(px(4.0))
                        .flex()
                        .gap(px(8.0))
                        .child(div().text_xs().text_color(rgba(0x60A5FAFF)).child("AI:"))
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .text_color(rgba(0xBFDBFEFF))
                                .child(r),
                        ),
                )
            })
            // Action buttons when selected
            .when(is_selected, |d| {
                d.child(
                    div()
                        .pl(px(44.0))
                        .mt(px(8.0))
                        .flex()
                        .gap(px(8.0))
                        .child(
                            div()
                                .id("approve-btn")
                                .px(px(12.0))
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(rgba(0x22C55E20))
                                .hover(|d| d.bg(rgba(0x22C55E40)))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgba(0x22C55EFF))
                                        .child("Approve"),
                                ),
                        )
                        .child(
                            div()
                                .id("reject-btn")
                                .px(px(12.0))
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(rgba(0xEF444420))
                                .hover(|d| d.bg(rgba(0xEF444440)))
                                .child(
                                    div().text_sm().text_color(rgba(0xEF4444FF)).child("Reject"),
                                ),
                        )
                        .child(
                            div()
                                .id("view-email-btn")
                                .px(px(12.0))
                                .h(px(28.0))
                                .flex()
                                .items_center()
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(rgba(0x27272AFF))
                                .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgba(0xA1A1AAFF))
                                        .child("View Email"),
                                ),
                        ),
                )
            })
    }

    fn render_empty(&self) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.0))
            .child(div().text_2xl().text_color(rgba(0x52525BFF)).child("inbox"))
            .child(
                div()
                    .text_lg()
                    .text_color(rgba(0x71717AFF))
                    .child("No pending senders"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgba(0x52525BFF))
                    .text_center()
                    .child("New senders will appear here for your review"),
            )
    }
}

impl Render for ScreenerQueue {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("screener-hidden");
        }

        let filtered = self.filtered_entries();
        let count = filtered.len();

        // Backdrop
        div()
            .id("screener-backdrop")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Panel container
                div()
                    .id("screener-panel")
                    .w(px(600.0))
                    .h(px(640.0))
                    .bg(rgba(0x18181BFF))
                    .rounded(px(12.0))
                    .shadow_lg()
                    .border_1()
                    .border_color(rgba(0x27272AFF))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Header
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(rgba(0xF4F4F5FF))
                                            .child("Screener Queue"),
                                    )
                                    .child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(2.0))
                                            .bg(rgba(0x3B82F6FF))
                                            .rounded_full()
                                            .text_xs()
                                            .text_color(rgba(0xFFFFFFFF))
                                            .child(format!("{}", count)),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-screener")
                                    .size(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(rgba(0x27272AFF)))
                                    .child(div().text_lg().text_color(rgba(0x71717AFF)).child("x")),
                            ),
                    )
                    // Toolbar
                    .child(
                        div()
                            .h(px(48.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("Filter:"),
                                    )
                                    .child(
                                        div()
                                            .px(px(8.0))
                                            .h(px(28.0))
                                            .flex()
                                            .items_center()
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgba(0xA1A1AAFF))
                                                    .child("All Types"),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("approve-suggested")
                                    .px(px(12.0))
                                    .h(px(28.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .bg(rgba(0x22C55E20))
                                    .hover(|d| d.bg(rgba(0x22C55E40)))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x22C55EFF))
                                            .child("Approve All Suggested"),
                                    ),
                            ),
                    )
                    // Entry list
                    .child(if count == 0 {
                        div().flex_1().child(self.render_empty())
                    } else {
                        div().flex_1().overflow_hidden().children(
                            self.entries
                                .iter()
                                .enumerate()
                                .map(|(i, entry)| self.render_entry(entry, i)),
                        )
                    })
                    // Footer hints
                    .child(
                        div()
                            .h(px(40.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .gap(px(16.0))
                            .border_t_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("j/k"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .child("navigate"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("a"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .child("approve"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("r"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .child("reject"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("Esc"),
                                    )
                                    .child(
                                        div().text_xs().text_color(rgba(0x52525BFF)).child("close"),
                                    ),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screener_entry_builder() {
        let entry = ScreenerEntry::new("entry-1", "test@example.com")
            .with_name("Test User")
            .with_email_preview(Some("Hello".to_string()), "This is a preview...")
            .with_ai_analysis(
                SenderType::Newsletter,
                "Appears to be a newsletter based on sender domain",
                ScreenerAction::Reject,
            );

        assert_eq!(entry.display_name(), "Test User");
        assert_eq!(entry.sender_type_badge(), Some("Newsletter"));
        assert_eq!(entry.suggested_action_text(), Some("Reject"));
    }

    #[test]
    fn screener_entry_display_name_fallback() {
        let entry = ScreenerEntry::new("entry-1", "test@example.com");
        assert_eq!(entry.display_name(), "test@example.com");
    }

    #[test]
    fn screener_queue_selection() {
        // Selection tests require ViewContext
    }
}
