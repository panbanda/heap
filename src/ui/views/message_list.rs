//! Message list view.
//!
//! Displays a virtualized list of email threads for the current view.

use gpui::{
    div, prelude::FluentBuilder, px, ClickEvent, Context, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::app::ViewType;
use crate::domain::ThreadId;
use crate::ui::components::VirtualizedListState;
use crate::ui::theme::ThemeColors;

/// Callback type for thread selection.
type OnSelectCallback = Box<dyn Fn(ThreadId) + 'static>;

/// Message list view component.
pub struct MessageList {
    colors: ThemeColors,
    view_type: ViewType,
    threads: Vec<ThreadListItem>,
    selected_thread_id: Option<ThreadId>,
    focused_index: usize,
    list_state: VirtualizedListState,
    loading: bool,
    on_select: Option<OnSelectCallback>,
}

/// Thread item for the message list.
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
    pub has_attachments: bool,
}

impl MessageList {
    /// Create a new message list.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            colors: ThemeColors::dark(),
            view_type: ViewType::Inbox,
            threads: Vec::new(),
            selected_thread_id: None,
            focused_index: 0,
            list_state: VirtualizedListState::new(0).with_item_height(72.0),
            loading: false,
            on_select: None,
        }
    }

    /// Set the callback for when a thread is selected (clicked).
    pub fn on_select(&mut self, callback: impl Fn(ThreadId) + 'static) {
        self.on_select = Some(Box::new(callback));
    }

    /// Set the current view type.
    pub fn set_view_type(&mut self, view_type: ViewType) {
        self.view_type = view_type;
    }

    /// Set the list of threads.
    pub fn set_threads(&mut self, threads: Vec<ThreadListItem>) {
        self.list_state = VirtualizedListState::new(threads.len()).with_item_height(72.0);
        self.threads = threads;
    }

    /// Set the selected thread.
    pub fn set_selected(&mut self, thread_id: Option<ThreadId>) {
        self.selected_thread_id = thread_id.clone();
        if let Some(id) = &thread_id {
            if let Some(idx) = self.threads.iter().position(|t| t.id == *id) {
                self.focused_index = idx;
            }
        }
    }

    /// Set loading state.
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Move focus to next thread.
    pub fn focus_next(&mut self) {
        if self.focused_index + 1 < self.threads.len() {
            self.focused_index += 1;
            self.selected_thread_id = Some(self.threads[self.focused_index].id.clone());
            self.ensure_focused_visible();
        }
    }

    /// Move focus to previous thread.
    pub fn focus_previous(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
            self.selected_thread_id = Some(self.threads[self.focused_index].id.clone());
            self.ensure_focused_visible();
        }
    }

    /// Ensure the focused item is visible in the viewport.
    fn ensure_focused_visible(&mut self) {
        let item_top = self.list_state.item_offset(self.focused_index);
        let item_bottom = item_top + self.list_state.item_height;
        let scroll_top = self.list_state.scroll_offset;
        let scroll_bottom = scroll_top + self.list_state.viewport_height;

        if item_top < scroll_top {
            self.list_state.scroll_offset = item_top;
        } else if item_bottom > scroll_bottom {
            self.list_state.scroll_offset = item_bottom - self.list_state.viewport_height;
        }
    }

    /// Get the currently focused thread.
    pub fn focused_thread(&self) -> Option<&ThreadListItem> {
        self.threads.get(self.focused_index)
    }

    /// Update the viewport height for virtualization.
    pub fn set_viewport_height(&mut self, height: f32) {
        self.list_state.viewport_height = height;
    }

    /// Handle scroll by delta pixels.
    pub fn scroll_by(&mut self, delta: f32) {
        self.list_state.scroll_by(delta);
    }

    /// Scroll to ensure focused item is visible.
    pub fn scroll_to_focused(&mut self) {
        self.list_state.scroll_to_item(self.focused_index);
    }

    /// Get the visible range of items.
    pub fn visible_range(&self) -> std::ops::Range<usize> {
        self.list_state.visible_range()
    }

    fn view_title(&self) -> &str {
        match &self.view_type {
            ViewType::Inbox => "Inbox",
            ViewType::Starred => "Starred",
            ViewType::Sent => "Sent",
            ViewType::Drafts => "Drafts",
            ViewType::Archive => "Archive",
            ViewType::Trash => "Trash",
            ViewType::Snoozed => "Snoozed",
            ViewType::Label(_) => "Label",
            ViewType::Screener => "New Senders",
            ViewType::Search(_) => "Search Results",
            ViewType::Settings => "Settings",
            ViewType::Stats => "Statistics",
            ViewType::Actionable => "Actionable",
            ViewType::Newsletters => "Newsletters",
            ViewType::Social => "Social",
            ViewType::Updates => "Updates",
        }
    }

    fn render_header(&self) -> impl IntoElement {
        div()
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(self.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(self.colors.text_primary)
                            .child(SharedString::from(self.view_title().to_string())),
                    )
                    .child(div().text_sm().text_color(self.colors.text_muted).child(
                        SharedString::from(format!("{} threads", self.threads.len())),
                    )),
            )
    }

    fn render_empty_state(&self) -> impl IntoElement {
        div().flex_1().flex().items_center().justify_center().child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .text_color(self.colors.text_primary)
                        .child(SharedString::from("No messages")),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(self.colors.text_muted)
                        .child(SharedString::from("Your inbox is empty")),
                ),
        )
    }

    fn render_loading_state(&self) -> impl IntoElement {
        div().flex_1().flex().items_center().justify_center().child(
            div()
                .text_color(self.colors.text_muted)
                .child(SharedString::from("Loading...")),
        )
    }
}

