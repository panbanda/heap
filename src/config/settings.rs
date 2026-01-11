//! Application settings and configuration types.
//!
//! Settings are persisted to `~/.config/margin/settings.json` (or XDG equivalent)
//! and loaded at application startup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level application settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Visual appearance settings.
    pub appearance: AppearanceSettings,
    /// AI feature configuration.
    pub ai: AiSettings,
    /// Notification preferences.
    pub notifications: NotificationSettings,
    /// Background sync settings.
    pub sync: SyncSettings,
    /// Custom keybinding overrides.
    pub keybindings: KeybindingSettings,
    /// Privacy-related settings.
    pub privacy: PrivacySettings,
}

/// Visual appearance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    /// Color theme.
    pub theme: Theme,
    /// UI font family name.
    pub font_family: String,
    /// Base font size in pixels.
    pub font_size: u8,
    /// UI density/spacing.
    pub density: Density,
    /// Sidebar width in pixels.
    pub sidebar_width: u32,
    /// Reading pane width in pixels.
    pub reading_pane_width: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            font_family: "Inter".to_string(),
            font_size: 14,
            density: Density::Default,
            sidebar_width: 240,
            reading_pane_width: 500,
        }
    }
}

/// Color theme selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Dark color scheme.
    Dark,
    /// Light color scheme.
    Light,
    /// Follow system preference.
    System,
}

/// UI density/spacing level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    /// Tighter spacing for power users.
    Compact,
    /// Balanced spacing.
    Default,
    /// More generous spacing.
    Relaxed,
}

/// AI feature configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiSettings {
    /// Master switch for AI features.
    pub enabled: bool,
    /// Name of the default AI provider.
    pub default_provider: String,
    /// Provider-specific configurations keyed by provider name.
    pub providers: HashMap<String, ProviderSettings>,
    /// Email summarization settings.
    pub summary_settings: SummarySettings,
    /// Reply composition settings.
    pub compose_settings: ComposeSettings,
    /// Semantic search settings.
    pub search_settings: SearchSettings,
}

/// Configuration for a single AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    /// Keychain identifier for the API key.
    pub api_key_keychain_id: String,
    /// Custom API endpoint (for self-hosted or compatible APIs).
    pub base_url: Option<String>,
    /// Model identifier.
    pub model: String,
    /// Sampling temperature (0.0 to 1.0).
    pub temperature: f32,
    /// Maximum tokens in response.
    pub max_tokens: Option<usize>,
}

/// Settings for email summarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarySettings {
    /// Whether summarization is enabled.
    pub enabled: bool,
    /// Override provider for summaries (uses default if None).
    pub provider: Option<String>,
    /// System prompt for summarization.
    pub system_prompt: String,
    /// Maximum summary length in characters.
    pub max_length: usize,
}

impl Default for SummarySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: None,
            system_prompt:
                "Summarize this email thread concisely, highlighting key points and action items."
                    .to_string(),
            max_length: 500,
        }
    }
}

/// Settings for AI-assisted reply composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeSettings {
    /// Whether AI composition is enabled.
    pub enabled: bool,
    /// Override provider for composition (uses default if None).
    pub provider: Option<String>,
    /// System prompt for composition.
    pub system_prompt: String,
    /// Writing tone preference.
    pub tone: Tone,
    /// Whether to learn from user's sent emails.
    pub learn_from_sent: bool,
}

impl Default for ComposeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: None,
            system_prompt: "Draft a reply matching the user's communication style.".to_string(),
            tone: Tone::Casual,
            learn_from_sent: false,
        }
    }
}

/// Writing tone for AI-generated content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    /// Professional, business-appropriate language.
    Formal,
    /// Friendly, conversational language.
    Casual,
    /// Concise, to-the-point responses.
    Brief,
    /// Comprehensive, thorough responses.
    Detailed,
    /// User-defined custom prompt.
    Custom(String),
}

/// Settings for semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    /// Whether semantic search is enabled.
    pub enabled: bool,
    /// Maximum number of search results.
    pub max_results: usize,
    /// Minimum similarity score (0.0 to 1.0).
    pub min_similarity: f32,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_results: 50,
            min_similarity: 0.5,
        }
    }
}

/// Notification preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Master switch for notifications.
    pub enabled: bool,
    /// Which emails trigger notifications.
    pub new_email: NewEmailNotification,
    /// Whether to notify for snooze reminders.
    pub snooze_reminders: bool,
    /// Whether to play notification sounds.
    pub sound_enabled: bool,
    /// Optional quiet hours configuration.
    pub quiet_hours: Option<QuietHours>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            new_email: NewEmailNotification::VipOnly,
            snooze_reminders: true,
            sound_enabled: false,
            quiet_hours: None,
        }
    }
}

