//! Usage statistics dashboard component.
//!
//! Displays email, productivity, and AI usage metrics with configurable
//! time ranges and export capability. Accessible via `G A`.

use chrono::{DateTime, Duration, Utc};
use gpui::{
    div, prelude::*, px, rgba, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

/// Time range for statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsTimeRange {
    /// Today only.
    Today,
    /// Last 7 days.
    #[default]
    Week,
    /// Last 30 days.
    Month,
    /// Last 90 days.
    Quarter,
    /// All time.
    AllTime,
}

impl StatsTimeRange {
    /// Returns the display name.
    pub fn name(&self) -> &'static str {
        match self {
            StatsTimeRange::Today => "Today",
            StatsTimeRange::Week => "Last 7 Days",
            StatsTimeRange::Month => "Last 30 Days",
            StatsTimeRange::Quarter => "Last 90 Days",
            StatsTimeRange::AllTime => "All Time",
        }
    }

    /// Returns all time ranges.
    pub fn all() -> &'static [StatsTimeRange] {
        &[
            StatsTimeRange::Today,
            StatsTimeRange::Week,
            StatsTimeRange::Month,
            StatsTimeRange::Quarter,
            StatsTimeRange::AllTime,
        ]
    }

    /// Returns the start date for this range.
    pub fn start_date(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        match self {
            StatsTimeRange::Today => Some(now - Duration::days(1)),
            StatsTimeRange::Week => Some(now - Duration::days(7)),
            StatsTimeRange::Month => Some(now - Duration::days(30)),
            StatsTimeRange::Quarter => Some(now - Duration::days(90)),
            StatsTimeRange::AllTime => None,
        }
    }
}

/// Email volume statistics.
#[derive(Debug, Clone, Default)]
pub struct EmailStats {
    /// Emails received.
    pub received: u32,
    /// Emails sent.
    pub sent: u32,
    /// Emails archived.
    pub archived: u32,
    /// Emails deleted.
    pub deleted: u32,
    /// Emails starred.
    pub starred: u32,
    /// Change from previous period (percentage).
    pub received_change: Option<f32>,
}

/// Productivity statistics.
#[derive(Debug, Clone, Default)]
pub struct ProductivityStats {
    /// Average response time in minutes.
    pub avg_response_time_mins: Option<f32>,
    /// Times reached inbox zero.
    pub inbox_zero_count: u32,
    /// Total sessions.
    pub sessions: u32,
    /// Time in app (seconds).
    pub time_in_app_secs: u64,
    /// Emails processed per session.
    pub emails_per_session: f32,
}