impl Render for MessageList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get visible range from virtualization state
        let visible_range = self.list_state.visible_range();
        let total_height = self.list_state.total_height();

        // Only render visible items with their absolute positions
        let thread_items: Vec<_> = self
            .threads
            .iter()
            .enumerate()
            .filter(|(idx, _)| visible_range.contains(idx))
            .map(|(idx, thread)| {
                let offset = self.list_state.item_offset(idx);
                self.render_thread_item_virtualized(thread, idx, offset, cx)
            })
            .collect();

        div()
            .id("message-list")
            .w(px(380.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(self.colors.background)
            .border_r_1()
            .border_color(self.colors.border)
            .child(self.render_header())
            .child(
                div()
                    .flex_1()
                    .overflow_y_hidden()
                    .when(self.loading, |this| this.child(self.render_loading_state()))
                    .when(!self.loading && self.threads.is_empty(), |this| {
                        this.child(self.render_empty_state())
                    })
                    .when(!self.loading && !self.threads.is_empty(), |this| {
                        // Container with total height for proper scrolling
                        this.child(
                            div()
                                .relative()
                                .h(px(total_height))
                                .w_full()
                                .children(thread_items),
                        )
                    }),
            )
    }
}

impl MessageList {
    fn render_thread_item_virtualized(
        &self,
        thread: &ThreadListItem,
        index: usize,
        top_offset: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self
            .selected_thread_id
            .as_ref()
            .is_some_and(|id| *id == thread.id);
        let is_focused = index == self.focused_index;
        let item_height = self.list_state.item_height;

        let bg = if is_selected {
            self.colors.surface_elevated
        } else if is_focused {
            self.colors.surface
        } else {
            gpui::Hsla::transparent_black()
        };

        let text_weight = if thread.is_unread {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        };

        let hover_bg = self.colors.surface;
        let border_color = self.colors.border;
        let text_primary = self.colors.text_primary;
        let text_secondary = self.colors.text_secondary;
        let text_muted = self.colors.text_muted;
        let starred_color = self.colors.starred;

        let thread_id = thread.id.clone();
        let click_handler = cx.listener(move |this, _event: &ClickEvent, _window, _cx| {
            this.selected_thread_id = Some(thread_id.clone());
            if let Some(idx) = this.threads.iter().position(|t| t.id == thread_id) {
                this.focused_index = idx;
            }
            if let Some(ref callback) = this.on_select {
                callback(thread_id.clone());
            }
        });

        div()
            .id(SharedString::from(format!("thread-{}", index)))
            .absolute()
            .left_0()
            .right_0()
            .top(px(top_offset))
            .h(px(item_height))
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
                                this.child(div().text_xs().text_color(text_muted).child(
                                    SharedString::from(format!("({})", thread.message_count)),
                                ))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_thread(id: &str, subject: &str, unread: bool) -> ThreadListItem {
        ThreadListItem {
            id: ThreadId::from(id),
            subject: subject.to_string(),
            sender_name: "Sender".to_string(),
            sender_email: "sender@example.com".to_string(),
            snippet: "Preview text...".to_string(),
            timestamp: "12:30".to_string(),
            is_unread: unread,
            is_starred: false,
            message_count: 1,
            has_attachments: false,
        }
    }

    #[test]
    fn thread_list_item() {
        let thread = make_thread("thread-1", "Test Subject", true);
        assert_eq!(thread.subject, "Test Subject");
        assert!(thread.is_unread);
    }

    #[test]
    fn view_titles() {
        let list = MessageList {
            colors: ThemeColors::dark(),
            view_type: ViewType::Inbox,
            threads: Vec::new(),
            selected_thread_id: None,
            focused_index: 0,
            list_state: VirtualizedListState::new(0),
            loading: false,
            on_select: None,
        };
        assert_eq!(list.view_title(), "Inbox");
    }

    #[test]
    fn focus_navigation() {
        let mut list = MessageList {
            colors: ThemeColors::dark(),
            view_type: ViewType::Inbox,
            threads: vec![
                make_thread("1", "Thread 1", false),
                make_thread("2", "Thread 2", false),
                make_thread("3", "Thread 3", false),
            ],
            selected_thread_id: None,
            focused_index: 0,
            list_state: VirtualizedListState::new(3),
            loading: false,
            on_select: None,
        };

        assert_eq!(list.focused_index, 0);

        list.focus_next();
        assert_eq!(list.focused_index, 1);

        list.focus_next();
        assert_eq!(list.focused_index, 2);

        list.focus_next(); // Should stay at 2
        assert_eq!(list.focused_index, 2);

        list.focus_previous();
        assert_eq!(list.focused_index, 1);
    }

    #[test]
    fn virtualization_visible_range() {
        let mut list = MessageList {
            colors: ThemeColors::dark(),
            view_type: ViewType::Inbox,
            threads: (0..100)
                .map(|i| make_thread(&format!("{}", i), &format!("Thread {}", i), false))
                .collect(),
            selected_thread_id: None,
            focused_index: 0,
            list_state: VirtualizedListState::new(100)
                .with_item_height(72.0)
                .with_viewport_height(360.0) // 5 items visible
                .with_buffer(2),
            loading: false,
            on_select: None,
        };

        // Initially should show items 0-6 (5 visible + 2 buffer below)
        let range = list.visible_range();
        assert_eq!(range.start, 0);
        assert!(range.end <= 8); // buffer of 2 on each side

        // Scroll down
        list.scroll_by(144.0); // 2 items
        let range = list.visible_range();
        // Buffer may include item 0
        assert!(range.contains(&2));
        assert!(range.contains(&6));
    }

    #[test]
    fn virtualization_scroll_to_focused() {
        let mut list = MessageList {
            colors: ThemeColors::dark(),
            view_type: ViewType::Inbox,
            threads: (0..50)
                .map(|i| make_thread(&format!("{}", i), &format!("Thread {}", i), false))
                .collect(),
            selected_thread_id: None,
            focused_index: 0,
            list_state: VirtualizedListState::new(50)
                .with_item_height(72.0)
                .with_viewport_height(360.0),
            loading: false,
            on_select: None,
        };

        // Navigate to item 10
        for _ in 0..10 {
            list.focus_next();
        }
        assert_eq!(list.focused_index, 10);

        // The scroll offset should have adjusted to keep item visible
        let visible = list.visible_range();
        assert!(visible.contains(&10));
    }
}
