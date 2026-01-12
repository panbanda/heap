//! Screener service for managing unknown sender triage.
//!
//! The screener helps users manage emails from unknown senders by:
//! - Queuing new senders for review
//! - Applying AI analysis to suggest actions
//! - Maintaining allow/block rules for automatic decisions
//! - Learning from user decisions over time

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use thiserror::Error;

use crate::domain::{
    AccountId, EmailId, RuleType, ScreenerAction, ScreenerEntry, ScreenerRule, ScreenerStatus,
    SenderAnalysis, SenderType,
};

/// Errors that can occur during screener operations.
#[derive(Debug, Error)]
pub enum ScreenerError {
    /// Entry not found.
    #[error("screener entry not found: {0}")]
    NotFound(String),

    /// Rule not found.
    #[error("screener rule not found: {0}")]
    RuleNotFound(String),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),

    /// AI analysis error.
    #[error("AI analysis error: {0}")]
    AiError(String),

    /// Invalid operation.
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type for screener operations.
pub type ScreenerResult<T> = Result<T, ScreenerError>;

/// Storage trait for screener persistence.
#[async_trait]
pub trait ScreenerStorage: Send + Sync {
    /// Gets all pending entries for an account.
    async fn get_pending_entries(
        &self,
        account_id: &AccountId,
    ) -> ScreenerResult<Vec<ScreenerEntry>>;

    /// Gets an entry by ID.
    async fn get_entry(&self, id: &str) -> ScreenerResult<Option<ScreenerEntry>>;

    /// Gets an entry by sender email.
    async fn get_entry_by_email(
        &self,
        account_id: &AccountId,
        email: &str,
    ) -> ScreenerResult<Option<ScreenerEntry>>;

    /// Saves an entry.
    async fn save_entry(&self, entry: &ScreenerEntry) -> ScreenerResult<()>;

    /// Deletes an entry.
    async fn delete_entry(&self, id: &str) -> ScreenerResult<()>;

    /// Gets all rules for an account.
    async fn get_rules(&self, account_id: &AccountId) -> ScreenerResult<Vec<ScreenerRule>>;

    /// Gets a rule by ID.
    async fn get_rule(&self, id: &str) -> ScreenerResult<Option<ScreenerRule>>;

    /// Saves a rule.
    async fn save_rule(&self, rule: &ScreenerRule) -> ScreenerResult<()>;

    /// Deletes a rule.
    async fn delete_rule(&self, id: &str) -> ScreenerResult<()>;

    /// Counts entries by status.
    async fn count_by_status(
        &self,
        account_id: &AccountId,
        status: ScreenerStatus,
    ) -> ScreenerResult<u32>;
}

/// Filter for querying screener entries.
#[derive(Debug, Clone, Default)]
pub struct ScreenerFilter {
    /// Filter by status.
    pub status: Option<ScreenerStatus>,
    /// Filter by sender type.
    pub sender_type: Option<SenderType>,
    /// Filter by suggested action.
    pub suggested_action: Option<ScreenerAction>,
    /// Search in sender email/name.
    pub search: Option<String>,
    /// Maximum entries to return.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

impl ScreenerFilter {
    /// Creates a filter for pending entries.
    pub fn pending() -> Self {
        Self {
            status: Some(ScreenerStatus::Pending),
            ..Default::default()
        }
    }

    /// Creates a filter for a specific sender type.
    pub fn by_type(sender_type: SenderType) -> Self {
        Self {
            sender_type: Some(sender_type),
            ..Default::default()
        }
    }

