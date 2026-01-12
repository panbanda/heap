//! Telemetry service for local usage statistics.
//!
//! Collects and aggregates usage data locally:
//! - Email metrics (received, sent, archived, trashed)
//! - Productivity metrics (response time, sessions, time in app)
//! - AI metrics (summaries, drafts, searches, token usage)
//!
//! All data stays local and can be exported or purged by the user.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::AccountId;

/// Types of telemetry events that can be recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Email received.
    EmailReceived,
    /// Email sent.
    EmailSent,
    /// Email archived.
    EmailArchived,
    /// Email trashed.
    EmailTrashed,
    /// Email read.
    EmailRead,
    /// Email starred.
    EmailStarred,
    /// Thread opened.
    ThreadOpened,
    /// Search performed.
    SearchPerformed,
    /// AI summary generated.
    AiSummary,
    /// AI draft generated.
    AiDraft,
    /// AI semantic search.
    AiSearch,
    /// Session started.
    SessionStart,
    /// Session ended.
    SessionEnd,
    /// App focused.
    AppFocused,
    /// App unfocused.
    AppUnfocused,
}

impl EventType {
    /// Returns the string representation for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::EmailReceived => "email_received",
            EventType::EmailSent => "email_sent",
            EventType::EmailArchived => "email_archived",
            EventType::EmailTrashed => "email_trashed",
            EventType::EmailRead => "email_read",
            EventType::EmailStarred => "email_starred",
            EventType::ThreadOpened => "thread_opened",
            EventType::SearchPerformed => "search_performed",
            EventType::AiSummary => "ai_summary",
            EventType::AiDraft => "ai_draft",
            EventType::AiSearch => "ai_search",
            EventType::SessionStart => "session_start",
            EventType::SessionEnd => "session_end",
            EventType::AppFocused => "app_focused",
            EventType::AppUnfocused => "app_unfocused",
        }
    }
}

/// Payload for a telemetry event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventPayload {
    /// Associated account ID, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Token count for AI operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u32>,
    /// Duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl EventPayload {
    /// Creates a new empty payload.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a payload with an account ID.
    pub fn with_account(account_id: &AccountId) -> Self {
        Self {
            account_id: Some(account_id.to_string()),
            ..Default::default()
        }
    }

    /// Sets the token count.
    pub fn tokens(mut self, tokens: u32) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Sets the duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self
    }

    /// Adds metadata.
    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }
}

/// A recorded telemetry event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Event type.
    pub event_type: EventType,
    /// Event payload.
    pub payload: EventPayload,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

impl TelemetryEvent {
    /// Creates a new event.
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            payload: EventPayload::default(),
            timestamp: Utc::now(),
        }
    }

    /// Creates a new event with payload.
    pub fn with_payload(event_type: EventType, payload: EventPayload) -> Self {
        Self {
            event_type,
            payload,
            timestamp: Utc::now(),
        }
    }
}

/// Aggregated daily statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyStats {
    /// Date of the stats.
    pub date: NaiveDate,
    /// Account ID (None for global stats).
    pub account_id: Option<String>,
    /// Emails received.
    pub emails_received: u32,
    /// Emails sent.
    pub emails_sent: u32,
    /// Emails archived.
    pub emails_archived: u32,
    /// Emails trashed.
    pub emails_trashed: u32,
    /// Time spent in app (seconds).
    pub time_in_app_seconds: u32,
    /// Number of sessions.
    pub sessions: u32,
    /// AI summaries generated.
    pub ai_summaries: u32,
    /// AI drafts generated.
    pub ai_drafts: u32,
    /// AI searches performed.
    pub ai_searches: u32,
    /// Total AI tokens used.
    pub ai_tokens_used: u32,
}

impl DailyStats {
    /// Creates new stats for today.
    pub fn today() -> Self {
        Self {
            date: Utc::now().date_naive(),
            ..Default::default()
        }
    }

    /// Creates stats for a specific date.
    pub fn for_date(date: NaiveDate) -> Self {
        Self {
            date,
            ..Default::default()
        }
    }

