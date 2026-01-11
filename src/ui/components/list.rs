//! List components.
//!
//! Provides virtualized lists for efficient rendering of large datasets.

use std::ops::Range;

use gpui::{
    div, px, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled,
};

use crate::ui::theme::ThemeColors;

/// State for a virtualized list.
#[derive(Debug, Clone, Default)]
pub struct VirtualizedListState {
    /// Total number of items.
    pub total_items: usize,
    /// Height of each item in pixels.
    pub item_height: f32,
    /// Current scroll offset in pixels.
    pub scroll_offset: f32,
    /// Viewport height in pixels.
    pub viewport_height: f32,
    /// Number of items to render above/below viewport as buffer.
    pub buffer_count: usize,
}

impl VirtualizedListState {
    /// Create new list state with defaults.
    pub fn new(total_items: usize) -> Self {
        Self {
            total_items,
            item_height: 56.0,
            scroll_offset: 0.0,
            viewport_height: 500.0,
            buffer_count: 5,
        }
    }

    /// Set item height.
    pub fn with_item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }

    /// Set viewport height.
    pub fn with_viewport_height(mut self, height: f32) -> Self {
        self.viewport_height = height;
        self
    }

    /// Set buffer count.
    pub fn with_buffer(mut self, count: usize) -> Self {
        self.buffer_count = count;
        self
    }

    /// Get the range of visible item indices.
    pub fn visible_range(&self) -> Range<usize> {
        if self.total_items == 0 || self.item_height == 0.0 {
            return 0..0;
        }

        let first_visible = (self.scroll_offset / self.item_height).floor() as usize;
        let visible_count = (self.viewport_height / self.item_height).ceil() as usize;

        let start = first_visible.saturating_sub(self.buffer_count);
        let end = (first_visible + visible_count + self.buffer_count).min(self.total_items);

        start..end
    }

    /// Get the total scrollable height.
    pub fn total_height(&self) -> f32 {
        self.total_items as f32 * self.item_height
    }

    /// Get the offset for item at given index.
    pub fn item_offset(&self, index: usize) -> f32 {
        index as f32 * self.item_height
    }

    /// Scroll to a specific item.
    pub fn scroll_to_item(&mut self, index: usize) {
        let target_offset = self.item_offset(index);
        let max_offset = (self.total_height() - self.viewport_height).max(0.0);
        self.scroll_offset = target_offset.min(max_offset);
    }

    /// Scroll by delta pixels.
    pub fn scroll_by(&mut self, delta: f32) {
        let max_offset = (self.total_height() - self.viewport_height).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_offset);
    }
}

/// A basic list item component.
pub struct ListItem {
    id: ElementId,
    label: SharedString,
    secondary: Option<SharedString>,
    selected: bool,
    highlighted: bool,
}

impl ListItem {
    /// Create a new list item.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            secondary: None,
            selected: false,
            highlighted: false,
        }
    }

    /// Set secondary text.
    pub fn secondary(mut self, text: impl Into<SharedString>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Set selected state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set highlighted (hover) state.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }
}

impl RenderOnce for ListItem {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        let bg = if self.selected {
            colors.surface_elevated
        } else if self.highlighted {
            colors.surface
        } else {
            gpui::Hsla::transparent_black()
        };

        let hover_bg = colors.surface_elevated;

        let mut element = div()
            .id(self.id)
            .px(px(12.0))
            .py(px(8.0))
            .w_full()
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
                    .text_color(colors.text_primary)
                    .child(self.label),
            );

        if let Some(secondary) = self.secondary {
            element = element.child(
                div()
                    .text_color(colors.text_secondary)
                    .text_sm()
                    .child(secondary),
            );
        }

        element
    }
}

/// A divider between list items.
pub struct ListDivider;

impl RenderOnce for ListDivider {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        div()
            .h(px(1.0))
            .w_full()
            .bg(colors.border)
    }
}

/// A section header in a list.
pub struct ListHeader {
    label: SharedString,
}

impl ListHeader {
    /// Create a new list header.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl RenderOnce for ListHeader {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        div()
            .px(px(12.0))
            .py(px(8.0))
            .w_full()
            .bg(colors.surface)
            .text_color(colors.text_muted)
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(self.label)
    }
}

