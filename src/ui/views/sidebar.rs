//! Sidebar view.
//!
//! Contains navigation, account list, mailboxes, labels, and smart views.

use gpui::{
    div, prelude::FluentBuilder, px, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};

use crate::app::ViewType;
use crate::domain::{AccountId, LabelId};
use crate::ui::theme::ThemeColors;

/// Sidebar view component.
pub struct Sidebar {
    colors: ThemeColors,
    active_view: ViewType,
    accounts: Vec<SidebarAccount>,
    labels: Vec<SidebarLabel>,
    collapsed: bool,
}

/// Account representation for sidebar.
#[derive(Clone)]
pub struct SidebarAccount {
    pub id: AccountId,
    pub email: String,
    pub display_name: Option<String>,
    pub unread_count: u32,
}

/// Label representation for sidebar.
#[derive(Clone)]
pub struct SidebarLabel {
    pub id: LabelId,
    pub name: String,
    pub color: Option<String>,
    pub unread_count: u32,
}

impl Sidebar {
    /// Create a new sidebar.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            colors: ThemeColors::dark(),
            active_view: ViewType::Inbox,
            accounts: Vec::new(),
            labels: Vec::new(),
            collapsed: false,
        }
    }

    /// Set the active view.
    pub fn set_active_view(&mut self, view: ViewType) {
        self.active_view = view;
    }

    /// Set the list of accounts.
    pub fn set_accounts(&mut self, accounts: Vec<SidebarAccount>) {
        self.accounts = accounts;
    }

    /// Set the list of labels.
    pub fn set_labels(&mut self, labels: Vec<SidebarLabel>) {
        self.labels = labels;
    }

    /// Toggle sidebar collapsed state.
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    fn render_mailbox_item(
        &self,
        id: impl Into<ElementId>,
        label: &str,
        icon: &str,
        view_type: ViewType,
        unread: Option<u32>,
    ) -> impl IntoElement {
        let is_active = self.active_view == view_type;
        let bg = if is_active {
            self.colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = self.colors.surface_elevated;
        let text_color = self.colors.text_primary;
        let muted_color = self.colors.text_muted;

        let mut item = div()
            .id(id.into())
            .px(px(12.0))
            .py(px(8.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
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
                                    .text_color(muted_color)
                                    .child(SharedString::from(icon.to_string())),
                            )
                            .child(
                                div()
                                    .text_color(text_color)
                                    .child(SharedString::from(label.to_string())),
                            ),
                    ),
            );

        if let Some(count) = unread {
            if count > 0 {
                item = item.child(
                    div()
                        .text_xs()
                        .text_color(muted_color)
                        .child(SharedString::from(count.to_string())),
                );
            }
        }

        item
    }

    fn render_section_header(&self, label: &str) -> impl IntoElement {
        div()
            .px(px(20.0))
            .py(px(8.0))
            .text_xs()
            .text_color(self.colors.text_muted)
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(SharedString::from(label.to_string()))
    }

    fn render_label_item(&self, label: &SidebarLabel) -> impl IntoElement {
        let is_active = matches!(&self.active_view, ViewType::Label(id) if *id == label.id);
        let bg = if is_active {
            self.colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = self.colors.surface_elevated;

        div()
            .id(SharedString::from(format!("label-{}", label.id.0)))
            .px(px(12.0))
            .py(px(6.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(8.0))
                            .rounded_full()
                            .bg(self.colors.accent),
                    )
                    .child(
                        div()
                            .text_color(self.colors.text_primary)
                            .child(SharedString::from(label.name.clone())),
                    ),
            )
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let width = if self.collapsed { 60.0 } else { 220.0 };

        div()
            .id("sidebar")
            .w(px(width))
            .h_full()
            .flex()
            .flex_col()
            .bg(self.colors.surface)
            .border_r_1()
            .border_color(self.colors.border)
            .child(
                div()
                    .flex_1()
                    .overflow_y_hidden()
                    .child(
                        div()
                            .py(px(8.0))
                            .child(self.render_mailbox_item(
                                "inbox",
                                "Inbox",
                                "I",
                                ViewType::Inbox,
                                Some(3),
                            ))
                            .child(self.render_mailbox_item(
                                "starred",
                                "Starred",
                                "S",
                                ViewType::Starred,
                                None,
                            ))
                            .child(self.render_mailbox_item(
                                "snoozed",
                                "Snoozed",
                                "Z",
                                ViewType::Snoozed,
                                None,
                            ))
                            .child(self.render_mailbox_item(
                                "sent",
                                "Sent",
                                ">",
                                ViewType::Sent,
                                None,
                            ))
                            .child(self.render_mailbox_item(
                                "drafts",
                                "Drafts",
                                "D",
                                ViewType::Drafts,
                                Some(1),
                            ))
                            .child(self.render_mailbox_item(
                                "archive",
                                "Archive",
                                "A",
                                ViewType::Archive,
                                None,
                            ))
                            .child(self.render_mailbox_item(
                                "trash",
                                "Trash",
                                "T",
                                ViewType::Trash,
                                None,
                            )),
                    )
                    .child(self.render_section_header("SCREENER"))
                    .child(
                        div().child(self.render_mailbox_item(
                            "screener",
                            "New Senders",
                            "?",
                            ViewType::Screener,
                            Some(5),
                        )),
                    )
                    .when(!self.labels.is_empty(), |this| {
                        this.child(self.render_section_header("LABELS"))
                            .children(self.labels.iter().map(|l| self.render_label_item(l)))
                    }),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_t_1()
                    .border_color(self.colors.border)
                    .child(self.render_mailbox_item(
                        "settings",
                        "Settings",
                        "G",
                        ViewType::Settings,
                        None,
                    ))
                    .child(self.render_mailbox_item(
                        "stats",
                        "Statistics",
                        "#",
                        ViewType::Stats,
                        None,
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_account() {
        let account = SidebarAccount {
            id: AccountId::from("acc-1"),
            email: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            unread_count: 5,
        };

        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.unread_count, 5);
    }

    #[test]
    fn sidebar_label() {
        let label = SidebarLabel {
            id: LabelId::from("label-1"),
            name: "Important".to_string(),
            color: Some("#ff0000".to_string()),
            unread_count: 2,
        };

        assert_eq!(label.name, "Important");
    }
}
