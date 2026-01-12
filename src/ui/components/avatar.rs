//! Avatar component.
//!
//! Displays a user avatar with optional image or initials fallback.

use gpui::{
    div, px, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled,
};

use crate::ui::theme::ThemeColors;

/// Avatar size options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AvatarSize {
    /// Extra small (20px).
    XSmall,
    /// Small (24px).
    Small,
    /// Medium (32px).
    #[default]
    Medium,
    /// Large (40px).
    Large,
    /// Extra large (48px).
    XLarge,
}

impl AvatarSize {
    fn px(self) -> f32 {
        match self {
            AvatarSize::XSmall => 20.0,
            AvatarSize::Small => 24.0,
            AvatarSize::Medium => 32.0,
            AvatarSize::Large => 40.0,
            AvatarSize::XLarge => 48.0,
        }
    }

    fn font_size(self) -> f32 {
        match self {
            AvatarSize::XSmall => 10.0,
            AvatarSize::Small => 11.0,
            AvatarSize::Medium => 13.0,
            AvatarSize::Large => 16.0,
            AvatarSize::XLarge => 18.0,
        }
    }
}

/// Avatar shape options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AvatarShape {
    /// Circular avatar.
    #[default]
    Circle,
    /// Rounded square avatar.
    Square,
}

/// A user avatar component.
pub struct Avatar {
    id: ElementId,
    initials: SharedString,
    size: AvatarSize,
    shape: AvatarShape,
    color_seed: u32,
}

impl Avatar {
    /// Create a new avatar with initials.
    pub fn new(id: impl Into<ElementId>, initials: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            initials: initials.into(),
            size: AvatarSize::Medium,
            shape: AvatarShape::Circle,
            color_seed: 0,
        }
    }

    /// Create avatar from a full name, extracting initials.
    pub fn from_name(id: impl Into<ElementId>, name: &str) -> Self {
        let initials = extract_initials(name);
        let color_seed = name.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        Self {
            id: id.into(),
            initials: initials.into(),
            size: AvatarSize::Medium,
            shape: AvatarShape::Circle,
            color_seed,
        }
    }

    /// Set the avatar size.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Set the avatar shape.
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set a seed for consistent background color generation.
    pub fn color_seed(mut self, seed: u32) -> Self {
        self.color_seed = seed;
        self
    }
}

fn extract_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.len() {
        0 => "?".to_string(),
        1 => parts[0]
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default(),
        _ => {
            let first = parts[0].chars().next().unwrap_or('?');
            let last = parts[parts.len() - 1].chars().next().unwrap_or('?');
            format!("{}{}", first.to_uppercase(), last.to_uppercase())
        }
    }
}

fn generate_color(seed: u32) -> gpui::Hsla {
    // Generate a consistent hue based on the seed
    let hue = ((seed * 137) % 360) as f32;
    gpui::hsla(hue / 360.0, 0.5, 0.35, 1.0)
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();
        let size = self.size.px();
        let font_size = self.size.font_size();
        let bg_color = generate_color(self.color_seed);

        let radius = match self.shape {
            AvatarShape::Circle => size / 2.0,
            AvatarShape::Square => 4.0,
        };

        div()
            .id(self.id)
            .size(px(size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius))
            .bg(bg_color)
            .text_color(colors.text_primary)
            .text_size(px(font_size))
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(self.initials)
    }
}

/// A group of overlapping avatars.
pub struct AvatarGroup {
    id: ElementId,
    avatars: Vec<(SharedString, u32)>,
    size: AvatarSize,
    max_visible: usize,
}

impl AvatarGroup {
    /// Create a new avatar group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            avatars: Vec::new(),
            size: AvatarSize::Small,
            max_visible: 3,
        }
    }

    /// Add an avatar by name.
    pub fn with_avatar(mut self, name: &str) -> Self {
        let initials = extract_initials(name);
        let seed = name.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        self.avatars.push((initials.into(), seed));
        self
    }

    /// Set the size for all avatars.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Set maximum visible avatars before showing overflow count.
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let colors = ThemeColors::dark();
        let size = self.size.px();
        let font_size = self.size.font_size();
        let overlap = size * 0.3;

        let visible_count = self.avatars.len().min(self.max_visible);
        let overflow_count = self.avatars.len().saturating_sub(self.max_visible);

        let mut container = div().id(self.id).flex().items_center();

        for (i, (initials, seed)) in self.avatars.iter().take(visible_count).enumerate() {
            let bg_color = generate_color(*seed);
            let margin = if i > 0 { -overlap } else { 0.0 };

            container = container.child(
                div()
                    .size(px(size))
                    .ml(px(margin))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(bg_color)
                    .border_2()
                    .border_color(colors.background)
                    .text_color(colors.text_primary)
                    .text_size(px(font_size))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(initials.clone()),
            );
        }

        if overflow_count > 0 {
            container = container.child(
                div()
                    .size(px(size))
                    .ml(px(-overlap))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(colors.surface_elevated)
                    .border_2()
                    .border_color(colors.background)
                    .text_color(colors.text_secondary)
                    .text_size(px(font_size * 0.9))
                    .child(format!("+{}", overflow_count)),
            );
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_initials_single_name() {
        assert_eq!(extract_initials("Alice"), "A");
    }

    #[test]
    fn extract_initials_full_name() {
        assert_eq!(extract_initials("Alice Bob"), "AB");
    }

    #[test]
    fn extract_initials_multiple_names() {
        assert_eq!(extract_initials("Alice Marie Bob"), "AB");
    }

    #[test]
    fn extract_initials_empty() {
        assert_eq!(extract_initials(""), "?");
    }

    #[test]
    fn avatar_builder() {
        let avatar = Avatar::new("test", "AB")
            .size(AvatarSize::Large)
            .shape(AvatarShape::Square)
            .color_seed(42);

        assert_eq!(avatar.size, AvatarSize::Large);
        assert_eq!(avatar.shape, AvatarShape::Square);
        assert_eq!(avatar.color_seed, 42);
    }

    #[test]
    fn avatar_sizes() {
        assert_eq!(AvatarSize::XSmall.px(), 20.0);
        assert_eq!(AvatarSize::Small.px(), 24.0);
        assert_eq!(AvatarSize::Medium.px(), 32.0);
        assert_eq!(AvatarSize::Large.px(), 40.0);
        assert_eq!(AvatarSize::XLarge.px(), 48.0);
    }

    #[test]
    fn avatar_group_builder() {
        let group = AvatarGroup::new("group")
            .with_avatar("Alice")
            .with_avatar("Bob")
            .with_avatar("Charlie")
            .size(AvatarSize::Medium)
            .max_visible(2);

        assert_eq!(group.avatars.len(), 3);
        assert_eq!(group.max_visible, 2);
    }

    #[test]
    fn color_generation_consistent() {
        let color1 = generate_color(42);
        let color2 = generate_color(42);
        assert_eq!(color1.h, color2.h);
    }
}
