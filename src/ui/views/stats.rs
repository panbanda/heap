//! Statistics dashboard view.
//!
//! Displays email activity metrics and AI usage statistics.

use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};

use crate::ui::theme::ThemeColors;

/// Statistics dashboard view component.
pub struct StatsView {
    colors: ThemeColors,
    stats: DashboardStats,
    time_range: TimeRange,
}

/// Dashboard statistics data.
#[derive(Clone, Default)]
pub struct DashboardStats {
    pub emails_received: u32,
    pub emails_sent: u32,
    pub emails_archived: u32,
    pub emails_trashed: u32,
    pub average_response_time_mins: u32,
    pub unread_count: u32,
    pub ai_summaries: u32,
    pub ai_drafts: u32,
    pub ai_searches: u32,
    pub ai_tokens_used: u64,
    pub time_in_app_mins: u32,
    pub sessions: u32,
    pub inbox_zero_days: u32,
}

/// Time range for statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TimeRange {
    Today,
    #[default]
    Week,
    Month,
    Year,
}

impl TimeRange {
    fn label(&self) -> &str {
        match self {
            Self::Today => "Today",
            Self::Week => "This Week",
            Self::Month => "This Month",
            Self::Year => "This Year",
        }
    }
}

impl StatsView {
    /// Create a new stats view.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            colors: ThemeColors::dark(),
            stats: DashboardStats::default(),
            time_range: TimeRange::Week,
        }
    }

    /// Set the statistics data.
    pub fn set_stats(&mut self, stats: DashboardStats) {
        self.stats = stats;
    }

    /// Set the time range.
    pub fn set_time_range(&mut self, range: TimeRange) {
        self.time_range = range;
    }

    fn render_stat_card(&self, title: &str, value: &str, subtitle: Option<&str>) -> impl IntoElement {
        let surface = self.colors.surface;
        let border = self.colors.border;
        let text_primary = self.colors.text_primary;
        let text_secondary = self.colors.text_secondary;
        let text_muted = self.colors.text_muted;

        div()
            .p(px(16.0))
            .rounded(px(8.0))
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                div()
                    .text_sm()
                    .text_color(text_secondary)
                    .mb(px(8.0))
                    .child(SharedString::from(title.to_string())),
            )
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(text_primary)
                    .child(SharedString::from(value.to_string())),
            )
            .when_some(subtitle, |this, sub| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(text_muted)
                        .mt(px(4.0))
                        .child(SharedString::from(sub.to_string())),
                )
            })
    }

    fn render_header(&self) -> impl IntoElement {
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;
        let surface = self.colors.surface;
        let border = self.colors.border;

        div()
            .flex()
            .items_center()
            .justify_between()
            .mb(px(24.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text_primary)
                    .child(SharedString::from("Statistics")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .children(
                        [TimeRange::Today, TimeRange::Week, TimeRange::Month, TimeRange::Year]
                            .into_iter()
                            .map(|range| {
                                let is_active = range == self.time_range;
                                let bg = if is_active {
                                    surface
                                } else {
                                    gpui::Hsla::transparent_black()
                                };

                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(bg)
                                    .border_1()
                                    .border_color(if is_active { border } else { gpui::Hsla::transparent_black() })
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(if is_active { text_primary } else { text_muted })
                                    .child(SharedString::from(range.label().to_string()))
                            }),
                    ),
            )
    }

    fn render_email_stats(&self) -> impl IntoElement {
        div()
            .mb(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(self.colors.text_primary)
                    .mb(px(16.0))
                    .child(SharedString::from("Email Activity")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(16.0))
                    .child(self.render_stat_card(
                        "Received",
                        &self.stats.emails_received.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Sent",
                        &self.stats.emails_sent.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Archived",
                        &self.stats.emails_archived.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Trashed",
                        &self.stats.emails_trashed.to_string(),
                        None,
                    )),
            )
    }

    fn render_productivity_stats(&self) -> impl IntoElement {
        let response_time = if self.stats.average_response_time_mins >= 60 {
            format!("{}h {}m", self.stats.average_response_time_mins / 60, self.stats.average_response_time_mins % 60)
        } else {
            format!("{}m", self.stats.average_response_time_mins)
        };

        let time_in_app = if self.stats.time_in_app_mins >= 60 {
            format!("{}h {}m", self.stats.time_in_app_mins / 60, self.stats.time_in_app_mins % 60)
        } else {
            format!("{}m", self.stats.time_in_app_mins)
        };

        div()
            .mb(px(24.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(self.colors.text_primary)
                    .mb(px(16.0))
                    .child(SharedString::from("Productivity")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(16.0))
                    .child(self.render_stat_card(
                        "Avg Response Time",
                        &response_time,
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Unread Emails",
                        &self.stats.unread_count.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Time in App",
                        &time_in_app,
                        Some(&format!("{} sessions", self.stats.sessions)),
                    ))
                    .child(self.render_stat_card(
                        "Inbox Zero Days",
                        &self.stats.inbox_zero_days.to_string(),
                        None,
                    )),
            )
    }

    fn render_ai_stats(&self) -> impl IntoElement {
        let tokens_display = if self.stats.ai_tokens_used >= 1_000_000 {
            format!("{:.1}M", self.stats.ai_tokens_used as f64 / 1_000_000.0)
        } else if self.stats.ai_tokens_used >= 1_000 {
            format!("{:.1}K", self.stats.ai_tokens_used as f64 / 1_000.0)
        } else {
            self.stats.ai_tokens_used.to_string()
        };

        div()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(self.colors.text_primary)
                    .mb(px(16.0))
                    .child(SharedString::from("AI Usage")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(16.0))
                    .child(self.render_stat_card(
                        "Summaries Generated",
                        &self.stats.ai_summaries.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Drafts Suggested",
                        &self.stats.ai_drafts.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Semantic Searches",
                        &self.stats.ai_searches.to_string(),
                        None,
                    ))
                    .child(self.render_stat_card(
                        "Tokens Used",
                        &tokens_display,
                        None,
                    )),
            )
    }
}

impl Render for StatsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("stats")
            .size_full()
            .p(px(24.0))
            .bg(self.colors.background)
            .overflow_y_hidden()
            .child(self.render_header())
            .child(self.render_email_stats())
            .child(self.render_productivity_stats())
            .child(self.render_ai_stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_range_labels() {
        assert_eq!(TimeRange::Today.label(), "Today");
        assert_eq!(TimeRange::Week.label(), "This Week");
        assert_eq!(TimeRange::Month.label(), "This Month");
        assert_eq!(TimeRange::Year.label(), "This Year");
    }

    #[test]
    fn set_stats() {
        let mut view = StatsView {
            colors: ThemeColors::dark(),
            stats: DashboardStats::default(),
            time_range: TimeRange::Week,
        };

        let stats = DashboardStats {
            emails_received: 100,
            emails_sent: 50,
            ..Default::default()
        };

        view.set_stats(stats);
        assert_eq!(view.stats.emails_received, 100);
        assert_eq!(view.stats.emails_sent, 50);
    }

    #[test]
    fn set_time_range() {
        let mut view = StatsView {
            colors: ThemeColors::dark(),
            stats: DashboardStats::default(),
            time_range: TimeRange::Week,
        };

        assert_eq!(view.time_range, TimeRange::Week);

        view.set_time_range(TimeRange::Month);
        assert_eq!(view.time_range, TimeRange::Month);
    }

    #[test]
    fn dashboard_stats_default() {
        let stats = DashboardStats::default();
        assert_eq!(stats.emails_received, 0);
        assert_eq!(stats.ai_tokens_used, 0);
    }
}