    /// Merges another stats object into this one.
    pub fn merge(&mut self, other: &DailyStats) {
        self.emails_received += other.emails_received;
        self.emails_sent += other.emails_sent;
        self.emails_archived += other.emails_archived;
        self.emails_trashed += other.emails_trashed;
        self.time_in_app_seconds += other.time_in_app_seconds;
        self.sessions += other.sessions;
        self.ai_summaries += other.ai_summaries;
        self.ai_drafts += other.ai_drafts;
        self.ai_searches += other.ai_searches;
        self.ai_tokens_used += other.ai_tokens_used;
    }
}

/// Time range for statistics queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsTimeRange {
    /// Today only.
    Today,
    /// Last 7 days.
    Week,
    /// Last 30 days.
    Month,
    /// Last 90 days.
    Quarter,
    /// All time.
    AllTime,
    /// Custom range.
    Custom { start: NaiveDate, end: NaiveDate },
}

impl StatsTimeRange {
    /// Returns the start date for this range.
    pub fn start_date(&self) -> Option<NaiveDate> {
        let today = Utc::now().date_naive();
        match self {
            StatsTimeRange::Today => Some(today),
            StatsTimeRange::Week => Some(today - chrono::Duration::days(7)),
            StatsTimeRange::Month => Some(today - chrono::Duration::days(30)),
            StatsTimeRange::Quarter => Some(today - chrono::Duration::days(90)),
            StatsTimeRange::AllTime => None,
            StatsTimeRange::Custom { start, .. } => Some(*start),
        }
    }

    /// Returns the end date for this range.
    pub fn end_date(&self) -> NaiveDate {
        match self {
            StatsTimeRange::Custom { end, .. } => *end,
            _ => Utc::now().date_naive(),
        }
    }
}

/// Aggregated statistics for a time period.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedStats {
    /// Time range.
    pub range: String,
    /// Total emails received.
    pub total_received: u32,
    /// Total emails sent.
    pub total_sent: u32,
    /// Total emails archived.
    pub total_archived: u32,
    /// Total emails trashed.
    pub total_trashed: u32,
    /// Total time in app (seconds).
    pub total_time_seconds: u32,
    /// Total sessions.
    pub total_sessions: u32,
    /// Total AI summaries.
    pub total_ai_summaries: u32,
    /// Total AI drafts.
    pub total_ai_drafts: u32,
    /// Total AI searches.
    pub total_ai_searches: u32,
    /// Total AI tokens.
    pub total_ai_tokens: u32,
    /// Average emails per day.
    pub avg_emails_per_day: f32,
    /// Average session duration (seconds).
    pub avg_session_duration: f32,
    /// Top correspondents.
    pub top_correspondents: Vec<(String, u32)>,
    /// Busiest hours (0-23 -> count).
    pub busiest_hours: Vec<(u8, u32)>,
}

impl AggregatedStats {
    /// Creates stats from daily stats.
    pub fn from_daily(daily: &[DailyStats], range_name: &str) -> Self {
        let mut result = Self {
            range: range_name.to_string(),
            ..Default::default()
        };

        for day in daily {
            result.total_received += day.emails_received;
            result.total_sent += day.emails_sent;
            result.total_archived += day.emails_archived;
            result.total_trashed += day.emails_trashed;
            result.total_time_seconds += day.time_in_app_seconds;
            result.total_sessions += day.sessions;
            result.total_ai_summaries += day.ai_summaries;
            result.total_ai_drafts += day.ai_drafts;
            result.total_ai_searches += day.ai_searches;
            result.total_ai_tokens += day.ai_tokens_used;
        }

        let days = daily.len().max(1) as f32;
        result.avg_emails_per_day = (result.total_received + result.total_sent) as f32 / days;

        if result.total_sessions > 0 {
            result.avg_session_duration =
                result.total_time_seconds as f32 / result.total_sessions as f32;
        }

        result
    }
}

/// Trait for telemetry storage backend.
pub trait TelemetryStorage: Send + Sync {
    /// Records a telemetry event.
    fn record_event(&self, event: &TelemetryEvent) -> Result<(), TelemetryError>;

    /// Gets daily stats for a date range.
    fn get_daily_stats(
        &self,
        start: Option<NaiveDate>,
        end: NaiveDate,
        account_id: Option<&AccountId>,
    ) -> Result<Vec<DailyStats>, TelemetryError>;

    /// Updates daily stats (upsert).
    fn update_daily_stats(&self, stats: &DailyStats) -> Result<(), TelemetryError>;

