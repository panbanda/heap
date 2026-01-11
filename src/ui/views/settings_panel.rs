//! Settings panel component.
//!
//! Provides a tabbed interface for configuring:
//! - Account settings
//! - AI features
//! - Keyboard shortcuts
//! - Theme and appearance
//! - Sync preferences

use gpui::{
    div, prelude::*, px, rgba, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

/// Settings tab categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    /// General/account settings.
    #[default]
    General,
    /// AI feature settings.
    Ai,
    /// Keyboard shortcut settings.
    Keybindings,
    /// Theme and appearance.
    Appearance,
    /// Sync and offline settings.
    Sync,
    /// Privacy and security.
    Privacy,
}

impl SettingsTab {
    /// Returns the display name for this tab.
    pub fn name(&self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Ai => "AI Features",
            SettingsTab::Keybindings => "Keyboard Shortcuts",
            SettingsTab::Appearance => "Appearance",
            SettingsTab::Sync => "Sync & Offline",
            SettingsTab::Privacy => "Privacy",
        }
    }

    /// Returns the icon for this tab.
    pub fn icon(&self) -> &'static str {
        match self {
            SettingsTab::General => "settings",
            SettingsTab::Ai => "sparkles",
            SettingsTab::Keybindings => "keyboard",
            SettingsTab::Appearance => "palette",
            SettingsTab::Sync => "refresh",
            SettingsTab::Privacy => "shield",
        }
    }

    /// Returns all tabs in order.
    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::General,
            SettingsTab::Ai,
            SettingsTab::Keybindings,
            SettingsTab::Appearance,
            SettingsTab::Sync,
            SettingsTab::Privacy,
        ]
    }
}

/// A toggle setting.
#[derive(Debug, Clone)]
pub struct ToggleSetting {
    /// Setting key.
    pub key: String,
    /// Display label.
    pub label: SharedString,
    /// Description text.
    pub description: Option<SharedString>,
    /// Current value.
    pub enabled: bool,
}

impl ToggleSetting {
    /// Creates a new toggle setting.
    pub fn new(key: impl Into<String>, label: impl Into<SharedString>, enabled: bool) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            description: None,
            enabled,
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A select/dropdown setting.
#[derive(Debug, Clone)]
pub struct SelectSetting {
    /// Setting key.
    pub key: String,
    /// Display label.
    pub label: SharedString,
    /// Available options.
    pub options: Vec<SelectOption>,
    /// Currently selected value.
    pub selected: String,
}

impl SelectSetting {
    /// Creates a new select setting.
    pub fn new(
        key: impl Into<String>,
        label: impl Into<SharedString>,
        options: Vec<SelectOption>,
        selected: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            options,
            selected: selected.into(),
        }
    }
}

/// An option in a select setting.
#[derive(Debug, Clone)]
pub struct SelectOption {
    /// Option value.
    pub value: String,
    /// Display label.
    pub label: SharedString,
}

impl SelectOption {
    /// Creates a new select option.
    pub fn new(value: impl Into<String>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// The settings panel view component.
pub struct SettingsPanel {
    /// Whether the panel is visible.
    visible: bool,
    /// Currently selected tab.
    current_tab: SettingsTab,
    /// Toggle settings by category.
    toggles: Vec<(SettingsTab, Vec<ToggleSetting>)>,
    /// Select settings by category.
    selects: Vec<(SettingsTab, Vec<SelectSetting>)>,
    /// Whether there are unsaved changes.
    has_changes: bool,
}

impl SettingsPanel {
    /// Creates a new settings panel.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            visible: false,
            current_tab: SettingsTab::General,
            toggles: Self::default_toggles(),
            selects: Self::default_selects(),
            has_changes: false,
        }
    }

