//! Button component.
//!
//! Provides styled button variants for different use cases.

use gpui::{
    div, px, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled,
};

use crate::ui::theme::ThemeColors;

/// Type alias for button click handlers.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>;

/// Button variant styles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Primary action button.
    #[default]
    Primary,
    /// Secondary action button.
    Secondary,
    /// Destructive action button.
    Danger,
    /// Ghost/transparent button.
    Ghost,
}

/// Button size options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// Small button (28px height).
    Small,
    /// Medium button (32px height).
    #[default]
    Medium,
    /// Large button (40px height).
    Large,
}

/// A styled button component.
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    full_width: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// Create a new button with the given label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Primary,
            size: ButtonSize::Medium,
            disabled: false,
            full_width: false,
            on_click: None,
        }
    }

    /// Set the button variant.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the button size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Make the button full width.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Set the click handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn height(&self) -> f32 {
        match self.size {
            ButtonSize::Small => 28.0,
            ButtonSize::Medium => 32.0,
            ButtonSize::Large => 40.0,
        }
    }

    fn padding_x(&self) -> f32 {
        match self.size {
            ButtonSize::Small => 12.0,
            ButtonSize::Medium => 16.0,
            ButtonSize::Large => 20.0,
        }
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();

        let (bg, text_color, hover_bg) = match self.variant {
            ButtonVariant::Primary => (colors.accent, colors.text_primary, colors.accent_hover),
            ButtonVariant::Secondary => (colors.surface_elevated, colors.text_primary, colors.border),
            ButtonVariant::Danger => (colors.error, colors.text_primary, colors.error),
            ButtonVariant::Ghost => (
                gpui::Hsla::transparent_black(),
                colors.text_primary,
                colors.surface_elevated,
            ),
        };

        let opacity = if self.disabled { 0.5 } else { 1.0 };
        let height = self.height();
        let padding_x = self.padding_x();

        let mut element = div()
            .id(self.id)
            .h(px(height))
            .px(px(padding_x))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .bg(bg)
            .text_color(text_color)
            .opacity(opacity)
            .cursor_pointer()
            .child(self.label);

        if self.full_width {
            element = element.w_full();
        }

        if !self.disabled {
            element = element.hover(move |style| style.bg(hover_bg));

            if let Some(handler) = self.on_click {
                element = element.on_click(handler);
            }
        }

        element
    }
}

/// An icon button (square, typically for toolbar actions).
pub struct IconButton {
    id: ElementId,
    icon: SharedString,
    tooltip: Option<SharedString>,
    size: ButtonSize,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl IconButton {
    /// Create a new icon button.
    pub fn new(id: impl Into<ElementId>, icon: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            tooltip: None,
            size: ButtonSize::Medium,
            disabled: false,
            on_click: None,
        }
    }

    /// Set a tooltip for the button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set the button size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the click handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn size_px(&self) -> f32 {
        match self.size {
            ButtonSize::Small => 24.0,
            ButtonSize::Medium => 28.0,
            ButtonSize::Large => 32.0,
        }
    }
}

impl RenderOnce for IconButton {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();
        let size = self.size_px();
        let opacity = if self.disabled { 0.5 } else { 1.0 };

        let mut element = div()
            .id(self.id)
            .size(px(size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .text_color(colors.text_secondary)
            .opacity(opacity)
            .cursor_pointer()
            .child(self.icon);

        if !self.disabled {
            element = element.hover(move |style| {
                style.bg(colors.surface_elevated).text_color(colors.text_primary)
            });

            if let Some(handler) = self.on_click {
                element = element.on_click(handler);
            }
        }

        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_builder() {
        let button = Button::new("test", "Click me")
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Large)
            .disabled(false)
            .full_width();

        assert_eq!(button.variant, ButtonVariant::Primary);
        assert_eq!(button.size, ButtonSize::Large);
        assert!(!button.disabled);
        assert!(button.full_width);
    }

    #[test]
    fn button_sizes() {
        let small = Button::new("small", "Small").size(ButtonSize::Small);
        let medium = Button::new("medium", "Medium").size(ButtonSize::Medium);
        let large = Button::new("large", "Large").size(ButtonSize::Large);

        assert_eq!(small.height(), 28.0);
        assert_eq!(medium.height(), 32.0);
        assert_eq!(large.height(), 40.0);
    }

    #[test]
    fn icon_button_builder() {
        let button = IconButton::new("icon", "X")
            .tooltip("Close")
            .size(ButtonSize::Small)
            .disabled(true);

        assert!(button.tooltip.is_some());
        assert_eq!(button.size, ButtonSize::Small);
        assert!(button.disabled);
    }
}
