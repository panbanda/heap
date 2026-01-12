//! Statistics service for computing usage metrics.
//!
//! Aggregates and computes statistics for:
//! - Email volume (received, sent, archived, deleted)
//! - Productivity metrics (response time, inbox zero, sessions)
//! - AI usage (summaries, compose assists, tokens)
//! - Patterns (busiest hours, top correspondents)

use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use thiserror::Error;

use crate::domain::AccountId;

/// Helper to convert NaiveDate to DateTime<Utc> at midnight.
fn naive_date_to_utc(date: chrono::NaiveDate) -> DateTime<Utc> {
    Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN))
}

// Re-export StatsTimeRange from telemetry_service to avoid duplication
pub use super::telemetry_service::StatsTimeRange;

/// Errors that can occur during stats operations.
#[derive(Debug, Error)]
pub enum StatsError {
    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),

    /// Computation error.
    #[error("computation error: {0}")]
    Computation(String),
}

/// Result type for stats operations.
pub type StatsResult<T> = Result<T, StatsError>;

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

impl EmailStats {
    /// Computes the change percentage between two stats.
    pub fn compute_change(&self, previous: &EmailStats) -> f32 {
        if previous.received == 0 {
            return 0.0;
        }
        ((self.received as f32 - previous.received as f32) / previous.received as f32) * 100.0
    }
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

    /// Estimates cost based on token usage.
    pub fn estimate_cost(&mut self, cost_per_1k_tokens: f32) {
        self.estimated_cost_usd = (self.tokens_used as f32 / 1000.0) * cost_per_1k_tokens;
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
    /// Number sent to this contact.
    pub sent_count: u32,
    /// Number received from this contact.
    pub received_count: u32,
}

/// Busiest hour entry.
#[derive(Debug, Clone)]
pub struct BusiestHour {
    /// Hour (0-23).
    pub hour: u8,
    /// Number of emails.
    pub count: u32,
    /// Percentage of total.
    pub percentage: f32,
}

/// Daily activity data point.
#[derive(Debug, Clone)]
pub struct DailyActivity {
    /// Date.
    pub date: DateTime<Utc>,
    /// Emails received.
    pub received: u32,
    /// Emails sent.
    pub sent: u32,
    /// Emails archived.
    pub archived: u32,
}

/// Complete statistics report.
#[derive(Debug, Clone)]
pub struct StatsReport {
    /// Time range for this report.
    pub time_range: StatsTimeRange,
    /// Email statistics.
    pub email: EmailStats,
    /// Productivity statistics.
    pub productivity: ProductivityStats,
    /// AI statistics.
    pub ai: AiStats,
    /// Top correspondents.
    pub top_correspondents: Vec<TopCorrespondent>,
    /// Busiest hours.
    pub busiest_hours: Vec<BusiestHour>,
    /// Daily activity.
    pub daily_activity: Vec<DailyActivity>,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
}

impl Default for StatsReport {
    fn default() -> Self {
        Self {
            time_range: StatsTimeRange::Week,
            email: EmailStats::default(),
            productivity: ProductivityStats::default(),
            ai: AiStats::default(),
            top_correspondents: Vec::new(),
            busiest_hours: Vec::new(),
            daily_activity: Vec::new(),
            generated_at: Utc::now(),
        }
    }
}

impl StatsReport {
    /// Creates a new empty report.
    pub fn new(time_range: StatsTimeRange) -> Self {
        Self {
            time_range,
            ..Default::default()
        }
    }
}

/// Storage trait for stats persistence.
#[async_trait]
pub trait StatsStorage: Send + Sync {
    /// Gets email counts for a time range.
    async fn get_email_counts(
        &self,
        account_id: &AccountId,
        start: Option<DateTime<Utc>>,
        end: DateTime<Utc>,
    ) -> StatsResult<EmailStats>;

    /// Gets AI usage for a time range.
    async fn get_ai_usage(
        &self,
        account_id: &AccountId,
        start: Option<DateTime<Utc>>,
        end: DateTime<Utc>,
    ) -> StatsResult<AiStats>;

