//! Input components.
//!
//! Provides text input fields with various styles and states.

use gpui::{
    div, px, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled,
};

use crate::ui::theme::ThemeColors;

/// Input size options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputSize {
    /// Small input (28px height).
    Small,
    /// Medium input (32px height).
    #[default]
    Medium,
    /// Large input (40px height).
    Large,
}

/// A text input field.
pub struct TextInput {
    id: ElementId,
    value: SharedString,
    placeholder: Option<SharedString>,
    size: InputSize,
    disabled: bool,
    error: bool,
    focus_handle: Option<FocusHandle>,
}

impl TextInput {
    /// Create a new text input.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: SharedString::default(),
            placeholder: None,
            size: InputSize::Medium,
            disabled: false,
            error: false,
            focus_handle: None,
        }
    }

    /// Set the current value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the input size.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show error state.
    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    /// Set the focus handle.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    fn height(&self) -> f32 {
        match self.size {
            InputSize::Small => 28.0,
            InputSize::Medium => 32.0,
            InputSize::Large => 40.0,
        }
    }
}

impl RenderOnce for TextInput {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        let border_color = if self.error {
            colors.error
        } else {
            colors.border
        };

        let opacity = if self.disabled { 0.5 } else { 1.0 };
        let height = self.height();

        let is_empty = self.value.is_empty();
        let display_text = if is_empty {
            self.placeholder.unwrap_or_default()
        } else {
            self.value
        };

        let text_color = if is_empty {
            colors.text_muted
        } else {
            colors.text_primary
        };

        div()
            .id(self.id)
            .h(px(height))
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .bg(colors.surface)
            .border_1()
            .border_color(border_color)
            .rounded(px(6.0))
            .text_color(text_color)
            .opacity(opacity)
            .cursor_text()
            .child(display_text)
    }
}

/// A search input with icon.
pub struct SearchInput {
    id: ElementId,
    value: SharedString,
    placeholder: SharedString,
    size: InputSize,
}

impl SearchInput {
    /// Create a new search input.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: SharedString::default(),
            placeholder: SharedString::from("Search..."),
            size: InputSize::Medium,
        }
    }

    /// Set the current value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the input size.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    fn height(&self) -> f32 {
        match self.size {
            InputSize::Small => 28.0,
            InputSize::Medium => 32.0,
            InputSize::Large => 40.0,
        }
    }
}

impl RenderOnce for SearchInput {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();
        let height = self.height();

        let is_empty = self.value.is_empty();
        let display_text = if is_empty {
            self.placeholder
        } else {
            self.value
        };

        let text_color = if is_empty {
            colors.text_muted
        } else {
            colors.text_primary
        };

        div()
            .id(self.id)
            .h(px(height))
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .rounded(px(6.0))
            .cursor_text()
            .child(
                div()
                    .text_color(colors.text_muted)
                    .child(SharedString::from("/")),
            )
            .child(div().flex_1().text_color(text_color).child(display_text))
    }
}

/// A text area (multiline input).
pub struct TextArea {
    id: ElementId,
    value: SharedString,
    placeholder: Option<SharedString>,
    rows: u32,
    disabled: bool,
}

impl TextArea {
    /// Create a new text area.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: SharedString::default(),
            placeholder: None,
            rows: 4,
            disabled: false,
        }
    }

    /// Set the current value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the number of visible rows.
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows;
        self
    }

    /// Disable the text area.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for TextArea {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();
        let line_height = 20.0;
        let height = (self.rows as f32 * line_height) + 16.0;

        let opacity = if self.disabled { 0.5 } else { 1.0 };

        let display_text = if self.value.is_empty() {
            self.placeholder.unwrap_or_default()
        } else {
            self.value.clone()
        };

        let text_color = if self.value.is_empty() {
            colors.text_muted
        } else {
            colors.text_primary
        };

        div()
            .id(self.id)
            .min_h(px(height))
            .w_full()
            .p(px(12.0))
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .rounded(px(6.0))
            .text_color(text_color)
            .opacity(opacity)
            .cursor_text()
            .child(display_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_builder() {
        let input = TextInput::new("test")
            .value("Hello")
            .placeholder("Enter text")
            .size(InputSize::Large)
            .disabled(true)
            .error(false);

        assert_eq!(input.value.as_ref(), "Hello");
        assert!(input.placeholder.is_some());
        assert_eq!(input.size, InputSize::Large);
        assert!(input.disabled);
        assert!(!input.error);
    }

    #[test]
    fn input_sizes() {
        let small = TextInput::new("small").size(InputSize::Small);
        let medium = TextInput::new("medium").size(InputSize::Medium);
        let large = TextInput::new("large").size(InputSize::Large);

        assert_eq!(small.height(), 28.0);
        assert_eq!(medium.height(), 32.0);
        assert_eq!(large.height(), 40.0);
    }

    #[test]
    fn search_input_builder() {
        let search = SearchInput::new("search")
            .value("query")
            .placeholder("Search emails...");

        assert_eq!(search.value.as_ref(), "query");
        assert_eq!(search.placeholder.as_ref(), "Search emails...");
    }

    #[test]
    fn text_area_builder() {
        let area = TextArea::new("area")
            .value("Content")
            .placeholder("Enter message")
            .rows(6)
            .disabled(false);

        assert_eq!(area.value.as_ref(), "Content");
        assert_eq!(area.rows, 6);
        assert!(!area.disabled);
    }
}
