//! Business services layer.
//!
//! This module contains the core services that orchestrate business logic,
//! coordinating between providers, storage, and domain types.
//!
//! # Architecture
//!
//! Services sit between the application layer and the infrastructure layer:
//!
//! ```text
//! Application Layer (UI, Actions, Events)
//!          |
//!          v
//!    Services Layer  <-- You are here
//!          |
//!          v
//! Infrastructure (Providers, Storage)
//! ```
//!
//! # Services Overview
//!
//! - [`EmailService`]: Orchestrates email operations across providers and storage
//! - [`AiService`]: Manages AI provider interactions for summarization, drafts, and search
//! - [`SyncService`]: Handles synchronization between remote providers and local storage
//! - [`SearchService`]: Combined full-text and semantic search across emails
//! - [`ContactService`]: Manages contacts extracted from email interactions
//! - [`LabelService`]: Manages email labels and folders
//! - [`SnoozeService`]: Temporarily hides emails until a scheduled time
//! - [`TelemetryService`]: Local usage statistics and event tracking
//! - [`ScreenerService`]: Manages unknown sender triage and screening
//! - [`NotificationService`]: In-app and system notifications
//! - [`SmartViewService`]: AI-powered email classification into smart views
//! - [`StatsService`]: Usage statistics and metrics aggregation
//! - [`AccountService`]: Manages email account configuration and credentials
//! - [`ThreadService`]: Thread operations and metadata management

mod account_service;
mod ai_service;
mod contact_service;
mod email_service;
mod label_service;
mod notification_service;
mod screener_service;
mod search_service;
mod smart_view_service;
mod snooze_service;
mod stats_service;
mod sync_service;
mod telemetry_service;
mod thread_service;
mod undo_service;

pub use account_service::{
    AccountError, AccountService, AccountStats, AccountStorage, AccountUpdate,
    CreateAccountRequest, CredentialStore,
};
pub use ai_service::{
    AiService, AiSettings, Category, DraftSuggestion, SearchResult, Summary, SummarySettings,
};
pub use contact_service::{
    ContactError, ContactFilter, ContactService, ContactSort, ContactStats, ContactStorage,
};
pub use email_service::{Draft, EmailService, Pagination, ViewType};
pub use label_service::{LabelError, LabelService, LabelSort, LabelStorage};
pub use notification_service::{
    NotificationCategory, NotificationError, NotificationPriority, NotificationRequest,
    NotificationService, NotificationSettings, SentNotification,
};
pub use screener_service::{
    ScreenerError, ScreenerFilter, ScreenerService, ScreenerStats, ScreenerStorage,
};
pub use search_service::{
    DateRange, EmailMetadata, FtsHit, SearchFolder, SearchHit, SearchMode, SearchQuery,
    SearchResults, SearchService, SearchSettings, SearchSource, SearchStorage,
};
pub use smart_view_service::{
    Classification, ClassificationCriteria, ClassificationInput, SmartViewError, SmartViewService,
    SmartViewStorage, SmartViewType,
};
pub use snooze_service::{SnoozeDuration, SnoozeError, SnoozeService, SnoozeStorage, SnoozedItem};
pub use stats_service::{
    AiStats, BusiestHour, DailyActivity, EmailStats, ProductivityStats, StatsError, StatsEvent,
    StatsReport, StatsService, StatsStorage, TopCorrespondent,
};
pub use sync_service::{SyncResult, SyncService, SyncSettings, SyncStatus};
pub use telemetry_service::{
    AggregatedStats, DailyStats, EventPayload, EventType, StatsTimeRange, TelemetryError,
    TelemetryEvent, TelemetryService, TelemetryStorage,
};
pub use thread_service::{
    ThreadError, ThreadFilter, ThreadService, ThreadSort, ThreadStats, ThreadStorage,
};
pub use undo_service::{
    ActionBuilder, ActionResult, ActionState, ActionType, UndoService, UndoableAction,
};