    /// Returns the default toggle settings.
    fn default_toggles() -> Vec<(SettingsTab, Vec<ToggleSetting>)> {
        vec![
            (
                SettingsTab::General,
                vec![
                    ToggleSetting::new("notifications", "Desktop Notifications", true)
                        .with_description("Show desktop notifications for new emails"),
                    ToggleSetting::new("sounds", "Sound Effects", false)
                        .with_description("Play sounds for notifications and actions"),
                    ToggleSetting::new("confirm_send", "Confirm Before Sending", true)
                        .with_description("Show confirmation dialog before sending emails"),
                    ToggleSetting::new("undo_send", "Undo Send", true)
                        .with_description("Allow cancelling sent emails for 10 seconds"),
                ],
            ),
            (
                SettingsTab::Ai,
                vec![
                    ToggleSetting::new("ai_enabled", "Enable AI Features", true)
                        .with_description("Use AI for summaries, drafts, and search"),
                    ToggleSetting::new("auto_summarize", "Auto-Summarize Threads", true)
                        .with_description("Automatically generate summaries for long threads"),
                    ToggleSetting::new("smart_compose", "Smart Compose", true)
                        .with_description("AI-powered writing suggestions"),
                    ToggleSetting::new("semantic_search", "Semantic Search", true)
                        .with_description("Use AI to find conceptually similar emails"),
                    ToggleSetting::new("local_models", "Prefer Local Models", false)
                        .with_description("Use on-device models when possible for privacy"),
                ],
            ),
            (
                SettingsTab::Sync,
                vec![
                    ToggleSetting::new("auto_sync", "Auto-Sync", true)
                        .with_description("Automatically sync emails in the background"),
                    ToggleSetting::new("sync_attachments", "Sync Attachments", true)
                        .with_description("Download attachments for offline access"),
                    ToggleSetting::new("offline_mode", "Offline Mode", false)
                        .with_description("Disable all network requests"),
                ],
            ),
            (
                SettingsTab::Privacy,
                vec![
                    ToggleSetting::new("block_tracking", "Block Tracking Pixels", true)
                        .with_description("Prevent senders from knowing when you read emails"),
                    ToggleSetting::new("external_images", "Load External Images", false)
                        .with_description("Automatically load images from external servers"),
                    ToggleSetting::new("telemetry", "Anonymous Telemetry", true)
                        .with_description("Help improve margin by sending anonymous usage data"),
                ],
            ),
        ]
    }

    /// Returns the default select settings.
    fn default_selects() -> Vec<(SettingsTab, Vec<SelectSetting>)> {
        vec![
            (
                SettingsTab::General,
                vec![SelectSetting::new(
                    "default_view",
                    "Default View",
                    vec![
                        SelectOption::new("inbox", "Inbox"),
                        SelectOption::new("all", "All Mail"),
                        SelectOption::new("starred", "Starred"),
                    ],
                    "inbox",
                )],
            ),
            (
                SettingsTab::Ai,
                vec![
                    SelectSetting::new(
                        "ai_provider",
                        "AI Provider",
                        vec![
                            SelectOption::new("anthropic", "Anthropic Claude"),
                            SelectOption::new("openai", "OpenAI GPT-4"),
                            SelectOption::new("local", "Local (Candle)"),
                        ],
                        "anthropic",
                    ),
                    SelectSetting::new(
                        "compose_tone",
                        "Default Compose Tone",
                        vec![
                            SelectOption::new("casual", "Casual"),
                            SelectOption::new("formal", "Formal"),
                            SelectOption::new("brief", "Brief"),
                            SelectOption::new("detailed", "Detailed"),
                        ],
                        "casual",
                    ),
                ],
            ),
            (
                SettingsTab::Appearance,
                vec![
                    SelectSetting::new(
                        "theme",
                        "Theme",
                        vec![
                            SelectOption::new("dark", "Dark"),
                            SelectOption::new("light", "Light"),
                            SelectOption::new("system", "System"),
                        ],
                        "dark",
                    ),
                    SelectSetting::new(
                        "font_size",
                        "Font Size",
                        vec![
                            SelectOption::new("small", "Small"),
                            SelectOption::new("medium", "Medium"),
                            SelectOption::new("large", "Large"),
                        ],
                        "medium",
                    ),
                    SelectSetting::new(
                        "density",
                        "Display Density",
                        vec![
                            SelectOption::new("compact", "Compact"),
                            SelectOption::new("comfortable", "Comfortable"),
                            SelectOption::new("spacious", "Spacious"),
                        ],
                        "comfortable",
                    ),
                ],
            ),
            (
                SettingsTab::Sync,
                vec![SelectSetting::new(
                    "sync_interval",
                    "Sync Interval",
                    vec![
                        SelectOption::new("1", "1 minute"),
                        SelectOption::new("5", "5 minutes"),
                        SelectOption::new("15", "15 minutes"),
                        SelectOption::new("30", "30 minutes"),
                    ],
                    "5",
                )],
            ),
        ]
    }

    /// Opens the settings panel.
    pub fn open(&mut self) {
        self.visible = true;
        self.current_tab = SettingsTab::General;
        self.has_changes = false;
    }

    /// Closes the settings panel.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Returns whether the panel is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Sets the current tab.
    pub fn set_tab(&mut self, tab: SettingsTab) {
        self.current_tab = tab;
    }

    /// Toggles a setting.
    pub fn toggle_setting(&mut self, key: &str) {
        for (_tab, settings) in &mut self.toggles {
            for setting in settings {
                if setting.key == key {
                    setting.enabled = !setting.enabled;
                    self.has_changes = true;
                    return;
                }
            }
        }
    }

