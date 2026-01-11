//! Search service for combined full-text and semantic search.
//!
//! The [`SearchService`] orchestrates search across multiple backends:
//! - FTS5-based full-text search for exact keyword matching
//! - Semantic search via embeddings for conceptual similarity
//! - Faceted filtering by folder, date range, sender, attachments

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::domain::{AccountId, EmailId, ThreadId};
use crate::services::{AiService, SearchResult};

/// Search query with filters and options.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// The search text (natural language or keywords).
    pub text: String,
    /// Accounts to search within.
    pub account_ids: Vec<AccountId>,
    /// Filter by folder/view.
    pub folder: Option<SearchFolder>,
    /// Filter by date range.
    pub date_range: Option<DateRange>,
    /// Filter by sender email.
    pub from: Option<String>,
    /// Filter by recipient email.
    pub to: Option<String>,
    /// Filter for emails with attachments.
    pub has_attachment: Option<bool>,
    /// Filter for unread only.
    pub is_unread: Option<bool>,
    /// Filter for starred only.
    pub is_starred: Option<bool>,
    /// Maximum results to return.
    pub limit: usize,
    /// Results to skip (pagination).
    pub offset: usize,
    /// Search mode.
    pub mode: SearchMode,
}

impl SearchQuery {
    /// Creates a new search query with text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: 50,
            ..Default::default()
        }
    }

    /// Sets the accounts to search within.
    pub fn with_accounts(mut self, accounts: Vec<AccountId>) -> Self {
        self.account_ids = accounts;
        self
    }

    /// Sets the folder filter.
    pub fn with_folder(mut self, folder: SearchFolder) -> Self {
        self.folder = Some(folder);
        self
    }

    /// Sets the date range filter.
    pub fn with_date_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.date_range = Some(DateRange { start, end });
        self
    }

    /// Sets the sender filter.
    pub fn with_from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Sets the recipient filter.
    pub fn with_to(mut self, to: impl Into<String>) -> Self {
        self.to = Some(to.into());
        self
    }

    /// Filters by attachment presence.
    pub fn with_attachment(mut self, has_attachment: bool) -> Self {
        self.has_attachment = Some(has_attachment);
        self
    }

    /// Sets the limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Sets the offset.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the search mode.
    pub fn with_mode(mut self, mode: SearchMode) -> Self {
        self.mode = mode;
        self
    }
}

/// Folder to filter search results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFolder {
    /// All mail.
    All,
    /// Inbox only.
    Inbox,
    /// Sent mail.
    Sent,
    /// Drafts.
    Drafts,
    /// Archive.
    Archive,
    /// Trash.
    Trash,
    /// Custom label.
    Label(String),
}

/// Date range filter.
#[derive(Debug, Clone)]
pub struct DateRange {
    /// Start of range (inclusive).
    pub start: DateTime<Utc>,
    /// End of range (inclusive).
    pub end: DateTime<Utc>,
}

/// Search execution mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchMode {
    /// Full-text search only (faster, exact matches).
    FullText,
    /// Semantic search only (AI-powered, conceptual).
    Semantic,
    /// Combine both, merging and ranking results.
    #[default]
    Hybrid,
}

/// Combined search result with source tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// ID of the matching email.
    pub email_id: EmailId,
    /// ID of the thread containing the email.
    pub thread_id: ThreadId,
    /// Email subject.
    pub subject: Option<String>,
    /// Preview snippet with highlights.
    pub snippet: String,
    /// Sender display.
    pub from: String,
    /// Email date.
    pub date: DateTime<Utc>,
    /// Whether the email is read.
    pub is_read: bool,
    /// Combined relevance score (0.0-1.0).
    pub score: f32,
    /// Where the match came from.
    pub source: SearchSource,
    /// Highlighted matching segments.
    pub highlights: Vec<String>,
}

/// Source of a search result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSource {
    /// From FTS5 full-text search.
    FullText,
    /// From semantic embedding search.
    Semantic,
    /// Appeared in both searches.
    Both,
}

/// Search result page.
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// Matching results.
    pub hits: Vec<SearchHit>,
    /// Total matches (before pagination).
    pub total: usize,
    /// Query that was executed.
    pub query: String,
    /// Time taken in milliseconds.
    pub took_ms: u64,
    /// Whether semantic search was used.
    pub used_semantic: bool,
}

/// Storage trait for search operations.
#[async_trait::async_trait]
pub trait SearchStorage: Send + Sync {
    /// Performs full-text search using FTS5.
    async fn fts_search(&self, query: &SearchQuery) -> Result<Vec<FtsHit>>;

    /// Looks up email metadata by IDs.
    async fn get_email_metadata(&self, ids: &[EmailId]) -> Result<Vec<EmailMetadata>>;

    /// Rebuilds the FTS index for an account.
    async fn rebuild_fts_index(&self, account_id: &AccountId) -> Result<()>;
}

/// Raw FTS hit from database.
#[derive(Debug, Clone)]
pub struct FtsHit {
    /// Email ID.
    pub email_id: EmailId,
    /// Thread ID.
    pub thread_id: ThreadId,
    /// FTS rank score.
    pub rank: f32,
    /// Matching snippet.
    pub snippet: String,
}