impl ProductivityStats {
    /// Formats time in app as human-readable string.
    pub fn time_in_app_display(&self) -> String {
        let hours = self.time_in_app_secs / 3600;
        let mins = (self.time_in_app_secs % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    /// Formats response time as human-readable string.
    pub fn response_time_display(&self) -> String {
        match self.avg_response_time_mins {
            Some(mins) if mins >= 60.0 => format!("{:.1}h", mins / 60.0),
            Some(mins) => format!("{:.0}m", mins),
            None => "N/A".to_string(),
        }
    }
}

/// AI usage statistics.
#[derive(Debug, Clone, Default)]
pub struct AiStats {
    /// Thread summaries generated.
    pub summaries_generated: u32,
    /// Compose assists used.
    pub compose_assists: u32,
    /// Compose assists accepted.
    pub compose_accepted: u32,
    /// Semantic searches performed.
    pub semantic_searches: u32,
    /// Total tokens used.
    pub tokens_used: u64,
    /// Estimated cost in USD.
    pub estimated_cost_usd: f32,
}

impl AiStats {
    /// Returns the compose assist acceptance rate.
    pub fn acceptance_rate(&self) -> Option<f32> {
        if self.compose_assists > 0 {
            Some(self.compose_accepted as f32 / self.compose_assists as f32 * 100.0)
        } else {
            None
        }
    }

    /// Formats tokens as human-readable string.
    pub fn tokens_display(&self) -> String {
        if self.tokens_used >= 1_000_000 {
            format!("{:.1}M", self.tokens_used as f32 / 1_000_000.0)
        } else if self.tokens_used >= 1_000 {
            format!("{:.1}K", self.tokens_used as f32 / 1_000.0)
        } else {
            format!("{}", self.tokens_used)
        }
    }
}

/// Top correspondent entry.
#[derive(Debug, Clone)]
pub struct TopCorrespondent {
    /// Email address.
    pub email: String,
    /// Display name.
    pub name: Option<String>,
    /// Number of emails exchanged.
    pub email_count: u32,
}

impl TopCorrespondent {
    /// Returns the display name or email.
    pub fn display(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.email)
    }
}

/// Busiest hour entry.
#[derive(Debug, Clone)]
pub struct BusiestHour {
    /// Hour (0-23).
    pub hour: u8,
    /// Number of emails.
    pub count: u32,
}

impl BusiestHour {
    /// Formats the hour for display.
    pub fn display(&self) -> String {
        let suffix = if self.hour < 12 { "AM" } else { "PM" };
        let hour = if self.hour == 0 {
            12
        } else if self.hour > 12 {
            self.hour - 12
        } else {
            self.hour
        };
        format!("{} {}", hour, suffix)
    }
}

/// The stats dashboard view component.
pub struct StatsDashboard {
    /// Whether the dashboard is visible.
    visible: bool,
    /// Selected time range.
    time_range: StatsTimeRange,
    /// Email statistics.
    email_stats: EmailStats,
    /// Productivity statistics.
    productivity_stats: ProductivityStats,
    /// AI statistics.
    ai_stats: AiStats,
    /// Top correspondents.
    top_correspondents: Vec<TopCorrespondent>,
    /// Busiest hours.
    busiest_hours: Vec<BusiestHour>,
    /// Whether data is loading.
    loading: bool,
}

impl StatsDashboard {
    /// Creates a new stats dashboard.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            visible: false,
            time_range: StatsTimeRange::Week,
            email_stats: EmailStats::default(),
            productivity_stats: ProductivityStats::default(),
            ai_stats: AiStats::default(),
            top_correspondents: Vec::new(),
            busiest_hours: Vec::new(),
            loading: false,
        }
    }

    /// Opens the dashboard.
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// Closes the dashboard.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Returns whether the dashboard is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Sets the time range.
    pub fn set_time_range(&mut self, range: StatsTimeRange) {
        self.time_range = range;
        // Trigger reload of stats
        self.loading = true;
    }

    /// Updates the email statistics.
    pub fn set_email_stats(&mut self, stats: EmailStats) {
        self.email_stats = stats;
    }

    /// Updates the productivity statistics.
    pub fn set_productivity_stats(&mut self, stats: ProductivityStats) {
        self.productivity_stats = stats;
    }

    /// Updates the AI statistics.
    pub fn set_ai_stats(&mut self, stats: AiStats) {
        self.ai_stats = stats;
    }

    /// Sets top correspondents.
    pub fn set_top_correspondents(&mut self, correspondents: Vec<TopCorrespondent>) {
        self.top_correspondents = correspondents;
    }

    /// Sets busiest hours.
    pub fn set_busiest_hours(&mut self, hours: Vec<BusiestHour>) {
        self.busiest_hours = hours;
    }

    /// Marks loading as complete.
    pub fn set_loaded(&mut self) {
        self.loading = false;
    }

    fn render_stat_card(
        &self,
        title: &str,
        value: &str,
        subtitle: Option<&str>,
        accent: bool,
    ) -> impl IntoElement {
        let title = title.to_string();
        let value = value.to_string();

        div()
            .p(px(16.0))
            .bg(rgba(0x27272AFF))
            .rounded(px(8.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(div().text_xs().text_color(rgba(0x71717AFF)).child(title))
            .child(
                div()
                    .text_2xl()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .when(accent, |d| d.text_color(rgba(0x3B82F6FF)))
                    .when(!accent, |d| d.text_color(rgba(0xF4F4F5FF)))
                    .child(value),
            )
            .when_some(subtitle.map(|s| s.to_string()), |d, s| {
                d.child(div().text_xs().text_color(rgba(0x52525BFF)).child(s))
            })
    }

    fn render_time_range_selector(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap(px(4.0))
            .children(StatsTimeRange::all().iter().map(|range| {
                let is_selected = *range == self.time_range;
                div()
                    .id(SharedString::from(format!("range-{:?}", range)))
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .when(is_selected, |d| d.bg(rgba(0x3B82F6FF)))
                    .when(!is_selected, |d| {
                        d.bg(rgba(0x27272AFF)).hover(|d| d.bg(rgba(0x3F3F46FF)))
                    })
                    .child(
                        div()
                            .text_xs()
                            .when(is_selected, |d| d.text_color(rgba(0xFFFFFFFF)))
                            .when(!is_selected, |d| d.text_color(rgba(0xA1A1AAFF)))
                            .child(range.name()),
                    )
            }))
    }

    fn render_email_section(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(0xE4E4E7FF))
                    .child("Email Volume"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        self.render_stat_card(
                            "Received",
                            &format!("{}", self.email_stats.received),
                            self.email_stats
                                .received_change
                                .map(|c| {
                                    if c >= 0.0 {
                                        format!("+{:.0}% vs prev", c)
                                    } else {
                                        format!("{:.0}% vs prev", c)
                                    }
                                })
                                .as_deref(),
                            true,
                        ),
                    )
                    .child(self.render_stat_card(
                        "Sent",
                        &format!("{}", self.email_stats.sent),
                        None,
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Archived",
                        &format!("{}", self.email_stats.archived),
                        None,
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Starred",
                        &format!("{}", self.email_stats.starred),
                        None,
                        false,
                    )),
            )
    }

    fn render_productivity_section(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(0xE4E4E7FF))
                    .child("Productivity"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(self.render_stat_card(
                        "Avg Response Time",
                        &self.productivity_stats.response_time_display(),
                        None,
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Time in App",
                        &self.productivity_stats.time_in_app_display(),
                        Some(&format!("{} sessions", self.productivity_stats.sessions)),
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Inbox Zero",
                        &format!("{}x", self.productivity_stats.inbox_zero_count),
                        None,
                        self.productivity_stats.inbox_zero_count > 0,
                    ))
                    .child(self.render_stat_card(
                        "Emails/Session",
                        &format!("{:.1}", self.productivity_stats.emails_per_session),
                        None,
                        false,
                    )),
            )
    }

    fn render_ai_section(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let acceptance = self
            .ai_stats
            .acceptance_rate()
            .map(|r| format!("{:.0}% accepted", r));

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(0xE4E4E7FF))
                    .child("AI Usage"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(self.render_stat_card(
                        "Summaries",
                        &format!("{}", self.ai_stats.summaries_generated),
                        None,
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Compose Assists",
                        &format!("{}", self.ai_stats.compose_assists),
                        acceptance.as_deref(),
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Semantic Searches",
                        &format!("{}", self.ai_stats.semantic_searches),
                        None,
                        false,
                    ))
                    .child(self.render_stat_card(
                        "Tokens Used",
                        &self.ai_stats.tokens_display(),
                        Some(&format!("~${:.2}", self.ai_stats.estimated_cost_usd)),
                        false,
                    )),
            )
    }

    fn render_correspondents_section(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(0xE4E4E7FF))
                    .child("Top Correspondents"),
            )
            .child(
                div()
                    .bg(rgba(0x27272AFF))
                    .rounded(px(8.0))
                    .p(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .when(self.top_correspondents.is_empty(), |d| {
                        d.child(
                            div()
                                .text_sm()
                                .text_color(rgba(0x52525BFF))
                                .child("No data yet"),
                        )
                    })
                    .when(!self.top_correspondents.is_empty(), |d| {
                        d.children(self.top_correspondents.iter().take(5).enumerate().map(
                            |(i, c)| {
                                let display = c.display().to_string();
                                let count = c.email_count;
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .w(px(16.0))
                                            .child(format!("{}.", i + 1)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .text_color(rgba(0xE4E4E7FF))
                                            .truncate()
                                            .child(display),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child(format!("{} emails", count)),
                                    )
                            },
                        ))
                    }),
            )
    }

    fn render_hours_section(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        // Find max for scaling
        let max_count = self
            .busiest_hours
            .iter()
            .map(|h| h.count)
            .max()
            .unwrap_or(1);

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(0xE4E4E7FF))
                    .child("Busiest Hours"),
            )
            .child(
                div()
                    .bg(rgba(0x27272AFF))
                    .rounded(px(8.0))
                    .p(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .when(self.busiest_hours.is_empty(), |d| {
                        d.child(
                            div()
                                .text_sm()
                                .text_color(rgba(0x52525BFF))
                                .child("No data yet"),
                        )
                    })
                    .when(!self.busiest_hours.is_empty(), |d| {
                        d.children(self.busiest_hours.iter().take(5).map(|h| {
                            let width_pct = (h.count as f32 / max_count as f32 * 100.0) as i32;
                            let display = h.display();
                            let count = h.count;
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .w(px(48.0))
                                        .text_xs()
                                        .text_color(rgba(0x71717AFF))
                                        .child(display),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(16.0))
                                        .bg(rgba(0x1F1F23FF))
                                        .rounded(px(2.0))
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .h_full()
                                                .w(gpui::relative(width_pct as f32 / 100.0))
                                                .bg(rgba(0x3B82F6FF)),
                                        ),
                                )
                                .child(
                                    div()
                                        .w(px(40.0))
                                        .text_xs()
                                        .text_color(rgba(0x71717AFF))
                                        .text_right()
                                        .child(format!("{}", count)),
                                )
                        }))
                    }),
            )
    }
}