    /// Sets a select setting value.
    pub fn set_select(&mut self, key: &str, value: String) {
        for (_tab, settings) in &mut self.selects {
            for setting in settings {
                if setting.key == key {
                    setting.selected = value;
                    self.has_changes = true;
                    return;
                }
            }
        }
    }

    /// Returns toggle settings for the current tab.
    fn current_toggles(&self) -> &[ToggleSetting] {
        self.toggles
            .iter()
            .find(|(tab, _)| *tab == self.current_tab)
            .map(|(_, settings)| settings.as_slice())
            .unwrap_or(&[])
    }

    /// Returns select settings for the current tab.
    fn current_selects(&self) -> &[SelectSetting] {
        self.selects
            .iter()
            .find(|(tab, _)| *tab == self.current_tab)
            .map(|(_, settings)| settings.as_slice())
            .unwrap_or(&[])
    }

    fn render_tab(&self, tab: SettingsTab, _cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = tab == self.current_tab;

        div()
            .id(SharedString::from(format!("tab-{:?}", tab)))
            .h(px(40.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .rounded(px(6.0))
            .when(is_selected, |d| d.bg(rgba(0x3B82F620)))
            .when(!is_selected, |d| d.hover(|d| d.bg(rgba(0xFFFFFF08))))
            .child(
                div()
                    .text_sm()
                    .when(is_selected, |d| d.text_color(rgba(0x3B82F6FF)))
                    .when(!is_selected, |d| d.text_color(rgba(0x71717AFF)))
                    .child(tab.icon()),
            )
            .child(
                div()
                    .text_sm()
                    .when(is_selected, |d| d.text_color(rgba(0xF4F4F5FF)))
                    .when(!is_selected, |d| d.text_color(rgba(0xA1A1AAFF)))
                    .child(tab.name()),
            )
    }

    fn render_toggle(&self, setting: &ToggleSetting, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(SharedString::from(format!("toggle-{}", setting.key)))
            .py(px(12.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .border_b_1()
            .border_color(rgba(0x27272AFF))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgba(0xE4E4E7FF))
                            .child(setting.label.clone()),
                    )
                    .when_some(setting.description.clone(), |d, desc| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(rgba(0x71717AFF))
                                .child(desc),
                        )
                    }),
            )
            .child(
                // Toggle switch
                div()
                    .w(px(44.0))
                    .h(px(24.0))
                    .rounded_full()
                    .cursor_pointer()
                    .when(setting.enabled, |d| d.bg(rgba(0x3B82F6FF)))
                    .when(!setting.enabled, |d| d.bg(rgba(0x3F3F46FF)))
                    .child(
                        div()
                            .size(px(20.0))
                            .mt(px(2.0))
                            .when(setting.enabled, |d| d.ml(px(22.0)))
                            .when(!setting.enabled, |d| d.ml(px(2.0)))
                            .rounded_full()
                            .bg(rgba(0xFFFFFFFF)),
                    ),
            )
    }

    fn render_select(&self, setting: &SelectSetting, _cx: &mut Context<Self>) -> impl IntoElement {
        let selected_label = setting
            .options
            .iter()
            .find(|o| o.value == setting.selected)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| SharedString::from("Select..."));

        div()
            .id(SharedString::from(format!("select-{}", setting.key)))
            .py(px(12.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .border_b_1()
            .border_color(rgba(0x27272AFF))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(0xE4E4E7FF))
                    .child(setting.label.clone()),
            )
            .child(
                div()
                    .min_w(px(140.0))
                    .h(px(32.0))
                    .px(px(12.0))
                    .bg(rgba(0x27272AFF))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .hover(|d| d.bg(rgba(0x3F3F46FF)))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgba(0xE4E4E7FF))
                            .child(selected_label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0x71717AFF))
                            .child("v"),
                    ),
            )
    }

    fn render_keybindings_content(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let bindings = vec![
            ("Compose", "C"),
            ("Reply", "R"),
            ("Reply All", "Shift+R"),
            ("Forward", "F"),
            ("Archive", "E"),
            ("Delete", "#"),
            ("Star", "S"),
            ("Mark Read", "Shift+U"),
            ("Go to Inbox", "G I"),
            ("Go to Starred", "G S"),
            ("Search", "/"),
            ("Command Palette", "Cmd+K"),
            ("Settings", "Cmd+,"),
        ];

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(bindings.iter().map(|(name, shortcut)| {
                div()
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgba(0x27272AFF))
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(rgba(0xE4E4E7FF))
                            .child(*name),
                    )
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .bg(rgba(0x27272AFF))
                            .rounded(px(4.0))
                            .text_xs()
                            .text_color(rgba(0xA1A1AAFF))
                            .child(*shortcut),
                    )
            }))
    }

    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.current_tab {
            SettingsTab::Keybindings => {
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_keybindings_content(cx))
            }
            _ => {
                let toggles = self.current_toggles();
                let selects = self.current_selects();

                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(toggles.iter().map(|t| self.render_toggle(t, cx)))
                    .children(selects.iter().map(|s| self.render_select(s, cx)))
            }
        }
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("settings-hidden");
        }

        // Backdrop
        div()
            .id("settings-backdrop")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Panel container
                div()
                    .id("settings-panel")
                    .w(px(720.0))
                    .h(px(560.0))
                    .bg(rgba(0x18181BFF))
                    .rounded(px(12.0))
                    .shadow_lg()
                    .border_1()
                    .border_color(rgba(0x27272AFF))
                    .flex()
                    .overflow_hidden()
                    // Sidebar
                    .child(
                        div()
                            .w(px(200.0))
                            .h_full()
                            .bg(rgba(0x0F0F10FF))
                            .border_r_1()
                            .border_color(rgba(0x27272AFF))
                            .flex()
                            .flex_col()
                            .child(
                                // Header
                                div()
                                    .h(px(56.0))
                                    .px(px(16.0))
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(rgba(0x27272AFF))
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(rgba(0xF4F4F5FF))
                                            .child("Settings"),
                                    ),
                            )
                            .child(
                                // Tab list
                                div()
                                    .flex_1()
                                    .py(px(8.0))
                                    .px(px(8.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .children(
                                        SettingsTab::all()
                                            .iter()
                                            .map(|tab| self.render_tab(*tab, cx)),
                                    ),
                            ),
                    )
                    // Main content
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            // Content header
                            .child(
                                div()
                                    .h(px(56.0))
                                    .px(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .border_b_1()
                                    .border_color(rgba(0x27272AFF))
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgba(0xF4F4F5FF))
                                            .child(self.current_tab.name()),
                                    )
                                    .child(
                                        // Close button
                                        div()
                                            .id("close-settings")
                                            .size(px(32.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(6.0))
                                            .cursor_pointer()
                                            .hover(|d| d.bg(rgba(0x27272AFF)))
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .text_color(rgba(0x71717AFF))
                                                    .child("x"),
                                            ),
                                    ),
                            )
                            // Content body
                            .child(
                                div()
                                    .flex_1()
                                    .p(px(24.0))
                                    .overflow_hidden()
                                    .child(self.render_content(cx)),
                            )
                            // Footer with save button
                            .when(self.has_changes, |d| {
                                d.child(
                                    div()
                                        .h(px(64.0))
                                        .px(px(24.0))
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap(px(12.0))
                                        .border_t_1()
                                        .border_color(rgba(0x27272AFF))
                                        .child(
                                            div()
                                                .id("cancel-changes")
                                                .px(px(16.0))
                                                .h(px(36.0))
                                                .flex()
                                                .items_center()
                                                .rounded(px(6.0))
                                                .cursor_pointer()
                                                .bg(rgba(0x27272AFF))
                                                .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(rgba(0xE4E4E7FF))
                                                        .child("Cancel"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("save-changes")
                                                .px(px(16.0))
                                                .h(px(36.0))
                                                .flex()
                                                .items_center()
                                                .rounded(px(6.0))
                                                .cursor_pointer()
                                                .bg(rgba(0x3B82F6FF))
                                                .hover(|d| d.bg(rgba(0x2563EBFF)))
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                        .text_color(rgba(0xFFFFFFFF))
                                                        .child("Save Changes"),
                                                ),
                                        ),
                                )
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_tab_names() {
        assert_eq!(SettingsTab::General.name(), "General");
        assert_eq!(SettingsTab::Ai.name(), "AI Features");
        assert_eq!(SettingsTab::Keybindings.name(), "Keyboard Shortcuts");
    }

    #[test]
    fn settings_tab_all() {
        let tabs = SettingsTab::all();
        assert_eq!(tabs.len(), 6);
        assert!(tabs.contains(&SettingsTab::General));
        assert!(tabs.contains(&SettingsTab::Privacy));
    }

    #[test]
    fn toggle_setting_builder() {
        let toggle = ToggleSetting::new("test_key", "Test Setting", true)
            .with_description("A test description");

        assert_eq!(toggle.key, "test_key");
        assert_eq!(toggle.label.as_ref(), "Test Setting");
        assert!(toggle.enabled);
        assert!(toggle.description.is_some());
    }

    #[test]
    fn select_setting_builder() {
        let select = SelectSetting::new(
            "theme",
            "Theme",
            vec![
                SelectOption::new("dark", "Dark"),
                SelectOption::new("light", "Light"),
            ],
            "dark",
        );

        assert_eq!(select.key, "theme");
        assert_eq!(select.options.len(), 2);
        assert_eq!(select.selected, "dark");
    }
}