/// Email metadata for search results.
#[derive(Debug, Clone)]
pub struct EmailMetadata {
    /// Email ID.
    pub email_id: EmailId,
    /// Thread ID.
    pub thread_id: ThreadId,
    /// Subject.
    pub subject: Option<String>,
    /// Snippet.
    pub snippet: String,
    /// Sender display.
    pub from: String,
    /// Date.
    pub date: DateTime<Utc>,
    /// Read status.
    pub is_read: bool,
}

/// Search service settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    /// Whether semantic search is enabled.
    pub semantic_enabled: bool,
    /// Weight for FTS results in hybrid mode (0.0-1.0).
    pub fts_weight: f32,
    /// Weight for semantic results in hybrid mode (0.0-1.0).
    pub semantic_weight: f32,
    /// Minimum score threshold.
    pub min_score: f32,
    /// Default result limit.
    pub default_limit: usize,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            semantic_enabled: true,
            fts_weight: 0.6,
            semantic_weight: 0.4,
            min_score: 0.3,
            default_limit: 50,
        }
    }
}

/// Search service combining full-text and semantic search.
///
/// Provides a unified search interface that merges results from:
/// - SQLite FTS5 for fast keyword matching
/// - AI embeddings for semantic similarity
pub struct SearchService<S: SearchStorage> {
    /// Storage backend.
    storage: Arc<S>,
    /// AI service for semantic search.
    ai_service: Option<Arc<AiService>>,
    /// Search settings.
    settings: RwLock<SearchSettings>,
    /// Recent queries for suggestions.
    recent_queries: RwLock<Vec<String>>,
}