    /// Gets recent events.
    fn get_recent_events(&self, limit: usize) -> Result<Vec<TelemetryEvent>, TelemetryError>;

    /// Purges events older than a date.
    fn purge_events_before(&self, before: DateTime<Utc>) -> Result<u64, TelemetryError>;

    /// Purges all telemetry data.
    fn purge_all(&self) -> Result<(), TelemetryError>;
}

/// Telemetry error type.
#[derive(Debug, Clone)]
pub enum TelemetryError {
    /// Storage error.
    Storage(String),
    /// Serialization error.
    Serialization(String),
}

impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryError::Storage(msg) => write!(f, "Storage error: {}", msg),
            TelemetryError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for TelemetryError {}

/// Service for collecting and querying telemetry data.
pub struct TelemetryService<S: TelemetryStorage> {
    storage: S,
    /// Current session start time.
    session_start: Option<Instant>,
    /// Current session stats being accumulated.
    current_stats: DailyStats,
    /// Whether telemetry is enabled.
    enabled: bool,
    /// Retention period in days.
    retention_days: u32,
}

impl<S: TelemetryStorage> TelemetryService<S> {
    /// Creates a new telemetry service.
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            session_start: None,
            current_stats: DailyStats::today(),
            enabled: true,
            retention_days: 90,
        }
    }

    /// Sets whether telemetry is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether telemetry is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets the retention period in days.
    pub fn set_retention_days(&mut self, days: u32) {
        self.retention_days = days;
    }

    /// Records a telemetry event.
    pub fn record(&mut self, event_type: EventType, payload: EventPayload) {
        if !self.enabled {
            return;
        }

        let event = TelemetryEvent::with_payload(event_type, payload);

        // Update current day stats
        self.update_current_stats(&event);

        // Store the event
        let _ = self.storage.record_event(&event);
    }

    /// Records a simple event without payload.
    pub fn record_simple(&mut self, event_type: EventType) {
        self.record(event_type, EventPayload::default());
    }

    /// Records an event for an account.
    pub fn record_for_account(&mut self, event_type: EventType, account_id: &AccountId) {
        self.record(event_type, EventPayload::with_account(account_id));
    }

    /// Records an AI event with token count.
    pub fn record_ai(&mut self, event_type: EventType, tokens: u32) {
        self.record(event_type, EventPayload::new().tokens(tokens));
    }

    /// Starts a new session.
    pub fn start_session(&mut self) {
        self.session_start = Some(Instant::now());
        self.current_stats.sessions += 1;
        self.record_simple(EventType::SessionStart);
    }

    /// Ends the current session.
    pub fn end_session(&mut self) {
        if let Some(start) = self.session_start.take() {
            let duration = start.elapsed();
            self.current_stats.time_in_app_seconds += duration.as_secs() as u32;
            self.record(EventType::SessionEnd, EventPayload::new().duration(duration));
        }

        // Flush current stats
        let _ = self.flush_current_stats();
    }

    /// Records app focus change.
    pub fn record_focus(&mut self, focused: bool) {
        if focused {
            self.record_simple(EventType::AppFocused);
        } else {
            self.record_simple(EventType::AppUnfocused);
        }
    }

    /// Gets aggregated stats for a time range.
    pub fn get_stats(&self, range: StatsTimeRange) -> Result<AggregatedStats, TelemetryError> {
        let daily = self
            .storage
            .get_daily_stats(range.start_date(), range.end_date(), None)?;

        let range_name = match range {
            StatsTimeRange::Today => "Today",
            StatsTimeRange::Week => "Last 7 days",
            StatsTimeRange::Month => "Last 30 days",
            StatsTimeRange::Quarter => "Last 90 days",
            StatsTimeRange::AllTime => "All time",
            StatsTimeRange::Custom { .. } => "Custom",
        };

        Ok(AggregatedStats::from_daily(&daily, range_name))
    }

    /// Gets stats for a specific account.
    pub fn get_account_stats(
        &self,
        account_id: &AccountId,
        range: StatsTimeRange,
    ) -> Result<AggregatedStats, TelemetryError> {
        let daily =
            self.storage
                .get_daily_stats(range.start_date(), range.end_date(), Some(account_id))?;

        Ok(AggregatedStats::from_daily(&daily, "Account"))
    }

    /// Exports stats to JSON.
    pub fn export_json(&self, range: StatsTimeRange) -> Result<String, TelemetryError> {
        let stats = self.get_stats(range)?;
        serde_json::to_string_pretty(&stats)
            .map_err(|e| TelemetryError::Serialization(e.to_string()))
    }

    /// Exports stats to CSV format.
    pub fn export_csv(&self, range: StatsTimeRange) -> Result<String, TelemetryError> {
        let daily = self
            .storage
            .get_daily_stats(range.start_date(), range.end_date(), None)?;

        let mut csv = String::from(
            "date,emails_received,emails_sent,emails_archived,emails_trashed,time_seconds,sessions,ai_summaries,ai_drafts,ai_searches,ai_tokens\n"
        );

        for day in daily {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                day.date,
                day.emails_received,
                day.emails_sent,
                day.emails_archived,
                day.emails_trashed,
                day.time_in_app_seconds,
                day.sessions,
                day.ai_summaries,
                day.ai_drafts,
                day.ai_searches,
                day.ai_tokens_used
            ));
        }

        Ok(csv)
    }

    /// Purges old telemetry data based on retention settings.
    pub fn purge_old_data(&self) -> Result<u64, TelemetryError> {
        let cutoff = Utc::now() - chrono::Duration::days(self.retention_days as i64);
        self.storage.purge_events_before(cutoff)
    }

    /// Purges all telemetry data.
    pub fn purge_all(&self) -> Result<(), TelemetryError> {
        self.storage.purge_all()
    }

    fn update_current_stats(&mut self, event: &TelemetryEvent) {
        // Check if we need to roll over to a new day
        let today = Utc::now().date_naive();
        if self.current_stats.date != today {
            let _ = self.flush_current_stats();
            self.current_stats = DailyStats::for_date(today);
        }

        match event.event_type {
            EventType::EmailReceived => self.current_stats.emails_received += 1,
            EventType::EmailSent => self.current_stats.emails_sent += 1,
            EventType::EmailArchived => self.current_stats.emails_archived += 1,
            EventType::EmailTrashed => self.current_stats.emails_trashed += 1,
            EventType::AiSummary => {
                self.current_stats.ai_summaries += 1;
                if let Some(tokens) = event.payload.tokens {
                    self.current_stats.ai_tokens_used += tokens;
                }
            }
            EventType::AiDraft => {
                self.current_stats.ai_drafts += 1;
                if let Some(tokens) = event.payload.tokens {
                    self.current_stats.ai_tokens_used += tokens;
                }
            }
            EventType::AiSearch => {
                self.current_stats.ai_searches += 1;
                if let Some(tokens) = event.payload.tokens {
                    self.current_stats.ai_tokens_used += tokens;
                }
            }
            _ => {}
        }
    }

    fn flush_current_stats(&self) -> Result<(), TelemetryError> {
        self.storage.update_daily_stats(&self.current_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;

    /// Mock storage for testing.
    struct MockStorage {
        events: RwLock<Vec<TelemetryEvent>>,
        daily_stats: RwLock<Vec<DailyStats>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                events: RwLock::new(Vec::new()),
                daily_stats: RwLock::new(Vec::new()),
            }
        }
    }

    impl TelemetryStorage for MockStorage {
        fn record_event(&self, event: &TelemetryEvent) -> Result<(), TelemetryError> {
            self.events.write().unwrap().push(event.clone());
            Ok(())
        }

        fn get_daily_stats(
            &self,
            _start: Option<NaiveDate>,
            _end: NaiveDate,
            _account_id: Option<&AccountId>,
        ) -> Result<Vec<DailyStats>, TelemetryError> {
            Ok(self.daily_stats.read().unwrap().clone())
        }

        fn update_daily_stats(&self, stats: &DailyStats) -> Result<(), TelemetryError> {
            let mut daily = self.daily_stats.write().unwrap();
            if let Some(existing) = daily.iter_mut().find(|s| s.date == stats.date) {
                *existing = stats.clone();
            } else {
                daily.push(stats.clone());
            }
            Ok(())
        }

        fn get_recent_events(&self, limit: usize) -> Result<Vec<TelemetryEvent>, TelemetryError> {
            let events = self.events.read().unwrap();
            Ok(events.iter().rev().take(limit).cloned().collect())
        }

        fn purge_events_before(&self, _before: DateTime<Utc>) -> Result<u64, TelemetryError> {
            Ok(0)
        }

        fn purge_all(&self) -> Result<(), TelemetryError> {
            self.events.write().unwrap().clear();
            self.daily_stats.write().unwrap().clear();
            Ok(())
        }
    }

    #[test]
    fn event_type_as_str() {
        assert_eq!(EventType::EmailReceived.as_str(), "email_received");
        assert_eq!(EventType::AiSummary.as_str(), "ai_summary");
    }

    #[test]
    fn event_payload_builder() {
        let payload = EventPayload::new()
            .tokens(100)
            .duration(Duration::from_secs(5))
            .meta("key", "value");

        assert_eq!(payload.tokens, Some(100));
        assert_eq!(payload.duration_ms, Some(5000));
        assert!(payload.metadata.unwrap().contains_key("key"));
    }

    #[test]
    fn daily_stats_merge() {
        let mut stats1 = DailyStats::today();
        stats1.emails_received = 10;
        stats1.ai_tokens_used = 100;

        let mut stats2 = DailyStats::today();
        stats2.emails_received = 5;
        stats2.ai_tokens_used = 50;

        stats1.merge(&stats2);

        assert_eq!(stats1.emails_received, 15);
        assert_eq!(stats1.ai_tokens_used, 150);
    }

    #[test]
    fn stats_time_range_dates() {
        let today = Utc::now().date_naive();

        assert_eq!(StatsTimeRange::Today.start_date(), Some(today));
        assert_eq!(StatsTimeRange::AllTime.start_date(), None);

        let week_start = StatsTimeRange::Week.start_date().unwrap();
        assert!(week_start < today);
    }

    #[test]
    fn telemetry_service_records_events() {
        let storage = MockStorage::new();
        let mut service = TelemetryService::new(storage);

        service.record_simple(EventType::EmailReceived);
        service.record_simple(EventType::EmailSent);

        assert_eq!(service.current_stats.emails_received, 1);
        assert_eq!(service.current_stats.emails_sent, 1);
    }

    #[test]
    fn telemetry_service_disabled() {
        let storage = MockStorage::new();
        let mut service = TelemetryService::new(storage);

        service.set_enabled(false);
        service.record_simple(EventType::EmailReceived);

        assert_eq!(service.current_stats.emails_received, 0);
    }

    #[test]
    fn telemetry_service_ai_tokens() {
        let storage = MockStorage::new();
        let mut service = TelemetryService::new(storage);

        service.record_ai(EventType::AiSummary, 500);
        service.record_ai(EventType::AiDraft, 300);

        assert_eq!(service.current_stats.ai_summaries, 1);
        assert_eq!(service.current_stats.ai_drafts, 1);
        assert_eq!(service.current_stats.ai_tokens_used, 800);
    }

    #[test]
    fn telemetry_service_session() {
        let storage = MockStorage::new();
        let mut service = TelemetryService::new(storage);

        service.start_session();
        assert_eq!(service.current_stats.sessions, 1);
        assert!(service.session_start.is_some());

        service.end_session();
        assert!(service.session_start.is_none());
    }

    #[test]
    fn aggregated_stats_from_daily() {
        let daily = vec![
            DailyStats {
                emails_received: 10,
                emails_sent: 5,
                ai_tokens_used: 100,
                ..DailyStats::today()
            },
            DailyStats {
                emails_received: 20,
                emails_sent: 10,
                ai_tokens_used: 200,
                ..DailyStats::today()
            },
        ];

        let agg = AggregatedStats::from_daily(&daily, "Test");

        assert_eq!(agg.total_received, 30);
        assert_eq!(agg.total_sent, 15);
        assert_eq!(agg.total_ai_tokens, 300);
        assert_eq!(agg.avg_emails_per_day, 22.5); // (30 + 15) / 2
    }

    #[test]
    fn export_csv_format() {
        let storage = MockStorage::new();
        {
            let mut daily = storage.daily_stats.write().unwrap();
            daily.push(DailyStats {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                emails_received: 10,
                emails_sent: 5,
                ..Default::default()
            });
        }

        let service = TelemetryService::new(storage);
        let csv = service.export_csv(StatsTimeRange::Week).unwrap();

        assert!(csv.contains("date,emails_received"));
        assert!(csv.contains("2025-01-01,10,5"));
    }
}