    /// Gets session data.
    async fn get_session_data(
        &self,
        account_id: &AccountId,
        start: Option<DateTime<Utc>>,
        end: DateTime<Utc>,
    ) -> StatsResult<ProductivityStats>;

    /// Gets top correspondents.
    async fn get_top_correspondents(
        &self,
        account_id: &AccountId,
        start: Option<DateTime<Utc>>,
        limit: usize,
    ) -> StatsResult<Vec<TopCorrespondent>>;

    /// Gets email counts by hour.
    async fn get_hourly_distribution(
        &self,
        account_id: &AccountId,
        start: Option<DateTime<Utc>>,
    ) -> StatsResult<Vec<BusiestHour>>;

    /// Gets daily activity.
    async fn get_daily_activity(
        &self,
        account_id: &AccountId,
        start: Option<DateTime<Utc>>,
        end: DateTime<Utc>,
    ) -> StatsResult<Vec<DailyActivity>>;

    /// Records an event.
    async fn record_event(&self, account_id: &AccountId, event: StatsEvent) -> StatsResult<()>;
}

/// Events to track for statistics.
#[derive(Debug, Clone)]
pub enum StatsEvent {
    /// Email received.
    EmailReceived { from: String },
    /// Email sent.
    EmailSent { to: Vec<String> },
    /// Email archived.
    EmailArchived { count: u32 },
    /// Email deleted.
    EmailDeleted { count: u32 },
    /// Email starred.
    EmailStarred,
    /// Email unstarred.
    EmailUnstarred,
    /// Session started.
    SessionStart,
    /// Session ended.
    SessionEnd { duration_secs: u64 },
    /// Inbox zero reached.
    InboxZero,
    /// Summary generated.
    AiSummary { tokens: u32 },
    /// Compose assist used.
    AiComposeUsed { tokens: u32 },
    /// Compose assist accepted.
    AiComposeAccepted,
    /// Semantic search performed.
    AiSemanticSearch { tokens: u32 },
    /// Response sent.
    ResponseSent { response_time_secs: u64 },
}

/// Service for computing and managing statistics.
pub struct StatsService<S: StatsStorage> {
    storage: S,
    account_id: AccountId,
    cost_per_1k_tokens: f32,
}

impl<S: StatsStorage> StatsService<S> {
    /// Creates a new stats service.
    pub fn new(storage: S, account_id: AccountId) -> Self {
        Self {
            storage,
            account_id,
            cost_per_1k_tokens: 0.002, // Default pricing
        }
    }

    /// Sets the cost per 1k tokens for AI cost estimation.
    pub fn set_token_cost(&mut self, cost: f32) {
        self.cost_per_1k_tokens = cost;
    }

    /// Records an event.
    pub async fn record(&self, event: StatsEvent) -> StatsResult<()> {
        self.storage.record_event(&self.account_id, event).await
    }

    /// Generates a complete stats report for a time range.
    pub async fn generate_report(&self, time_range: StatsTimeRange) -> StatsResult<StatsReport> {
        let now = Utc::now();
        let start = time_range.start_date().map(naive_date_to_utc);

        // Get all stats in parallel conceptually (sequential here for simplicity)
        let email = self
            .storage
            .get_email_counts(&self.account_id, start, now)
            .await?;
        let mut ai = self
            .storage
            .get_ai_usage(&self.account_id, start, now)
            .await?;
        let productivity = self
            .storage
            .get_session_data(&self.account_id, start, now)
            .await?;
        let top_correspondents = self
            .storage
            .get_top_correspondents(&self.account_id, start, 10)
            .await?;
        let busiest_hours = self
            .storage
            .get_hourly_distribution(&self.account_id, start)
            .await?;
        let daily_activity = self
            .storage
            .get_daily_activity(&self.account_id, start, now)
            .await?;

        // Estimate AI cost
        ai.estimate_cost(self.cost_per_1k_tokens);

        Ok(StatsReport {
            time_range,
            email,
            productivity,
            ai,
            top_correspondents,
            busiest_hours,
            daily_activity,
            generated_at: now,
        })
    }

    /// Gets email stats only.
    pub async fn get_email_stats(&self, time_range: StatsTimeRange) -> StatsResult<EmailStats> {
        let now = Utc::now();
        let start = time_range.start_date().map(naive_date_to_utc);
        self.storage
            .get_email_counts(&self.account_id, start, now)
            .await
    }