impl Render for StatsDashboard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("stats-hidden");
        }

        // Backdrop
        div()
            .id("stats-backdrop")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Panel container
                div()
                    .id("stats-panel")
                    .w(px(800.0))
                    .h(px(640.0))
                    .bg(rgba(0x18181BFF))
                    .rounded(px(12.0))
                    .shadow_lg()
                    .border_1()
                    .border_color(rgba(0x27272AFF))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Header
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div().flex().items_center().gap(px(12.0)).child(
                                    div()
                                        .text_lg()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(rgba(0xF4F4F5FF))
                                        .child("Usage Statistics"),
                                ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .child(self.render_time_range_selector(cx))
                                    .child(
                                        div()
                                            .id("close-stats")
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
                            ),
                    )
                    // Content
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .p(px(20.0))
                            .flex()
                            .flex_col()
                            .gap(px(24.0))
                            .child(self.render_email_section(cx))
                            .child(self.render_productivity_section(cx))
                            .child(self.render_ai_section(cx))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(20.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(self.render_correspondents_section(cx)),
                                    )
                                    .child(div().flex_1().child(self.render_hours_section(cx))),
                            ),
                    )
                    // Footer
                    .child(
                        div()
                            .h(px(48.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_t_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgba(0x52525BFF))
                                    .child("All data stored locally"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .id("export-json")
                                            .px(px(12.0))
                                            .h(px(28.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .bg(rgba(0x27272AFF))
                                            .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgba(0xA1A1AAFF))
                                                    .child("Export JSON"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("export-csv")
                                            .px(px(12.0))
                                            .h(px(28.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .bg(rgba(0x27272AFF))
                                            .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgba(0xA1A1AAFF))
                                                    .child("Export CSV"),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_range_names() {
        assert_eq!(StatsTimeRange::Today.name(), "Today");
        assert_eq!(StatsTimeRange::Week.name(), "Last 7 Days");
        assert_eq!(StatsTimeRange::AllTime.name(), "All Time");
    }

    #[test]
    fn time_range_start_dates() {
        assert!(StatsTimeRange::Today.start_date().is_some());
        assert!(StatsTimeRange::AllTime.start_date().is_none());
    }

    #[test]
    fn productivity_time_display() {
        let stats = ProductivityStats {
            time_in_app_secs: 7200, // 2 hours
            ..Default::default()
        };
        assert_eq!(stats.time_in_app_display(), "2h 0m");

        let stats2 = ProductivityStats {
            time_in_app_secs: 1800, // 30 mins
            ..Default::default()
        };
        assert_eq!(stats2.time_in_app_display(), "30m");
    }

    #[test]
    fn ai_stats_tokens_display() {
        let stats = AiStats {
            tokens_used: 1_500_000,
            ..Default::default()
        };
        assert_eq!(stats.tokens_display(), "1.5M");

        let stats2 = AiStats {
            tokens_used: 15_000,
            ..Default::default()
        };
        assert_eq!(stats2.tokens_display(), "15.0K");
    }

    #[test]
    fn ai_stats_acceptance_rate() {
        let stats = AiStats {
            compose_assists: 10,
            compose_accepted: 8,
            ..Default::default()
        };
        assert_eq!(stats.acceptance_rate(), Some(80.0));

        let stats2 = AiStats::default();
        assert_eq!(stats2.acceptance_rate(), None);
    }

    #[test]
    fn busiest_hour_display() {
        assert_eq!(BusiestHour { hour: 0, count: 5 }.display(), "12 AM");
        assert_eq!(BusiestHour { hour: 9, count: 5 }.display(), "9 AM");
        assert_eq!(BusiestHour { hour: 12, count: 5 }.display(), "12 PM");
        assert_eq!(BusiestHour { hour: 15, count: 5 }.display(), "3 PM");
    }
}