    /// Adds a search term.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Limits results.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Tests if an entry matches this filter.
    pub fn matches(&self, entry: &ScreenerEntry) -> bool {
        if let Some(status) = self.status {
            if entry.status != status {
                return false;
            }
        }

        if let Some(sender_type) = self.sender_type {
            if let Some(ref analysis) = entry.ai_analysis {
                if analysis.likely_type != sender_type {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(action) = self.suggested_action {
            if let Some(ref analysis) = entry.ai_analysis {
                if analysis.suggested_action != action {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref search) = self.search {
            let search_lower = search.to_lowercase();
            let email_match = entry.sender_email.to_lowercase().contains(&search_lower);
            let name_match = entry
                .sender_name
                .as_ref()
                .map(|n| n.to_lowercase().contains(&search_lower))
                .unwrap_or(false);
            if !email_match && !name_match {
                return false;
            }
        }

        true
    }
}

/// Statistics for the screener queue.
#[derive(Debug, Clone, Default)]
pub struct ScreenerStats {
    /// Total pending entries.
    pub pending: u32,
    /// Total approved senders.
    pub approved: u32,
    /// Total rejected senders.
    pub rejected: u32,
    /// Entries by sender type.
    pub by_type: HashMap<SenderType, u32>,
}

/// Service for managing the email screener.
pub struct ScreenerService<S: ScreenerStorage> {
    storage: S,
    account_id: AccountId,
}

impl<S: ScreenerStorage> ScreenerService<S> {
    /// Creates a new screener service.
    pub fn new(storage: S, account_id: AccountId) -> Self {
        Self {
            storage,
            account_id,
        }
    }

    /// Returns the account ID.
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    /// Gets all pending entries.
    pub async fn get_pending(&self) -> ScreenerResult<Vec<ScreenerEntry>> {
        self.storage.get_pending_entries(&self.account_id).await
    }

    /// Gets entries matching a filter.
    pub async fn get_entries(&self, filter: &ScreenerFilter) -> ScreenerResult<Vec<ScreenerEntry>> {
        let entries = self.storage.get_pending_entries(&self.account_id).await?;
        let filtered: Vec<ScreenerEntry> =
            entries.into_iter().filter(|e| filter.matches(e)).collect();

        let offset = filter.offset.unwrap_or(0) as usize;
        let limit = filter.limit.unwrap_or(u32::MAX) as usize;

        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    /// Gets an entry by ID.
    pub async fn get_entry(&self, id: &str) -> ScreenerResult<ScreenerEntry> {
        self.storage
            .get_entry(id)
            .await?
            .ok_or_else(|| ScreenerError::NotFound(id.to_string()))
    }

    /// Checks if a sender is known (approved or rejected).
    pub async fn is_known_sender(&self, email: &str) -> ScreenerResult<Option<ScreenerStatus>> {
        if let Some(entry) = self
            .storage
            .get_entry_by_email(&self.account_id, email)
            .await?
        {
            if entry.status != ScreenerStatus::Pending {
                return Ok(Some(entry.status));
            }
        }

        // Check rules
        let rules = self.storage.get_rules(&self.account_id).await?;
        for rule in rules {
            if self.rule_matches(&rule, email) {
                return Ok(Some(match rule.action {
                    ScreenerAction::Approve => ScreenerStatus::Approved,
                    ScreenerAction::Reject => ScreenerStatus::Rejected,
                    ScreenerAction::Review => ScreenerStatus::Pending,
                }));
            }
        }

        Ok(None)
    }

    /// Adds a new sender to the screener queue.
    pub async fn add_sender(
        &self,
        email: &str,
        name: Option<&str>,
        first_email_id: Option<EmailId>,
    ) -> ScreenerResult<ScreenerEntry> {
        // Check if already exists
        if let Some(existing) = self
            .storage
            .get_entry_by_email(&self.account_id, email)
            .await?
        {
            return Ok(existing);
        }

        // Check rules for automatic decision
        let rules = self.storage.get_rules(&self.account_id).await?;
        let auto_status = rules.iter().find_map(|rule| {
            if self.rule_matches(rule, email) {
                Some(match rule.action {
                    ScreenerAction::Approve => ScreenerStatus::Approved,
                    ScreenerAction::Reject => ScreenerStatus::Rejected,
                    ScreenerAction::Review => ScreenerStatus::Pending,
                })
            } else {
                None
            }
        });

        let entry = ScreenerEntry {
            id: format!("scr-{}", uuid::Uuid::new_v4()),
            sender_email: email.to_string(),
            sender_name: name.map(String::from),
            first_email_id,
            status: auto_status.unwrap_or(ScreenerStatus::Pending),
            ai_analysis: None,
            decided_at: if auto_status.is_some() {
                Some(Utc::now())
            } else {
                None
            },
            created_at: Utc::now(),
        };

        self.storage.save_entry(&entry).await?;
        Ok(entry)
    }

    /// Approves a sender.
    pub async fn approve(&self, id: &str) -> ScreenerResult<ScreenerEntry> {
        let mut entry = self.get_entry(id).await?;
        entry.status = ScreenerStatus::Approved;
        entry.decided_at = Some(Utc::now());
        self.storage.save_entry(&entry).await?;
        Ok(entry)
    }

    /// Rejects a sender.
    pub async fn reject(&self, id: &str) -> ScreenerResult<ScreenerEntry> {
        let mut entry = self.get_entry(id).await?;
        entry.status = ScreenerStatus::Rejected;
        entry.decided_at = Some(Utc::now());
        self.storage.save_entry(&entry).await?;
        Ok(entry)
    }

    /// Bulk approves multiple entries.
    pub async fn approve_bulk(&self, ids: &[&str]) -> ScreenerResult<u32> {
        let mut count = 0;
        for id in ids {
            if self.approve(id).await.is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Bulk rejects multiple entries.
    pub async fn reject_bulk(&self, ids: &[&str]) -> ScreenerResult<u32> {
        let mut count = 0;
        for id in ids {
            if self.reject(id).await.is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Sets AI analysis for an entry.
    pub async fn set_analysis(&self, id: &str, analysis: SenderAnalysis) -> ScreenerResult<()> {
        let mut entry = self.get_entry(id).await?;
        entry.ai_analysis = Some(analysis);
        self.storage.save_entry(&entry).await
    }

    /// Gets all rules.
    pub async fn get_rules(&self) -> ScreenerResult<Vec<ScreenerRule>> {
        self.storage.get_rules(&self.account_id).await
    }

    /// Adds a domain allow rule.
    pub async fn allow_domain(&self, domain: &str) -> ScreenerResult<ScreenerRule> {
        let rule = ScreenerRule {
            id: format!("rule-{}", uuid::Uuid::new_v4()),
            rule_type: RuleType::DomainAllow,
            pattern: domain.to_lowercase(),
            action: ScreenerAction::Approve,
            created_at: Utc::now(),
        };
        self.storage.save_rule(&rule).await?;
        Ok(rule)
    }

    /// Adds a domain block rule.
    pub async fn block_domain(&self, domain: &str) -> ScreenerResult<ScreenerRule> {
        let rule = ScreenerRule {
            id: format!("rule-{}", uuid::Uuid::new_v4()),
            rule_type: RuleType::DomainBlock,
            pattern: domain.to_lowercase(),
            action: ScreenerAction::Reject,
            created_at: Utc::now(),
        };
        self.storage.save_rule(&rule).await?;
        Ok(rule)
    }

    /// Deletes a rule.
    pub async fn delete_rule(&self, id: &str) -> ScreenerResult<()> {
        self.storage.delete_rule(id).await
    }

    /// Gets screener statistics.
    pub async fn get_stats(&self) -> ScreenerResult<ScreenerStats> {
        let pending = self
            .storage
            .count_by_status(&self.account_id, ScreenerStatus::Pending)
            .await?;
        let approved = self
            .storage
            .count_by_status(&self.account_id, ScreenerStatus::Approved)
            .await?;
        let rejected = self
            .storage
            .count_by_status(&self.account_id, ScreenerStatus::Rejected)
            .await?;

        // Count by sender type from pending entries
        let entries = self.get_pending().await?;
        let mut by_type = HashMap::new();
        for entry in entries {
            if let Some(ref analysis) = entry.ai_analysis {
                *by_type.entry(analysis.likely_type).or_insert(0) += 1;
            }
        }

        Ok(ScreenerStats {
            pending,
            approved,
            rejected,
            by_type,
        })
    }

    /// Checks if a rule matches an email address.
    fn rule_matches(&self, rule: &ScreenerRule, email: &str) -> bool {
        let email_lower = email.to_lowercase();
        let pattern = &rule.pattern;

        match rule.rule_type {
            RuleType::DomainAllow | RuleType::DomainBlock => {
                email_lower.ends_with(&format!("@{}", pattern))
                    || email_lower.ends_with(&format!(".{}", pattern))
            }
            RuleType::Pattern => {
                // Simple glob matching
                if pattern.contains('*') {
                    let parts: Vec<&str> = pattern.split('*').collect();
                    let mut pos = 0;
                    for part in parts {
                        if part.is_empty() {
                            continue;
                        }
                        if let Some(idx) = email_lower[pos..].find(part) {
                            pos += idx + part.len();
                        } else {
                            return false;
                        }
                    }
                    true
                } else {
                    email_lower.contains(pattern)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockStorage {
        entries: Mutex<Vec<ScreenerEntry>>,
        rules: Mutex<Vec<ScreenerRule>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
                rules: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ScreenerStorage for MockStorage {
        async fn get_pending_entries(
            &self,
            _account_id: &AccountId,
        ) -> ScreenerResult<Vec<ScreenerEntry>> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.status == ScreenerStatus::Pending)
                .cloned()
                .collect())
        }

        async fn get_entry(&self, id: &str) -> ScreenerResult<Option<ScreenerEntry>> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }

        async fn get_entry_by_email(
            &self,
            _account_id: &AccountId,
            email: &str,
        ) -> ScreenerResult<Option<ScreenerEntry>> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.sender_email == email)
                .cloned())
        }

        async fn save_entry(&self, entry: &ScreenerEntry) -> ScreenerResult<()> {
            let mut entries = self.entries.lock().unwrap();
            entries.retain(|e| e.id != entry.id);
            entries.push(entry.clone());
            Ok(())
        }

        async fn delete_entry(&self, id: &str) -> ScreenerResult<()> {
            self.entries.lock().unwrap().retain(|e| e.id != id);
            Ok(())
        }

        async fn get_rules(&self, _account_id: &AccountId) -> ScreenerResult<Vec<ScreenerRule>> {
            Ok(self.rules.lock().unwrap().clone())
        }

        async fn get_rule(&self, id: &str) -> ScreenerResult<Option<ScreenerRule>> {
            Ok(self
                .rules
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.id == id)
                .cloned())
        }

        async fn save_rule(&self, rule: &ScreenerRule) -> ScreenerResult<()> {
            let mut rules = self.rules.lock().unwrap();
            rules.retain(|r| r.id != rule.id);
            rules.push(rule.clone());
            Ok(())
        }

        async fn delete_rule(&self, id: &str) -> ScreenerResult<()> {
            self.rules.lock().unwrap().retain(|r| r.id != id);
            Ok(())
        }

        async fn count_by_status(
            &self,
            _account_id: &AccountId,
            status: ScreenerStatus,
        ) -> ScreenerResult<u32> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.status == status)
                .count() as u32)
        }
    }

    #[tokio::test]
    async fn add_and_approve_sender() {
        let storage = MockStorage::new();
        let service = ScreenerService::new(storage, AccountId::from("test"));

        let entry = service
            .add_sender("new@example.com", Some("New Person"), None)
            .await
            .unwrap();
        assert_eq!(entry.status, ScreenerStatus::Pending);

        let approved = service.approve(&entry.id).await.unwrap();
        assert_eq!(approved.status, ScreenerStatus::Approved);
        assert!(approved.decided_at.is_some());
    }

    #[tokio::test]
    async fn domain_rules() {
        let storage = MockStorage::new();
        let service = ScreenerService::new(storage, AccountId::from("test"));

        service.allow_domain("trusted.com").await.unwrap();

        // New sender from trusted domain should be auto-approved
        let entry = service
            .add_sender("user@trusted.com", None, None)
            .await
            .unwrap();
        assert_eq!(entry.status, ScreenerStatus::Approved);
    }

    #[tokio::test]
    async fn filter_matching() {
        let filter = ScreenerFilter::pending().search("john");

        let entry = ScreenerEntry {
            id: "1".to_string(),
            sender_email: "john@example.com".to_string(),
            sender_name: None,
            first_email_id: None,
            status: ScreenerStatus::Pending,
            ai_analysis: None,
            decided_at: None,
            created_at: Utc::now(),
        };
        assert!(filter.matches(&entry));

        let approved_entry = ScreenerEntry {
            status: ScreenerStatus::Approved,
            ..entry.clone()
        };
        assert!(!filter.matches(&approved_entry));
    }
}