    /// Gets AI stats only.
    pub async fn get_ai_stats(&self, time_range: StatsTimeRange) -> StatsResult<AiStats> {
        let now = Utc::now();
        let start = time_range.start_date().map(naive_date_to_utc);
        let mut ai = self
            .storage
            .get_ai_usage(&self.account_id, start, now)
            .await?;
        ai.estimate_cost(self.cost_per_1k_tokens);
        Ok(ai)
    }

    /// Gets productivity stats only.
    pub async fn get_productivity_stats(
        &self,
        time_range: StatsTimeRange,
    ) -> StatsResult<ProductivityStats> {
        let now = Utc::now();
        let start = time_range.start_date().map(naive_date_to_utc);
        self.storage
            .get_session_data(&self.account_id, start, now)
            .await
    }

    /// Compares stats between two time ranges.
    pub async fn compare(
        &self,
        current: StatsTimeRange,
        previous: StatsTimeRange,
    ) -> StatsResult<(StatsReport, StatsReport)> {
        let current_report = self.generate_report(current).await?;
        let previous_report = self.generate_report(previous).await?;
        Ok((current_report, previous_report))
    }

    /// Exports stats to JSON format.
    pub fn export_json(&self, report: &StatsReport) -> String {
        serde_json::to_string_pretty(&ReportExport::from(report)).unwrap_or_default()
    }