/// Empty state for when a list has no items.
pub struct EmptyState {
    title: SharedString,
    description: Option<SharedString>,
}

impl EmptyState {
    /// Create a new empty state.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
        }
    }

    /// Set the description.
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        let mut element = div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(32.0))
            .gap(px(8.0))
            .child(
                div()
                    .text_color(colors.text_primary)
                    .child(self.title),
            );

        if let Some(description) = self.description {
            element = element.child(
                div()
                    .text_color(colors.text_secondary)
                    .text_sm()
                    .child(description),
            );
        }

        element
    }
}

/// Loading state for a list.
pub struct LoadingState {
    message: SharedString,
}

impl LoadingState {
    /// Create a new loading state.
    pub fn new() -> Self {
        Self {
            message: SharedString::from("Loading..."),
        }
    }

    /// Set the loading message.
    pub fn message(mut self, text: impl Into<SharedString>) -> Self {
        self.message = text.into();
        self
    }
}

impl Default for LoadingState {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for LoadingState {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        div()
            .flex()
            .items_center()
            .justify_center()
            .p(px(32.0))
            .text_color(colors.text_muted)
            .child(self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtualized_list_visible_range() {
        let state = VirtualizedListState {
            total_items: 100,
            item_height: 50.0,
            scroll_offset: 0.0,
            viewport_height: 500.0,
            buffer_count: 2,
        };

        let range = state.visible_range();
        assert_eq!(range.start, 0);
        assert_eq!(range.end, 12); // 10 visible + 2 buffer
    }

    #[test]
    fn virtualized_list_visible_range_scrolled() {
        let state = VirtualizedListState {
            total_items: 100,
            item_height: 50.0,
            scroll_offset: 250.0, // Scrolled 5 items down
            viewport_height: 500.0,
            buffer_count: 2,
        };

        let range = state.visible_range();
        assert_eq!(range.start, 3); // 5 - 2 buffer
        assert_eq!(range.end, 17); // 5 + 10 + 2 buffer
    }

    #[test]
    fn virtualized_list_empty() {
        let state = VirtualizedListState::new(0);
        let range = state.visible_range();
        assert_eq!(range.start, 0);
        assert_eq!(range.end, 0);
    }

    #[test]
    fn virtualized_list_scroll_to_item() {
        let mut state = VirtualizedListState {
            total_items: 100,
            item_height: 50.0,
            scroll_offset: 0.0,
            viewport_height: 500.0,
            buffer_count: 2,
        };

        state.scroll_to_item(20);
        assert_eq!(state.scroll_offset, 1000.0);

        state.scroll_to_item(95);
        // Should cap at max scroll
        assert!(state.scroll_offset <= state.total_height() - state.viewport_height);
    }

    #[test]
    fn virtualized_list_scroll_by() {
        let mut state = VirtualizedListState {
            total_items: 100,
            item_height: 50.0,
            scroll_offset: 100.0,
            viewport_height: 500.0,
            buffer_count: 2,
        };

        state.scroll_by(50.0);
        assert_eq!(state.scroll_offset, 150.0);

        state.scroll_by(-200.0);
        assert_eq!(state.scroll_offset, 0.0); // Clamped to 0
    }

    #[test]
    fn list_item_builder() {
        let item = ListItem::new("item-1", "Primary text")
            .secondary("Secondary text")
            .selected(true)
            .highlighted(false);

        assert_eq!(item.label.as_ref(), "Primary text");
        assert!(item.secondary.is_some());
        assert!(item.selected);
        assert!(!item.highlighted);
    }

    #[test]
    fn list_header_new() {
        let header = ListHeader::new("Section");
        assert_eq!(header.label.as_ref(), "Section");
    }

    #[test]
    fn empty_state_builder() {
        let empty = EmptyState::new("No items")
            .description("Try adding some items");

        assert_eq!(empty.title.as_ref(), "No items");
        assert!(empty.description.is_some());
    }

    #[test]
    fn loading_state_builder() {
        let loading = LoadingState::new().message("Fetching data...");
        assert_eq!(loading.message.as_ref(), "Fetching data...");
    }
}
