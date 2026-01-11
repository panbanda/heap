//! Settings view.
//!
//! Application settings panel with sections for appearance, accounts, AI, etc.

use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};

use crate::ui::theme::{ThemeColors, ThemeMode};

/// Settings view component.
pub struct SettingsView {
    colors: ThemeColors,
    active_section: SettingsSection,
    theme_mode: ThemeMode,
    font_size: u8,
    ai_enabled: bool,
    ai_provider: String,
    notifications_enabled: bool,
}

/// Settings sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Accounts,
    Appearance,
    Ai,
    Notifications,
    Privacy,
    Keyboard,
}

impl SettingsSection {
    fn label(&self) -> &str {
        match self {
            Self::General => "General",
            Self::Accounts => "Accounts",
            Self::Appearance => "Appearance",
            Self::Ai => "AI",
            Self::Notifications => "Notifications",
            Self::Privacy => "Privacy",
            Self::Keyboard => "Keyboard Shortcuts",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::General,
            Self::Accounts,
            Self::Appearance,
            Self::Ai,
            Self::Notifications,
            Self::Privacy,
            Self::Keyboard,
        ]
    }
}

impl SettingsView {
    /// Create a new settings view.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            colors: ThemeColors::dark(),
            active_section: SettingsSection::General,
            theme_mode: ThemeMode::Dark,
            font_size: 14,
            ai_enabled: true,
            ai_provider: "anthropic".to_string(),
            notifications_enabled: true,
        }
    }

    /// Set the active section.
    pub fn set_section(&mut self, section: SettingsSection) {
        self.active_section = section;
    }

    fn render_sidebar(&self) -> impl IntoElement {
        let active = self.active_section;
        let surface = self.colors.surface;
        let surface_elevated = self.colors.surface_elevated;
        let border = self.colors.border;
        let text_primary = self.colors.text_primary;

        div()
            .w(px(200.0))
            .h_full()
            .bg(surface)
            .border_r_1()
            .border_color(border)
            .p(px(12.0))
            .children(SettingsSection::all().into_iter().map(|section| {
                let is_active = section == active;
                let bg = if is_active {
                    surface_elevated
                } else {
                    gpui::Hsla::transparent_black()
                };

                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(6.0))
                    .bg(bg)
                    .cursor_pointer()
                    .hover(move |style| style.bg(surface_elevated))
                    .text_color(text_primary)
                    .child(SharedString::from(section.label().to_string()))
            }))
    }

    fn render_section_header(&self, title: &str, description: &str) -> impl IntoElement {
        div()
            .mb(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.colors.text_primary)
                    .mb(px(4.0))
                    .child(SharedString::from(title.to_string())),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(self.colors.text_secondary)
                    .child(SharedString::from(description.to_string())),
            )
    }

    fn render_toggle(&self, label: &str, description: &str, enabled: bool) -> impl IntoElement {
        let surface = self.colors.surface;
        let accent = self.colors.accent;
        let text_primary = self.colors.text_primary;
        let text_secondary = self.colors.text_secondary;

        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(12.0))
            .border_b_1()
            .border_color(self.colors.border)
            .child(
                div()
                    .child(
                        div()
                            .text_color(text_primary)
                            .child(SharedString::from(label.to_string())),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_secondary)
                            .child(SharedString::from(description.to_string())),
                    ),
            )
            .child(
                div()
                    .w(px(44.0))
                    .h(px(24.0))
                    .rounded_full()
                    .bg(if enabled { accent } else { surface })
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(20.0))
                            .rounded_full()
                            .bg(text_primary)
                            .mt(px(2.0))
                            .ml(if enabled { px(22.0) } else { px(2.0) }),
                    ),
            )
    }

    fn render_select(
        &self,
        label: &str,
        description: &str,
        value: &str,
        _options: &[&str],
    ) -> impl IntoElement {
        let surface = self.colors.surface;
        let border = self.colors.border;
        let text_primary = self.colors.text_primary;
        let text_secondary = self.colors.text_secondary;

        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(12.0))
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .child(
                        div()
                            .text_color(text_primary)
                            .child(SharedString::from(label.to_string())),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_secondary)
                            .child(SharedString::from(description.to_string())),
                    ),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .cursor_pointer()
                    .text_color(text_primary)
                    .child(SharedString::from(value.to_string())),
            )
    }

    fn render_general_section(&self) -> impl IntoElement {
        div()
            .child(self.render_section_header(
                "General Settings",
                "Configure general application behavior",
            ))
            .child(self.render_toggle(
                "Check for updates",
                "Automatically check for new versions",
                true,
            ))
            .child(self.render_toggle(
                "Start at login",
                "Launch margin when you log in",
                false,
            ))
    }

    fn render_appearance_section(&self) -> impl IntoElement {
        let theme_value = match self.theme_mode {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
        };

        div()
            .child(self.render_section_header(
                "Appearance",
                "Customize how margin looks",
            ))
            .child(self.render_select(
                "Theme",
                "Choose your preferred color scheme",
                theme_value,
                &["Dark", "Light", "System"],
            ))
            .child(self.render_select(
                "Font Size",
                "Adjust the base font size",
                &format!("{}px", self.font_size),
                &["12px", "14px", "16px", "18px"],
            ))
            .child(self.render_select(
                "Density",
                "Control spacing between elements",
                "Default",
                &["Compact", "Default", "Relaxed"],
            ))
    }

    fn render_ai_section(&self) -> impl IntoElement {
        div()
            .child(self.render_section_header(
                "AI Settings",
                "Configure AI-powered features",
            ))
            .child(self.render_toggle(
                "Enable AI features",
                "Use AI for summaries, drafts, and search",
                self.ai_enabled,
            ))
            .child(self.render_select(
                "AI Provider",
                "Choose your AI service provider",
                &self.ai_provider,
                &["anthropic", "openai", "ollama"],
            ))
            .child(self.render_toggle(
                "Auto-summarize threads",
                "Automatically generate summaries for long threads",
                false,
            ))
            .child(self.render_toggle(
                "Learn from sent emails",
                "Improve drafts based on your writing style",
                true,
            ))
    }

    fn render_notifications_section(&self) -> impl IntoElement {
        div()
            .child(self.render_section_header(
                "Notifications",
                "Control how you receive notifications",
            ))
            .child(self.render_toggle(
                "Enable notifications",
                "Show desktop notifications for new emails",
                self.notifications_enabled,
            ))
            .child(self.render_select(
                "New email alerts",
                "Which emails trigger notifications",
                "VIP Only",
                &["All", "VIP Only", "None"],
            ))
            .child(self.render_toggle(
                "Sound effects",
                "Play sounds for notifications",
                false,
            ))
    }

    fn render_content(&self) -> impl IntoElement {
        div()
            .flex_1()
            .p(px(24.0))
            .overflow_y_hidden()
            .when(self.active_section == SettingsSection::General, |this| {
                this.child(self.render_general_section())
            })
            .when(
                self.active_section == SettingsSection::Appearance,
                |this| this.child(self.render_appearance_section()),
            )
            .when(self.active_section == SettingsSection::Ai, |this| {
                this.child(self.render_ai_section())
            })
            .when(
                self.active_section == SettingsSection::Notifications,
                |this| this.child(self.render_notifications_section()),
            )
            .when(
                !matches!(
                    self.active_section,
                    SettingsSection::General
                        | SettingsSection::Appearance
                        | SettingsSection::Ai
                        | SettingsSection::Notifications
                ),
                |this| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h_full()
                            .text_color(self.colors.text_muted)
                            .child(SharedString::from("Coming soon...")),
                    )
                },
            )
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("settings")
            .size_full()
            .flex()
            .bg(self.colors.background)
            .child(self.render_sidebar())
            .child(self.render_content())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_sections() {
        let sections = SettingsSection::all();
        assert_eq!(sections.len(), 7);
        assert_eq!(sections[0], SettingsSection::General);
    }

    #[test]
    fn section_labels() {
        assert_eq!(SettingsSection::General.label(), "General");
        assert_eq!(SettingsSection::Ai.label(), "AI");
        assert_eq!(SettingsSection::Keyboard.label(), "Keyboard Shortcuts");
    }

    #[test]
    fn set_section() {
        let mut view = SettingsView {
            colors: ThemeColors::dark(),
            active_section: SettingsSection::General,
            theme_mode: ThemeMode::Dark,
            font_size: 14,
            ai_enabled: true,
            ai_provider: "anthropic".to_string(),
            notifications_enabled: true,
        };

        assert_eq!(view.active_section, SettingsSection::General);

        view.set_section(SettingsSection::Ai);
        assert_eq!(view.active_section, SettingsSection::Ai);
    }
}
