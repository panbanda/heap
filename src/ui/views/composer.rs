//! Composer view.
//!
//! Email composition window for new messages, replies, and forwards.

use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};

use crate::app::ComposerMode;
use crate::ui::theme::ThemeColors;

/// Composer view component.
pub struct Composer {
    colors: ThemeColors,
    mode: ComposerMode,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    body: String,
    attachments: Vec<ComposerAttachment>,
    is_dirty: bool,
    is_sending: bool,
    ai_suggestion: Option<String>,
    show_cc: bool,
    show_bcc: bool,
}

/// Attachment in composer.
#[derive(Clone)]
pub struct ComposerAttachment {
    pub filename: String,
    pub size_bytes: u64,
    pub path: String,
}

impl Composer {
    /// Create a new composer.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            colors: ThemeColors::dark(),
            mode: ComposerMode::New,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            body: String::new(),
            attachments: Vec::new(),
            is_dirty: false,
            is_sending: false,
            ai_suggestion: None,
            show_cc: false,
            show_bcc: false,
        }
    }

    /// Create a composer for reply.
    pub fn for_reply(
        to: String,
        subject: String,
        _cx: &mut Context<Self>,
    ) -> Self {
        let subject = if subject.starts_with("Re:") {
            subject
        } else {
            format!("Re: {}", subject)
        };

        Self {
            colors: ThemeColors::dark(),
            mode: ComposerMode::Reply,
            to: vec![to],
            cc: Vec::new(),
            bcc: Vec::new(),
            subject,
            body: String::new(),
            attachments: Vec::new(),
            is_dirty: false,
            is_sending: false,
            ai_suggestion: None,
            show_cc: false,
            show_bcc: false,
        }
    }

    /// Set the AI suggestion.
    pub fn set_ai_suggestion(&mut self, suggestion: String) {
        self.ai_suggestion = Some(suggestion);
    }

    /// Accept AI suggestion.
    pub fn accept_ai_suggestion(&mut self) {
        if let Some(suggestion) = self.ai_suggestion.take() {
            self.body = suggestion;
            self.is_dirty = true;
        }
    }

    /// Reject AI suggestion.
    pub fn reject_ai_suggestion(&mut self) {
        self.ai_suggestion = None;
    }

    /// Add attachment.
    pub fn add_attachment(&mut self, attachment: ComposerAttachment) {
        self.attachments.push(attachment);
        self.is_dirty = true;
    }

    /// Remove attachment.
    pub fn remove_attachment(&mut self, index: usize) {
        if index < self.attachments.len() {
            self.attachments.remove(index);
            self.is_dirty = true;
        }
    }

    /// Check if can send.
    pub fn can_send(&self) -> bool {
        !self.to.is_empty() && !self.is_sending
    }

    /// Set sending state.
    pub fn set_sending(&mut self, sending: bool) {
        self.is_sending = sending;
    }

    fn mode_title(&self) -> &str {
        match self.mode {
            ComposerMode::New => "New Message",
            ComposerMode::Reply => "Reply",
            ComposerMode::ReplyAll => "Reply All",
            ComposerMode::Forward => "Forward",
            ComposerMode::EditDraft => "Edit Draft",
        }
    }

    fn render_header(&self) -> impl IntoElement {
        let surface = self.colors.surface;
        let border = self.colors.border;
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;

        div()
            .px(px(16.0))
            .py(px(12.0))
            .bg(surface)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_primary)
                            .child(SharedString::from(self.mode_title().to_string())),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_sm()
                                    .text_color(text_muted)
                                    .cursor_pointer()
                                    .child(SharedString::from("Discard")),
                            )
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(self.colors.accent)
                                    .text_sm()
                                    .text_color(text_primary)
                                    .cursor_pointer()
                                    .child(SharedString::from("Send")),
                            ),
                    ),
            )
    }

    fn render_field(&self, label: &str, value: &str, placeholder: &str) -> impl IntoElement {
        let border = self.colors.border;
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;

        let display = if value.is_empty() {
            placeholder.to_string()
        } else {
            value.to_string()
        };
        let color = if value.is_empty() {
            text_muted
        } else {
            text_primary
        };

        div()
            .px(px(16.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(60.0))
                            .text_sm()
                            .text_color(text_muted)
                            .child(SharedString::from(label.to_string())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(color)
                            .child(SharedString::from(display)),
                    ),
            )
    }

    fn render_body_area(&self) -> impl IntoElement {
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;

        let display = if self.body.is_empty() {
            "Compose your message..."
        } else {
            &self.body
        };
        let color = if self.body.is_empty() {
            text_muted
        } else {
            text_primary
        };

        div()
            .flex_1()
            .p(px(16.0))
            .text_color(color)
            .child(SharedString::from(display.to_string()))
    }

    fn render_ai_suggestion(&self, suggestion: &str) -> impl IntoElement {
        let surface = self.colors.surface_elevated;
        let border = self.colors.border;
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;
        let accent = self.colors.accent;

        div()
            .m(px(16.0))
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .mb(px(8.0))
                    .child(
                        div()
                            .text_sm()
                            .text_color(accent)
                            .font_weight(FontWeight::MEDIUM)
                            .child(SharedString::from("AI Suggestion")),
                    ),
            )
            .child(
                div()
                    .text_color(text_primary)
                    .mb(px(12.0))
                    .child(SharedString::from(suggestion.to_string())),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(accent)
                            .text_sm()
                            .text_color(text_primary)
                            .cursor_pointer()
                            .child(SharedString::from("Accept")),
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .text_sm()
                            .text_color(text_muted)
                            .cursor_pointer()
                            .child(SharedString::from("Dismiss")),
                    ),
            )
    }

    fn render_attachments(&self) -> impl IntoElement {
        let border = self.colors.border;
        let surface = self.colors.surface;
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;

        div()
            .px(px(16.0))
            .py(px(8.0))
            .border_t_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .children(self.attachments.iter().map(|att| {
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
                                    .cursor_pointer()
                                    .child(SharedString::from("x")),
                            )
                    }))
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .text_sm()
                            .text_color(text_muted)
                            .cursor_pointer()
                            .child(SharedString::from("+ Add attachment")),
                    ),
            )
    }
}

