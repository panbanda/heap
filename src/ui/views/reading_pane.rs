//! Reading pane view.
//!
//! Displays the selected email thread with messages and actions.

use std::collections::HashSet;

use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};

use crate::domain::{EmailId, ThreadId};
use crate::ui::theme::ThemeColors;

/// Reading pane view component.
pub struct ReadingPane {
    colors: ThemeColors,
    thread: Option<ThreadDetail>,
    expanded_messages: HashSet<EmailId>,
    inline_composer_visible: bool,
    scroll_offset: f32,
}

/// Detailed thread data for display.
#[derive(Clone)]
pub struct ThreadDetail {
    pub id: ThreadId,
    pub subject: String,
    pub messages: Vec<MessageDetail>,
    pub labels: Vec<String>,
}

/// Individual message in a thread.
#[derive(Clone)]
pub struct MessageDetail {
    pub id: EmailId,
    pub sender_name: String,
    pub sender_email: String,
    pub recipients: Vec<String>,
    pub timestamp: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub is_unread: bool,
}

/// Attachment information.
#[derive(Clone)]
pub struct AttachmentInfo {
    pub id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub content_type: String,
}

impl ReadingPane {
    /// Create a new reading pane.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            colors: ThemeColors::dark(),
            thread: None,
            expanded_messages: HashSet::new(),
            inline_composer_visible: false,
            scroll_offset: 0.0,
        }
    }

    /// Set the displayed thread.
    pub fn set_thread(&mut self, thread: Option<ThreadDetail>) {
        self.expanded_messages.clear();
        self.inline_composer_visible = false;
        self.scroll_offset = 0.0;

        if let Some(ref t) = thread {
            // Expand the last message by default
            if let Some(last) = t.messages.last() {
                self.expanded_messages.insert(last.id.clone());
            }
        }

        self.thread = thread;
    }

    /// Toggle message expansion.
    pub fn toggle_message(&mut self, message_id: &EmailId) {
        if self.expanded_messages.contains(message_id) {
            self.expanded_messages.remove(message_id);
        } else {
            self.expanded_messages.insert(message_id.clone());
        }
    }

    /// Expand all messages.
    pub fn expand_all(&mut self) {
        if let Some(ref thread) = self.thread {
            for msg in &thread.messages {
                self.expanded_messages.insert(msg.id.clone());
            }
        }
    }

    /// Collapse all messages.
    pub fn collapse_all(&mut self) {
        self.expanded_messages.clear();
    }

    /// Show inline composer for reply.
    pub fn show_composer(&mut self) {
        self.inline_composer_visible = true;
    }

    /// Hide inline composer.
    pub fn hide_composer(&mut self) {
        self.inline_composer_visible = false;
    }

    fn render_empty_state(&self) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(self.colors.text_muted)
                    .child(SharedString::from("Select a message to read")),
            )
    }

    fn render_thread_header(&self, thread: &ThreadDetail) -> impl IntoElement {
        div()
            .px(px(24.0))
            .py(px(16.0))
            .border_b_1()
            .border_color(self.colors.border)
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.colors.text_primary)
                    .child(SharedString::from(thread.subject.clone())),
            )
            .when(!thread.labels.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .mt(px(8.0))
                        .children(thread.labels.iter().map(|label| {
                            div()
                                .px(px(8.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(self.colors.surface_elevated)
                                .text_xs()
                                .text_color(self.colors.text_secondary)
                                .child(SharedString::from(label.clone()))
                        })),
                )
            })
    }

    fn render_message(&self, message: &MessageDetail, is_expanded: bool) -> impl IntoElement {
        let border_color = self.colors.border;
        let surface = self.colors.surface;
        let text_primary = self.colors.text_primary;
        let text_secondary = self.colors.text_secondary;
        let text_muted = self.colors.text_muted;

        if is_expanded {
            div()
                .px(px(24.0))
                .py(px(16.0))
                .border_b_1()
                .border_color(border_color)
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
                                        .bg(surface)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_color(text_secondary)
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
                                                .text_color(text_primary)
                                                .child(SharedString::from(
                                                    message.sender_name.clone(),
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(text_muted)
                                                .child(SharedString::from(format!(
                                                    "to {}",
                                                    message.recipients.join(", ")
                                                ))),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(text_muted)
                                .child(SharedString::from(message.timestamp.clone())),
                        ),
                )
                .child(
                    div()
                        .text_color(text_primary)
                        .child(SharedString::from(message.body_text.clone())),
                )
                .when(!message.attachments.is_empty(), |this| {
                    this.child(
                        div()
                            .mt(px(16.0))
                            .pt(px(12.0))
                            .border_t_1()
                            .border_color(border_color)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(text_muted)
                                    .mb(px(8.0))
                                    .child(SharedString::from(format!(
                                        "{} attachment(s)",
                                        message.attachments.len()
                                    ))),
                            )
                            .children(message.attachments.iter().map(|att| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(surface)
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(text_primary)
                                            .child(SharedString::from(att.filename.clone())),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_muted)
                                            .child(SharedString::from(format_size(att.size_bytes))),
                                    )
                            })),
                    )
                })
        } else {
            div()
                .px(px(24.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(border_color)
                .cursor_pointer()
                .hover(move |style| style.bg(surface))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            div()
                                .size(px(32.0))
                                .rounded_full()
                                .bg(surface)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(text_secondary)
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
                                            div()
                                                .text_sm()
                                                .text_color(text_primary)
                                                .child(SharedString::from(
                                                    message.sender_name.clone(),
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(text_muted)
                                                .child(SharedString::from(
                                                    message.timestamp.clone(),
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(text_secondary)
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

    fn render_inline_composer(&self) -> impl IntoElement {
        div()
            .px(px(24.0))
            .py(px(16.0))
            .border_t_1()
            .border_color(self.colors.border)
            .child(
                div()
                    .p(px(12.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(self.colors.border)
                    .bg(self.colors.surface)
                    .child(
                        div()
                            .text_color(self.colors.text_muted)
                            .child(SharedString::from("Click to reply...")),
                    ),
            )
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

impl Render for ReadingPane {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("reading-pane")
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(self.colors.background)
            .when(self.thread.is_none(), |this| {
                this.child(self.render_empty_state())
            })
            .when_some(self.thread.clone(), |this, thread| {
                this.child(self.render_thread_header(&thread))
                    .child(
                        div().flex_1().overflow_y_hidden().children(
                            thread.messages.iter().map(|msg| {
                                let is_expanded = self.expanded_messages.contains(&msg.id);
                                self.render_message(msg, is_expanded)
                            }),
                        ),
                    )
                    .when(self.inline_composer_visible, |this| {
                        this.child(self.render_inline_composer())
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn truncate_text_short() {
        assert_eq!(truncate_text("short", 10), "short");
    }

    #[test]
    fn truncate_text_long() {
        assert_eq!(truncate_text("this is a long text", 10), "this is a ...");
    }

    #[test]
    fn attachment_info() {
        let attachment = AttachmentInfo {
            id: "att-1".to_string(),
            filename: "document.pdf".to_string(),
            size_bytes: 1024 * 100,
            content_type: "application/pdf".to_string(),
        };

        assert_eq!(attachment.filename, "document.pdf");
    }

    #[test]
    fn message_expansion() {
        let mut pane = ReadingPane {
            colors: ThemeColors::dark(),
            thread: None,
            expanded_messages: HashSet::new(),
            inline_composer_visible: false,
            scroll_offset: 0.0,
        };

        let msg_id = EmailId::from("msg-1");

        pane.expanded_messages.insert(msg_id.clone());
        assert!(pane.expanded_messages.contains(&msg_id));

        pane.toggle_message(&msg_id);
        assert!(!pane.expanded_messages.contains(&msg_id));
    }
}