    /// Exports stats to CSV format.
    pub fn export_csv(&self, report: &StatsReport) -> String {
        let mut csv = String::new();
        csv.push_str("Metric,Value\n");
        csv.push_str(&format!("Emails Received,{}\n", report.email.received));
        csv.push_str(&format!("Emails Sent,{}\n", report.email.sent));
        csv.push_str(&format!("Emails Archived,{}\n", report.email.archived));
        csv.push_str(&format!("Emails Deleted,{}\n", report.email.deleted));
        csv.push_str(&format!("Emails Starred,{}\n", report.email.starred));
        csv.push_str(&format!("Sessions,{}\n", report.productivity.sessions));
        csv.push_str(&format!(
            "Time in App (sec),{}\n",
            report.productivity.time_in_app_secs
        ));
        csv.push_str(&format!(
            "Inbox Zero Count,{}\n",
            report.productivity.inbox_zero_count
        ));
        csv.push_str(&format!("AI Summaries,{}\n", report.ai.summaries_generated));
        csv.push_str(&format!(
            "AI Compose Assists,{}\n",
            report.ai.compose_assists
        ));
        csv.push_str(&format!("AI Tokens Used,{}\n", report.ai.tokens_used));
        csv.push_str(&format!(
            "AI Estimated Cost,$\"{:.2}\"\n",
            report.ai.estimated_cost_usd
        ));
        csv
    }
}

/// Serializable export format for stats.
#[derive(Debug, serde::Serialize)]
struct ReportExport {
    time_range: String,
    generated_at: String,
    email: EmailStatsExport,
    productivity: ProductivityStatsExport,
    ai: AiStatsExport,
}

#[derive(Debug, serde::Serialize)]
struct EmailStatsExport {
    received: u32,
    sent: u32,
    archived: u32,
    deleted: u32,
    starred: u32,
}

#[derive(Debug, serde::Serialize)]
struct ProductivityStatsExport {
    avg_response_time_mins: Option<f32>,
    inbox_zero_count: u32,
    sessions: u32,
    time_in_app_secs: u64,
    emails_per_session: f32,
}

#[derive(Debug, serde::Serialize)]
struct AiStatsExport {
    summaries_generated: u32,
    compose_assists: u32,
    compose_accepted: u32,
    semantic_searches: u32,
    tokens_used: u64,
    estimated_cost_usd: f32,
}

impl From<&StatsReport> for ReportExport {
    fn from(report: &StatsReport) -> Self {
        Self {
            time_range: format!("{:?}", report.time_range),
            generated_at: report.generated_at.to_rfc3339(),
            email: EmailStatsExport {
                received: report.email.received,
                sent: report.email.sent,
                archived: report.email.archived,
                deleted: report.email.deleted,
                starred: report.email.starred,
            },
            productivity: ProductivityStatsExport {
                avg_response_time_mins: report.productivity.avg_response_time_mins,
                inbox_zero_count: report.productivity.inbox_zero_count,
                sessions: report.productivity.sessions,
                time_in_app_secs: report.productivity.time_in_app_secs,
                emails_per_session: report.productivity.emails_per_session,
            },
            ai: AiStatsExport {
                summaries_generated: report.ai.summaries_generated,
                compose_assists: report.ai.compose_assists,
                compose_accepted: report.ai.compose_accepted,
                semantic_searches: report.ai.semantic_searches,
                tokens_used: report.ai.tokens_used,
                estimated_cost_usd: report.ai.estimated_cost_usd,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_range_start_dates() {
        assert!(StatsTimeRange::Today.start_date().is_some());
        assert!(StatsTimeRange::Week.start_date().is_some());
        assert!(StatsTimeRange::AllTime.start_date().is_none());
    }

    #[test]
    fn email_stats_change() {
        let current = EmailStats {
            received: 150,
            ..Default::default()
        };
        let previous = EmailStats {
            received: 100,
            ..Default::default()
        };

        let change = current.compute_change(&previous);
        assert!((change - 50.0).abs() < 0.01);
    }

    #[test]
    fn ai_stats_acceptance_rate() {
        let stats = AiStats {
            compose_assists: 10,
            compose_accepted: 7,
            ..Default::default()
        };

        assert_eq!(stats.acceptance_rate(), Some(70.0));
    }

    #[test]
    fn ai_stats_cost_estimation() {
        let mut stats = AiStats {
            tokens_used: 10000,
            ..Default::default()
        };

        stats.estimate_cost(0.002);
        assert!((stats.estimated_cost_usd - 0.02).abs() < 0.001);
    }

    #[test]
    fn csv_export() {
        let report = StatsReport {
            time_range: StatsTimeRange::Week,
            email: EmailStats {
                received: 100,
                sent: 50,
                archived: 30,
                deleted: 10,
                starred: 5,
                received_change: None,
            },
            ..Default::default()
        };

        let service = StatsService::new(MockStorage, AccountId::from("test"));
        let csv = service.export_csv(&report);

        assert!(csv.contains("Emails Received,100"));
        assert!(csv.contains("Emails Sent,50"));
    }

    struct MockStorage;

    #[async_trait]
    impl StatsStorage for MockStorage {
        async fn get_email_counts(
            &self,
            _account_id: &AccountId,
            _start: Option<DateTime<Utc>>,
            _end: DateTime<Utc>,
        ) -> StatsResult<EmailStats> {
            Ok(EmailStats::default())
        }

        async fn get_ai_usage(
            &self,
            _account_id: &AccountId,
            _start: Option<DateTime<Utc>>,
            _end: DateTime<Utc>,
        ) -> StatsResult<AiStats> {
            Ok(AiStats::default())
        }

        async fn get_session_data(
            &self,
            _account_id: &AccountId,
            _start: Option<DateTime<Utc>>,
            _end: DateTime<Utc>,
        ) -> StatsResult<ProductivityStats> {
            Ok(ProductivityStats::default())
        }

        async fn get_top_correspondents(
            &self,
            _account_id: &AccountId,
            _start: Option<DateTime<Utc>>,
            _limit: usize,
        ) -> StatsResult<Vec<TopCorrespondent>> {
            Ok(Vec::new())
        }

        async fn get_hourly_distribution(
            &self,
            _account_id: &AccountId,
            _start: Option<DateTime<Utc>>,
        ) -> StatsResult<Vec<BusiestHour>> {
            Ok(Vec::new())
        }

        async fn get_daily_activity(
            &self,
            _account_id: &AccountId,
            _start: Option<DateTime<Utc>>,
            _end: DateTime<Utc>,
        ) -> StatsResult<Vec<DailyActivity>> {
            Ok(Vec::new())
        }

        async fn record_event(
            &self,
            _account_id: &AccountId,
            _event: StatsEvent,
        ) -> StatsResult<()> {
            Ok(())
        }
    }
}