impl<S: SearchStorage> SearchService<S> {
    /// Creates a new search service.
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            ai_service: None,
            settings: RwLock::new(SearchSettings::default()),
            recent_queries: RwLock::new(Vec::new()),
        }
    }

    /// Sets the AI service for semantic search.
    pub fn with_ai_service(mut self, ai_service: Arc<AiService>) -> Self {
        self.ai_service = Some(ai_service);
        self
    }

    /// Updates search settings.
    pub async fn update_settings(&self, settings: SearchSettings) {
        let mut current = self.settings.write().await;
        *current = settings;
    }

    /// Returns current settings.
    pub async fn settings(&self) -> SearchSettings {
        self.settings.read().await.clone()
    }

    /// Executes a search query.
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let start = std::time::Instant::now();
        let settings = self.settings.read().await;

        // Track query for suggestions
        self.track_query(&query.text).await;

        let (fts_hits, semantic_hits) = match query.mode {
            SearchMode::FullText => {
                let fts = self.storage.fts_search(&query).await?;
                (fts, vec![])
            }
            SearchMode::Semantic => {
                if settings.semantic_enabled && self.ai_service.is_some() {
                    let semantic = self.semantic_search(&query).await?;
                    (vec![], semantic)
                } else {
                    // Fall back to FTS if semantic disabled
                    let fts = self.storage.fts_search(&query).await?;
                    (fts, vec![])
                }
            }
            SearchMode::Hybrid => {
                let fts_future = self.storage.fts_search(&query);
                let semantic_future = self.semantic_search(&query);

                // Execute in parallel if semantic is enabled
                if settings.semantic_enabled && self.ai_service.is_some() {
                    let (fts, semantic) = tokio::join!(fts_future, semantic_future);
                    (fts?, semantic.unwrap_or_default())
                } else {
                    (fts_future.await?, vec![])
                }
            }
        };

        let used_semantic = !semantic_hits.is_empty();

        // Merge and rank results
        let merged = self
            .merge_results(&fts_hits, &semantic_hits, &settings)
            .await?;

        // Apply pagination
        let total = merged.len();
        let hits: Vec<SearchHit> = merged
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        let took_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResults {
            hits,
            total,
            query: query.text,
            took_ms,
            used_semantic,
        })
    }

    /// Performs semantic search via AI service.
    async fn semantic_search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let ai_service = match &self.ai_service {
            Some(ai) => ai,
            None => return Ok(vec![]),
        };

        ai_service
            .semantic_search(&query.text, &query.account_ids)
            .await
    }

    /// Merges FTS and semantic results with ranking.
    async fn merge_results(
        &self,
        fts_hits: &[FtsHit],
        semantic_hits: &[SearchResult],
        settings: &SearchSettings,
    ) -> Result<Vec<SearchHit>> {
        // Collect all unique email IDs
        let mut email_ids: HashSet<EmailId> = HashSet::new();
        for hit in fts_hits {
            email_ids.insert(hit.email_id.clone());
        }
        for hit in semantic_hits {
            email_ids.insert(hit.email_id.clone());
        }

        if email_ids.is_empty() {
            return Ok(vec![]);
        }

        // Fetch metadata for all emails
        let ids: Vec<EmailId> = email_ids.into_iter().collect();
        let metadata = self.storage.get_email_metadata(&ids).await?;
        let metadata_map: std::collections::HashMap<EmailId, EmailMetadata> =
            metadata.into_iter().map(|m| (m.email_id.clone(), m)).collect();

        // Build scored results
        let mut results: Vec<SearchHit> = Vec::new();

        // Process FTS results
        let mut fts_map: std::collections::HashMap<EmailId, f32> = std::collections::HashMap::new();
        for hit in fts_hits {
            fts_map.insert(hit.email_id.clone(), hit.rank);
        }

        // Process semantic results
        let mut semantic_map: std::collections::HashMap<EmailId, f32> =
            std::collections::HashMap::new();
        for hit in semantic_hits {
            semantic_map.insert(hit.email_id.clone(), hit.relevance);
        }

        // Combine scores
        for (email_id, meta) in &metadata_map {
            let fts_score = fts_map.get(email_id).copied().unwrap_or(0.0);
            let semantic_score = semantic_map.get(email_id).copied().unwrap_or(0.0);

            let source = match (fts_score > 0.0, semantic_score > 0.0) {
                (true, true) => SearchSource::Both,
                (true, false) => SearchSource::FullText,
                (false, true) => SearchSource::Semantic,
                (false, false) => continue,
            };

            // Calculate combined score
            let combined_score = (fts_score * settings.fts_weight)
                + (semantic_score * settings.semantic_weight);

            if combined_score < settings.min_score {
                continue;
            }

            // Get snippet/highlights from FTS if available
            let snippet = fts_hits
                .iter()
                .find(|h| h.email_id == *email_id)
                .map(|h| h.snippet.clone())
                .unwrap_or_else(|| meta.snippet.clone());

            let highlights: Vec<String> = semantic_hits
                .iter()
                .find(|h| h.email_id == *email_id)
                .map(|h| h.highlights.clone())
                .unwrap_or_default();

            results.push(SearchHit {
                email_id: email_id.clone(),
                thread_id: meta.thread_id.clone(),
                subject: meta.subject.clone(),
                snippet,
                from: meta.from.clone(),
                date: meta.date,
                is_read: meta.is_read,
                score: combined_score,
                source,
                highlights,
            });
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Tracks a query for suggestions.
    async fn track_query(&self, query: &str) {
        if query.trim().is_empty() {
            return;
        }

        let mut recent = self.recent_queries.write().await;

        // Remove if already exists
        recent.retain(|q| q != query);

        // Add to front
        recent.insert(0, query.to_string());

        // Keep only recent 100
        recent.truncate(100);
    }

    /// Returns recent search queries for suggestions.
    pub async fn recent_queries(&self, limit: usize) -> Vec<String> {
        let recent = self.recent_queries.read().await;
        recent.iter().take(limit).cloned().collect()
    }

    /// Returns search suggestions based on prefix.
    pub async fn suggest(&self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let recent = self.recent_queries.read().await;

        recent
            .iter()
            .filter(|q| q.to_lowercase().starts_with(&prefix_lower))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Rebuilds the search index for an account.
    pub async fn rebuild_index(&self, account_id: &AccountId) -> Result<()> {
        self.storage.rebuild_fts_index(account_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_builder() {
        let query = SearchQuery::new("test query")
            .with_folder(SearchFolder::Inbox)
            .with_from("sender@example.com")
            .with_limit(20);

        assert_eq!(query.text, "test query");
        assert_eq!(query.folder, Some(SearchFolder::Inbox));
        assert_eq!(query.from, Some("sender@example.com".to_string()));
        assert_eq!(query.limit, 20);
    }

    #[test]
    fn search_mode_default() {
        let mode = SearchMode::default();
        assert_eq!(mode, SearchMode::Hybrid);
    }

    #[test]
    fn search_settings_default() {
        let settings = SearchSettings::default();
        assert!(settings.semantic_enabled);
        assert!(settings.fts_weight > 0.0);
        assert!(settings.semantic_weight > 0.0);
        assert_eq!(settings.default_limit, 50);
    }

    #[test]
    fn search_folder_variants() {
        assert_eq!(SearchFolder::Inbox, SearchFolder::Inbox);
        assert_ne!(SearchFolder::Inbox, SearchFolder::Sent);

        let label = SearchFolder::Label("custom".to_string());
        matches!(label, SearchFolder::Label(_));
    }

    #[test]
    fn search_source_serialization() {
        let source = SearchSource::Both;
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: SearchSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SearchSource::Both);
    }

    #[test]
    fn search_hit_serialization() {
        let hit = SearchHit {
            email_id: EmailId::from("email-1"),
            thread_id: ThreadId::from("thread-1"),
            subject: Some("Test Subject".to_string()),
            snippet: "Preview text...".to_string(),
            from: "sender@example.com".to_string(),
            date: Utc::now(),
            is_read: false,
            score: 0.85,
            source: SearchSource::FullText,
            highlights: vec!["matching".to_string()],
        };

        let json = serde_json::to_string(&hit).unwrap();
        assert!(json.contains("email-1"));
        assert!(json.contains("0.85"));
    }
}
