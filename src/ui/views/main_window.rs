//! Main application window
//!
//! Integrates sidebar, message list, and reading pane with full interactivity.

use std::collections::HashSet;

use gpui::{
    div, prelude::FluentBuilder, px, ClickEvent, Context, FocusHandle, Focusable, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::app::{NextMessage, PreviousMessage, ViewType};
use crate::domain::{EmailId, LabelId, ThreadId};
use crate::ui::theme::Theme;

/// Main window view containing the primary application layout
pub struct MainWindow {
    theme: Theme,
    focus_handle: FocusHandle,

    // App state
    current_view: ViewType,

    // Sidebar state
    sidebar_labels: Vec<SidebarLabel>,

    // Message list state
    threads: Vec<ThreadListItem>,
    selected_thread_id: Option<ThreadId>,
    focused_index: usize,

    // Reading pane state
    current_thread: Option<ThreadDetail>,
    expanded_messages: HashSet<EmailId>,
}

/// Label representation for sidebar
#[derive(Clone)]
pub struct SidebarLabel {
    pub id: LabelId,
    pub name: String,
    pub color: Option<String>,
}

/// Thread item for the message list
#[derive(Clone)]
pub struct ThreadListItem {
    pub id: ThreadId,
    pub subject: String,
    pub sender_name: String,
    pub sender_email: String,
    pub snippet: String,
    pub timestamp: String,
    pub is_unread: bool,
    pub is_starred: bool,
    pub message_count: u32,
}

/// Detailed thread data for reading pane
#[derive(Clone)]
pub struct ThreadDetail {
    pub id: ThreadId,
    pub subject: String,
    pub messages: Vec<MessageDetail>,
    pub labels: Vec<String>,
}

/// Individual message in a thread
#[derive(Clone)]
pub struct MessageDetail {
    pub id: EmailId,
    pub sender_name: String,
    pub sender_email: String,
    pub recipients: Vec<String>,
    pub timestamp: String,
    pub body_text: String,
    pub is_unread: bool,
}

impl MainWindow {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let mut this = Self {
            theme: Theme::dark(),
            focus_handle,
            current_view: ViewType::Inbox,
            sidebar_labels: Vec::new(),
            threads: Vec::new(),
            selected_thread_id: None,
            focused_index: 0,
            current_thread: None,
            expanded_messages: HashSet::new(),
        };

        this.load_sample_data();
        // Focus is managed via track_focus() in render

        this
    }

    fn load_sample_data(&mut self) {
        self.threads = vec![
            ThreadListItem {
                id: ThreadId::from("thread-1"),
                subject: "Welcome to The Heap".to_string(),
                sender_name: "The Heap Team".to_string(),
                sender_email: "team@theheap.email".to_string(),
                snippet: "Get started with your new email client...".to_string(),
                timestamp: "Just now".to_string(),
                is_unread: true,
                is_starred: true,
                message_count: 1,
            },
            ThreadListItem {
                id: ThreadId::from("thread-2"),
                subject: "Project Update: Q1 Planning".to_string(),
                sender_name: "Alice Chen".to_string(),
                sender_email: "alice@example.com".to_string(),
                snippet: "Hey team, I wanted to share the latest updates...".to_string(),
                timestamp: "10:30 AM".to_string(),
                is_unread: true,
                is_starred: false,
                message_count: 5,
            },
            ThreadListItem {
                id: ThreadId::from("thread-3"),
                subject: "Re: Code Review Request".to_string(),
                sender_name: "Bob Smith".to_string(),
                sender_email: "bob@example.com".to_string(),
                snippet: "Looks good to me! Just a few minor suggestions...".to_string(),
                timestamp: "Yesterday".to_string(),
                is_unread: false,
                is_starred: false,
                message_count: 3,
            },
            ThreadListItem {
                id: ThreadId::from("thread-4"),
                subject: "Meeting Notes - Product Sync".to_string(),
                sender_name: "Carol Davis".to_string(),
                sender_email: "carol@example.com".to_string(),
                snippet: "Here are the notes from today's meeting...".to_string(),
                timestamp: "Yesterday".to_string(),
                is_unread: false,
                is_starred: true,
                message_count: 1,
            },
            ThreadListItem {
                id: ThreadId::from("thread-5"),
                subject: "Weekend Plans?".to_string(),
                sender_name: "David Lee".to_string(),
                sender_email: "david@example.com".to_string(),
                snippet: "Anyone up for hiking this weekend?".to_string(),
                timestamp: "2 days ago".to_string(),
                is_unread: false,
                is_starred: false,
                message_count: 8,
            },
        ];

        self.sidebar_labels = vec![
            SidebarLabel {
                id: LabelId::from("work"),
                name: "Work".to_string(),
                color: Some("#3b82f6".to_string()),
            },
            SidebarLabel {
                id: LabelId::from("personal"),
                name: "Personal".to_string(),
                color: Some("#22c55e".to_string()),
            },
        ];
    }

    fn navigate_to(&mut self, view: ViewType, cx: &mut Context<Self>) {
        self.current_view = view;
        self.selected_thread_id = None;
        self.current_thread = None;
        self.focused_index = 0;
        cx.notify();
    }

    fn select_thread(&mut self, thread_id: ThreadId, cx: &mut Context<Self>) {
        self.selected_thread_id = Some(thread_id.clone());

        // Find index
        if let Some(idx) = self.threads.iter().position(|t| t.id == thread_id) {
            self.focused_index = idx;
        }

        // Load thread detail
        self.current_thread = Some(self.get_thread_detail(&thread_id));
        self.expanded_messages.clear();

        // Expand last message
        if let Some(ref thread) = self.current_thread {
            if let Some(last) = thread.messages.last() {
                self.expanded_messages.insert(last.id.clone());
            }
        }

        cx.notify();
    }

    fn focus_next(&mut self, cx: &mut Context<Self>) {
        if self.focused_index + 1 < self.threads.len() {
            self.focused_index += 1;
            let thread_id = self.threads[self.focused_index].id.clone();
            self.select_thread(thread_id, cx);
        }
    }

    fn focus_previous(&mut self, cx: &mut Context<Self>) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
            let thread_id = self.threads[self.focused_index].id.clone();
            self.select_thread(thread_id, cx);
        }
    }

    fn get_thread_detail(&self, thread_id: &ThreadId) -> ThreadDetail {
        match thread_id.0.as_str() {
            "thread-1" => ThreadDetail {
                id: thread_id.clone(),
                subject: "Welcome to The Heap".to_string(),
                messages: vec![MessageDetail {
                    id: EmailId::from("msg-1-1"),
                    sender_name: "The Heap Team".to_string(),
                    sender_email: "team@theheap.email".to_string(),
                    recipients: vec!["you@example.com".to_string()],
                    timestamp: "Today at 9:00 AM".to_string(),
                    body_text: "Welcome to The Heap!\n\nWe're excited to have you on board. Here are some tips to get started:\n\n1. Use 'j' and 'k' to navigate through your messages\n2. Press 'e' to archive, 's' to star\n3. Press 'c' to compose a new email\n4. Press '/' to search\n\nEnjoy your new email experience!".to_string(),
                    is_unread: true,
                }],
                labels: vec!["Getting Started".to_string()],
            },
            "thread-2" => ThreadDetail {
                id: thread_id.clone(),
                subject: "Project Update: Q1 Planning".to_string(),
                messages: vec![
                    MessageDetail {
                        id: EmailId::from("msg-2-1"),
                        sender_name: "Alice Chen".to_string(),
                        sender_email: "alice@example.com".to_string(),
                        recipients: vec!["team@example.com".to_string()],
                        timestamp: "Today at 10:30 AM".to_string(),
                        body_text: "Hey team,\n\nI wanted to share the latest updates on our Q1 planning. We've made great progress on the roadmap.\n\nKey highlights:\n- Feature A is on track for release next week\n- Feature B needs some additional work\n- We'll be hiring two new engineers\n\nLet me know if you have any questions!".to_string(),
                        is_unread: false,
                    },
                    MessageDetail {
                        id: EmailId::from("msg-2-2"),
                        sender_name: "You".to_string(),
                        sender_email: "you@example.com".to_string(),
                        recipients: vec!["alice@example.com".to_string()],
                        timestamp: "Today at 10:45 AM".to_string(),
                        body_text: "Thanks for the update, Alice! This looks great.\n\nQuick question - what's the timeline for Feature B?".to_string(),
                        is_unread: false,
                    },
                    MessageDetail {
                        id: EmailId::from("msg-2-3"),
                        sender_name: "Alice Chen".to_string(),
                        sender_email: "alice@example.com".to_string(),
                        recipients: vec!["you@example.com".to_string()],
                        timestamp: "Today at 11:00 AM".to_string(),
                        body_text: "Good question! We're aiming for end of February, but I'll have a more concrete timeline by next week.".to_string(),
                        is_unread: true,
                    },
                ],
                labels: vec!["Work".to_string(), "Planning".to_string()],
            },
            _ => ThreadDetail {
                id: thread_id.clone(),
                subject: self.threads.iter().find(|t| t.id == *thread_id).map(|t| t.subject.clone()).unwrap_or_else(|| "Message".to_string()),
                messages: vec![MessageDetail {
                    id: EmailId::from("msg-default"),
                    sender_name: self.threads.iter().find(|t| t.id == *thread_id).map(|t| t.sender_name.clone()).unwrap_or_else(|| "Sender".to_string()),
                    sender_email: "sender@example.com".to_string(),
                    recipients: vec!["you@example.com".to_string()],
                    timestamp: "Recently".to_string(),
                    body_text: self.threads.iter().find(|t| t.id == *thread_id).map(|t| t.snippet.clone()).unwrap_or_else(|| "This is a sample message.".to_string()),
                    is_unread: false,
                }],
                labels: vec![],
            },
        }
    }

    fn render_title_bar(&self) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id("title-bar")
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .bg(colors.surface)
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from("The Heap")),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(colors.surface_elevated)
                    .text_color(colors.text_muted)
                    .text_sm()
                    .child(SharedString::from("Press / to search")),
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let inbox = self.render_sidebar_item("inbox", "Inbox", ViewType::Inbox, Some(2), cx);
        let starred = self.render_sidebar_item("starred", "Starred", ViewType::Starred, None, cx);
        let snoozed = self.render_sidebar_item("snoozed", "Snoozed", ViewType::Snoozed, None, cx);
        let sent = self.render_sidebar_item("sent", "Sent", ViewType::Sent, None, cx);
        let drafts = self.render_sidebar_item("drafts", "Drafts", ViewType::Drafts, Some(1), cx);
        let archive = self.render_sidebar_item("archive", "Archive", ViewType::Archive, None, cx);
        let trash = self.render_sidebar_item("trash", "Trash", ViewType::Trash, None, cx);
        let screener = self.render_sidebar_item("screener", "New Senders", ViewType::Screener, Some(3), cx);

        div()
            .id("sidebar")
            .w(px(220.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.surface)
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex_1()
                    .overflow_y_hidden()
                    .py(px(8.0))
                    .child(inbox)
                    .child(starred)
                    .child(snoozed)
                    .child(sent)
                    .child(drafts)
                    .child(archive)
                    .child(trash)
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(12.0))
                            .text_xs()
                            .text_color(colors.text_muted)
                            .font_weight(FontWeight::MEDIUM)
                            .child(SharedString::from("SCREENER")),
                    )
                    .child(screener)
                    .when(!self.sidebar_labels.is_empty(), |this| {
                        this.child(
                            div()
                                .px(px(20.0))
                                .py(px(12.0))
                                .text_xs()
                                .text_color(colors.text_muted)
                                .font_weight(FontWeight::MEDIUM)
                                .child(SharedString::from("LABELS")),
                        )
                    })
                    .children(self.sidebar_labels.iter().map(|label| {
                        self.render_label_item(label, cx)
                    })),
            )
    }

    fn render_sidebar_item(
        &self,
        id: &str,
        label: &str,
        view: ViewType,
        count: Option<u32>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_active = self.current_view == view;
        let bg = if is_active {
            colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = colors.surface_elevated;
        let text_color = colors.text_primary;
        let muted_color = colors.text_muted;

        let target_view = view.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.navigate_to(target_view.clone(), cx);
        });

        div()
            .id(SharedString::from(id.to_string()))
            .px(px(12.0))
            .py(px(8.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(click_handler)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(text_color)
                            .child(SharedString::from(label.to_string())),
                    )
                    .when_some(count, |this, c| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(muted_color)
                                .child(SharedString::from(c.to_string())),
                        )
                    }),
            )
    }

    fn render_label_item(&self, label: &SidebarLabel, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_active = matches!(&self.current_view, ViewType::Label(id) if *id == label.id);
        let bg = if is_active {
            colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = colors.surface_elevated;

        let label_id = label.id.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.navigate_to(ViewType::Label(label_id.clone()), cx);
        });

        div()
            .id(SharedString::from(format!("label-{}", label.id.0)))
            .px(px(12.0))
            .py(px(6.0))
            .mx(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(click_handler)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().size(px(8.0)).rounded_full().bg(colors.accent))
                    .child(
                        div()
                            .text_color(colors.text_primary)
                            .child(SharedString::from(label.name.clone())),
                    ),
            )
    }

    fn render_message_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;
        let view_title = self.view_title();

        let thread_items: Vec<_> = self
            .threads
            .iter()
            .enumerate()
            .map(|(idx, thread)| self.render_thread_item(thread, idx, cx))
            .collect();

        div()
            .id("message-list")
            .w(px(380.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.text_primary)
                                    .child(SharedString::from(view_title.to_string())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_muted)
                                    .child(SharedString::from(format!("{} threads", self.threads.len()))),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_y_hidden()
                    .children(thread_items),
            )
    }

    fn render_thread_item(
        &self,
        thread: &ThreadListItem,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = &self.theme.colors;
        let is_selected = self.selected_thread_id.as_ref() == Some(&thread.id);
        let is_focused = index == self.focused_index;

        let bg = if is_selected {
            colors.surface_elevated
        } else if is_focused {
            colors.surface
        } else {
            gpui::Hsla::transparent_black()
        };

        let text_weight = if thread.is_unread {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        };

        let hover_bg = colors.surface;
        let border_color = colors.border;
        let text_primary = colors.text_primary;
        let text_secondary = colors.text_secondary;
        let text_muted = colors.text_muted;
        let starred_color = colors.starred;

        let thread_id = thread.id.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.select_thread(thread_id.clone(), cx);
        });

        div()
            .id(SharedString::from(format!("thread-{}", index)))
            .px(px(16.0))
            .py(px(12.0))
            .bg(bg)
            .border_b_1()
            .border_color(border_color)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_click(click_handler)
            .child(
                div()
                    .flex()
                    .justify_between()
                    .mb(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .font_weight(text_weight)
                                    .text_color(text_primary)
                                    .child(SharedString::from(thread.sender_name.clone())),
                            )
                            .when(thread.message_count > 1, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(text_muted)
                                        .child(SharedString::from(format!("({})", thread.message_count))),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when(thread.is_starred, |this| {
                                this.child(
                                    div()
                                        .text_color(starred_color)
                                        .child(SharedString::from("*")),
                                )
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child(SharedString::from(thread.timestamp.clone())),
                            ),
                    ),
            )
            .child(
                div()
                    .font_weight(text_weight)
                    .text_color(text_primary)
                    .text_sm()
                    .truncate()
                    .child(SharedString::from(thread.subject.clone())),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(text_secondary)
                    .truncate()
                    .child(SharedString::from(thread.snippet.clone())),
            )
    }

    fn render_reading_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id("reading-pane")
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .when(self.current_thread.is_none(), |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_color(colors.text_muted)
                                .child(SharedString::from("Select a message to read")),
                        ),
                )
            })
            .when_some(self.current_thread.clone(), |this, thread| {
                this.child(self.render_thread_header(&thread))
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_hidden()
                            .children(thread.messages.iter().map(|msg| {
                                let is_expanded = self.expanded_messages.contains(&msg.id);
                                self.render_message(msg, is_expanded, cx)
                            })),
                    )
            })
    }

    fn render_thread_header(&self, thread: &ThreadDetail) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .px(px(24.0))
            .py(px(16.0))
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
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
                                .bg(colors.surface_elevated)
                                .text_xs()
                                .text_color(colors.text_secondary)
                                .child(SharedString::from(label.clone()))
                        })),
                )
            })
    }

    fn render_message(&self, message: &MessageDetail, is_expanded: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        let msg_id = message.id.clone();
        let click_handler = cx.listener(move |this, _: &ClickEvent, _, cx| {
            if this.expanded_messages.contains(&msg_id) {
                this.expanded_messages.remove(&msg_id);
            } else {
                this.expanded_messages.insert(msg_id.clone());
            }
            cx.notify();
        });

        if is_expanded {
            div()
                .id(SharedString::from(format!("msg-{}", message.id.0)))
                .px(px(24.0))
                .py(px(16.0))
                .border_b_1()
                .border_color(colors.border)
                .cursor_pointer()
                .on_click(click_handler)
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
                                        .bg(colors.surface_elevated)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_color(colors.text_secondary)
                                        .child(SharedString::from(
                                            message.sender_name.chars().next().unwrap_or('?').to_string(),
                                        )),
                                )
                                .child(
                                    div()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(colors.text_primary)
                                                .child(SharedString::from(message.sender_name.clone())),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(colors.text_muted)
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
                                .text_color(colors.text_muted)
                                .child(SharedString::from(message.timestamp.clone())),
                        ),
                )
                .child(
                    div()
                        .text_color(colors.text_primary)
                        .child(SharedString::from(message.body_text.clone())),
                )
        } else {
            div()
                .id(SharedString::from(format!("msg-{}", message.id.0)))
                .px(px(24.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(colors.border)
                .cursor_pointer()
                .hover(move |style| style.bg(colors.surface))
                .on_click(click_handler)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            div()
                                .size(px(32.0))
                                .rounded_full()
                                .bg(colors.surface_elevated)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(colors.text_secondary)
                                .child(SharedString::from(
                                    message.sender_name.chars().next().unwrap_or('?').to_string(),
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
                                                .text_color(colors.text_primary)
                                                .child(SharedString::from(message.sender_name.clone())),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(colors.text_muted)
                                                .child(SharedString::from(message.timestamp.clone())),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(colors.text_secondary)
                                        .truncate()
                                        .child(SharedString::from(truncate_text(&message.body_text, 80))),
                                ),
                        ),
                )
        }
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let colors = &self.theme.colors;
        let status = if self.selected_thread_id.is_some() {
            "1 selected"
        } else {
            "Ready"
        };

        div()
            .id("status-bar")
            .h(px(24.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .bg(colors.surface)
            .border_t_1()
            .border_color(colors.border)
            .child(
                div()
                    .text_color(colors.text_muted)
                    .text_xs()
                    .child(SharedString::from(status.to_string())),
            )
            .child(
                div()
                    .text_color(colors.text_muted)
                    .text_xs()
                    .child(SharedString::from(self.view_title().to_string())),
            )
    }

    fn view_title(&self) -> &str {
        match &self.current_view {
            ViewType::Inbox => "Inbox",
            ViewType::Starred => "Starred",
            ViewType::Sent => "Sent",
            ViewType::Drafts => "Drafts",
            ViewType::Archive => "Archive",
            ViewType::Trash => "Trash",
            ViewType::Snoozed => "Snoozed",
            ViewType::Screener => "New Senders",
            ViewType::Settings => "Settings",
            ViewType::Stats => "Statistics",
            ViewType::Label(_) => "Label",
            ViewType::Search(_) => "Search",
        }
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or(text);
    if first_line.len() <= max_len {
        first_line.to_string()
    } else {
        format!("{}...", &first_line[..max_len])
    }
}

impl Focusable for MainWindow {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &self.theme.colors;

        div()
            .id("main-window")
            .key_context("MainWindow")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &NextMessage, _, cx| {
                this.focus_next(cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousMessage, _, cx| {
                this.focus_previous(cx);
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .text_color(colors.text_primary)
            .child(self.render_title_bar())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(self.render_sidebar(cx))
                    .child(self.render_message_list(cx))
                    .child(self.render_reading_pane(cx)),
            )
            .child(self.render_status_bar())
    }
}