impl Render for Composer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("composer")
            .size_full()
            .flex()
            .flex_col()
            .bg(self.colors.background)
            .child(self.render_header())
            .child(self.render_field("To", &self.to.join(", "), "Recipients"))
            .when(self.show_cc, |this| {
                this.child(self.render_field("Cc", &self.cc.join(", "), "Cc recipients"))
            })
            .when(self.show_bcc, |this| {
                this.child(self.render_field("Bcc", &self.bcc.join(", "), "Bcc recipients"))
            })
            .child(self.render_field("Subject", &self.subject, "Subject"))
            .when_some(self.ai_suggestion.clone(), |this, suggestion| {
                this.child(self.render_ai_suggestion(&suggestion))
            })
            .child(self.render_body_area())
            .when(!self.attachments.is_empty(), |this| {
                this.child(self.render_attachments())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composer_can_send() {
        let mut composer = Composer {
            colors: ThemeColors::dark(),
            mode: ComposerMode::New,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            body: String::new(),
            attachments: Vec::new(),
            is_dirty: false,
            is_sending: false,
            ai_suggestion: None,
            show_cc: false,
            show_bcc: false,
        };

        assert!(!composer.can_send());

        composer.to.push("test@example.com".to_string());
        assert!(composer.can_send());

        composer.is_sending = true;
        assert!(!composer.can_send());
    }

    #[test]
    fn ai_suggestion_flow() {
        let mut composer = Composer {
            colors: ThemeColors::dark(),
            mode: ComposerMode::Reply,
            to: vec!["test@example.com".to_string()],
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: "Re: Test".to_string(),
            body: String::new(),
            attachments: Vec::new(),
            is_dirty: false,
            is_sending: false,
            ai_suggestion: None,
            show_cc: false,
            show_bcc: false,
        };

        composer.set_ai_suggestion("AI drafted reply".to_string());
        assert!(composer.ai_suggestion.is_some());

        composer.accept_ai_suggestion();
        assert_eq!(composer.body, "AI drafted reply");
        assert!(composer.ai_suggestion.is_none());
        assert!(composer.is_dirty);
    }

    #[test]
    fn attachment_management() {
        let mut composer = Composer {
            colors: ThemeColors::dark(),
            mode: ComposerMode::New,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            body: String::new(),
            attachments: Vec::new(),
            is_dirty: false,
            is_sending: false,
            ai_suggestion: None,
            show_cc: false,
            show_bcc: false,
        };

        composer.add_attachment(ComposerAttachment {
            filename: "test.pdf".to_string(),
            size_bytes: 1024,
            path: "/tmp/test.pdf".to_string(),
        });

        assert_eq!(composer.attachments.len(), 1);
        assert!(composer.is_dirty);

        composer.remove_attachment(0);
        assert!(composer.attachments.is_empty());
    }
}