/// Which new emails trigger notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NewEmailNotification {
    /// Notify for all new emails.
    All,
    /// Only notify for VIP senders.
    VipOnly,
    /// Never notify for new emails.
    None,
}

/// Time range during which notifications are suppressed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    /// Start time in 24-hour format (e.g., "22:00").
    pub start: String,
    /// End time in 24-hour format (e.g., "07:00").
    pub end: String,
}

/// Background sync configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    /// Whether background sync is enabled.
    pub enabled: bool,
    /// Sync interval in seconds.
    pub interval_seconds: u32,
    /// Whether to sync when on battery power.
    pub sync_on_battery: bool,
    /// Whether to sync on metered connections.
    pub sync_on_metered: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 300,
            sync_on_battery: true,
            sync_on_metered: false,
        }
    }
}

/// Custom keybinding overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindingSettings {
    /// Map of action name to key sequence.
    pub overrides: HashMap<String, String>,
}

/// Privacy-related settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Whether to send read receipts.
    pub read_receipts_enabled: bool,
    /// Whether to load external content (images, etc.).
    pub external_content_enabled: bool,
    /// How long to retain local telemetry data (days).
    pub telemetry_retention_days: u32,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            read_receipts_enabled: false,
            external_content_enabled: false,
            telemetry_retention_days: 90,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        let settings = Settings::default();
        assert!(!settings.ai.enabled);
        assert!(settings.sync.enabled);
        assert_eq!(settings.appearance.font_size, 14);
    }

    #[test]
    fn theme_serialization() {
        let theme = Theme::Dark;
        let json = serde_json::to_string(&theme).unwrap();
        assert_eq!(json, "\"dark\"");

        let deserialized: Theme = serde_json::from_str("\"light\"").unwrap();
        assert_eq!(deserialized, Theme::Light);
    }

    #[test]
    fn density_serialization() {
        let density = Density::Compact;
        let json = serde_json::to_string(&density).unwrap();
        assert_eq!(json, "\"compact\"");
    }

    #[test]
    fn tone_custom_variant() {
        let tone = Tone::Custom("Be concise but warm.".to_string());
        let json = serde_json::to_string(&tone).unwrap();
        assert!(json.contains("custom"));

        let formal: Tone = serde_json::from_str("\"formal\"").unwrap();
        assert_eq!(formal, Tone::Formal);
    }

    #[test]
    fn settings_roundtrip() {
        let mut settings = Settings::default();
        settings.appearance.theme = Theme::Dark;
        settings.ai.enabled = true;
        settings.ai.default_provider = "anthropic".to_string();
        settings.ai.providers.insert(
            "anthropic".to_string(),
            ProviderSettings {
                api_key_keychain_id: "anthropic_api_key".to_string(),
                base_url: None,
                model: "claude-3-5-sonnet".to_string(),
                temperature: 0.7,
                max_tokens: Some(4096),
            },
        );

        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.appearance.theme, Theme::Dark);
        assert!(deserialized.ai.enabled);
        assert_eq!(deserialized.ai.default_provider, "anthropic");
        assert!(deserialized.ai.providers.contains_key("anthropic"));
    }

    #[test]
    fn new_email_notification_variants() {
        let all: NewEmailNotification = serde_json::from_str("\"all\"").unwrap();
        assert_eq!(all, NewEmailNotification::All);

        let vip: NewEmailNotification = serde_json::from_str("\"vip_only\"").unwrap();
        assert_eq!(vip, NewEmailNotification::VipOnly);

        let none: NewEmailNotification = serde_json::from_str("\"none\"").unwrap();
        assert_eq!(none, NewEmailNotification::None);
    }

    #[test]
    fn quiet_hours_config() {
        let quiet = QuietHours {
            start: "22:00".to_string(),
            end: "07:00".to_string(),
        };

        let json = serde_json::to_string(&quiet).unwrap();
        let deserialized: QuietHours = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.start, "22:00");
        assert_eq!(deserialized.end, "07:00");
    }

    #[test]
    fn keybinding_overrides() {
        let mut keybindings = KeybindingSettings::default();
        keybindings
            .overrides
            .insert("archive".to_string(), "a".to_string());
        keybindings
            .overrides
            .insert("trash".to_string(), "d".to_string());

        let json = serde_json::to_string(&keybindings).unwrap();
        let deserialized: KeybindingSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.overrides.get("archive"),
            Some(&"a".to_string())
        );
    }
}
