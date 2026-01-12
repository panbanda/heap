//! Smart View service for AI-powered email classification.
//!
//! Provides intelligent categorization of emails into smart views:
//! - Needs Reply: emails requiring user response
//! - Waiting For: sent emails awaiting replies
//! - Newsletters: promotional/bulk mail
//! - VIP: important contacts
//! - Follow Up: flagged for later action

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use thiserror::Error;

use crate::domain::{AccountId, ThreadId};

/// Types of smart views available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SmartViewType {
    /// Emails that need a reply from the user.
    NeedsReply,
    /// Emails where user is waiting for a response.
    WaitingFor,
    /// Newsletter/promotional emails.
    Newsletters,
    /// VIP/important contacts.
    Vip,
    /// Flagged for follow-up.
    FollowUp,
    /// Recently read but unactioned.
    RecentlyViewed,
    /// Emails with attachments.
    Attachments,
}

impl SmartViewType {
    /// Returns all smart view types.
    pub fn all() -> &'static [SmartViewType] {
        &[
            SmartViewType::NeedsReply,
            SmartViewType::WaitingFor,
            SmartViewType::Newsletters,
            SmartViewType::Vip,
            SmartViewType::FollowUp,
            SmartViewType::RecentlyViewed,
            SmartViewType::Attachments,
        ]
    }
}

/// Errors that can occur during smart view operations.
#[derive(Debug, Error)]
pub enum SmartViewError {
    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),

    /// AI classification error.
    #[error("classification error: {0}")]
    Classification(String),

    /// Thread not found.
    #[error("thread not found: {0}")]
    ThreadNotFound(String),
}

/// Result type for smart view operations.
pub type SmartViewResult<T> = Result<T, SmartViewError>;

/// Classification result for a thread.
#[derive(Debug, Clone)]
pub struct Classification {
    /// Thread that was classified.
    pub thread_id: ThreadId,
    /// View type assigned.
    pub view_type: SmartViewType,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Reason for classification.
    pub reason: String,
    /// When classification was made.
    pub classified_at: DateTime<Utc>,
    /// Whether this was manually assigned.
    pub manual: bool,
}

impl Classification {
    /// Creates an AI-generated classification.
    pub fn ai_classified(
        thread_id: ThreadId,
        view_type: SmartViewType,
        confidence: f32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            thread_id,
            view_type,
            confidence,
            reason: reason.into(),
            classified_at: Utc::now(),
            manual: false,
        }
    }

    /// Creates a manual classification.
    pub fn manual(
        thread_id: ThreadId,
        view_type: SmartViewType,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            thread_id,
            view_type,
            confidence: 1.0,
            reason: reason.into(),
            classified_at: Utc::now(),
            manual: true,
        }
    }
}

/// Input data for classification.
#[derive(Debug, Clone)]
pub struct ClassificationInput {
    /// Thread ID.
    pub thread_id: ThreadId,
    /// Subject line.
    pub subject: Option<String>,
    /// Snippet/preview text.
    pub snippet: String,
    /// Sender email.
    pub sender_email: String,
    /// Sender name.
    pub sender_name: Option<String>,
    /// Whether user is the last sender.
    pub user_was_last_sender: bool,
    /// Last message date.
    pub last_message_date: DateTime<Utc>,
    /// Whether the thread is read.
    pub is_read: bool,
    /// Whether the thread has attachments.
    pub has_attachments: bool,
    /// Number of messages in thread.
    pub message_count: u32,
    /// Labels on the thread.
    pub labels: Vec<String>,
}

/// Storage trait for smart view classifications.
#[async_trait]
pub trait SmartViewStorage: Send + Sync {
    /// Gets classifications for a thread.
    async fn get_classifications(
        &self,
        thread_id: &ThreadId,
    ) -> SmartViewResult<Vec<Classification>>;

    /// Gets all threads for a view type.
    async fn get_threads_for_view(
        &self,
        account_id: &AccountId,
        view_type: SmartViewType,
    ) -> SmartViewResult<Vec<Classification>>;

    /// Saves a classification.
    async fn save_classification(&self, classification: &Classification) -> SmartViewResult<()>;

    /// Removes classifications for a thread.
    async fn remove_classifications(&self, thread_id: &ThreadId) -> SmartViewResult<()>;

    /// Gets VIP contacts.
    async fn get_vip_contacts(&self, account_id: &AccountId) -> SmartViewResult<Vec<String>>;

    /// Adds a VIP contact.
    async fn add_vip_contact(&self, account_id: &AccountId, email: &str) -> SmartViewResult<()>;

    /// Removes a VIP contact.
    async fn remove_vip_contact(&self, account_id: &AccountId, email: &str) -> SmartViewResult<()>;
}

/// Criteria for classification.
#[derive(Debug, Clone)]
pub struct ClassificationCriteria {
    /// View type.
    pub view_type: SmartViewType,
    /// Maximum age of threads to include.
    pub max_age: Option<Duration>,
    /// Minimum confidence threshold.
    pub confidence_threshold: f32,
    /// Whether to include archived threads.
    pub include_archived: bool,
}

impl Default for ClassificationCriteria {
    fn default() -> Self {
        Self {
            view_type: SmartViewType::NeedsReply,
            max_age: None,
            confidence_threshold: 0.7,
            include_archived: false,
        }
    }
}

impl ClassificationCriteria {
    /// Creates criteria for needs reply view.
    pub fn needs_reply() -> Self {
        Self {
            view_type: SmartViewType::NeedsReply,
            max_age: Some(Duration::days(30)),
            confidence_threshold: 0.6,
            include_archived: false,
        }
    }

    /// Creates criteria for waiting for view.
    pub fn waiting_for() -> Self {
        Self {
            view_type: SmartViewType::WaitingFor,
            max_age: Some(Duration::days(14)),
            confidence_threshold: 0.7,
            include_archived: false,
        }
    }

    /// Creates criteria for newsletters view.
    pub fn newsletters() -> Self {
        Self {
            view_type: SmartViewType::Newsletters,
            max_age: Some(Duration::days(7)),
            confidence_threshold: 0.8,
            include_archived: true,
        }
    }
}

/// Service for managing smart view classifications.
pub struct SmartViewService<S: SmartViewStorage> {
    storage: S,
    account_id: AccountId,
    vip_contacts: Vec<String>,
    newsletter_patterns: Vec<String>,
}

impl<S: SmartViewStorage> SmartViewService<S> {
    /// Creates a new smart view service.
    pub fn new(storage: S, account_id: AccountId) -> Self {
        Self {
            storage,
            account_id,
            vip_contacts: Vec::new(),
            newsletter_patterns: vec![
                "noreply@".to_string(),
                "newsletter@".to_string(),
                "updates@".to_string(),
                "notifications@".to_string(),
                "digest@".to_string(),
                "weekly@".to_string(),
                "daily@".to_string(),
            ],
        }
    }

    /// Loads VIP contacts from storage.
    pub async fn load_vip_contacts(&mut self) -> SmartViewResult<()> {
        self.vip_contacts = self.storage.get_vip_contacts(&self.account_id).await?;
        Ok(())
    }

    /// Adds a VIP contact.
    pub async fn add_vip(&mut self, email: &str) -> SmartViewResult<()> {
        let email_lower = email.to_lowercase();
        if !self.vip_contacts.contains(&email_lower) {
            self.storage
                .add_vip_contact(&self.account_id, &email_lower)
                .await?;
            self.vip_contacts.push(email_lower);
        }
        Ok(())
    }

    /// Removes a VIP contact.
    pub async fn remove_vip(&mut self, email: &str) -> SmartViewResult<()> {
        let email_lower = email.to_lowercase();
        self.storage
            .remove_vip_contact(&self.account_id, &email_lower)
            .await?;
        self.vip_contacts.retain(|e| e != &email_lower);
        Ok(())
    }

    /// Checks if an email is a VIP contact.
    pub fn is_vip(&self, email: &str) -> bool {
        self.vip_contacts.contains(&email.to_lowercase())
    }

    /// Classifies a thread using rule-based heuristics.
    pub fn classify_heuristic(&self, input: &ClassificationInput) -> Vec<Classification> {
        let mut classifications = Vec::new();

        // Check VIP
        if self.is_vip(&input.sender_email) {
            classifications.push(Classification::ai_classified(
                input.thread_id.clone(),
                SmartViewType::Vip,
                1.0,
                "Sender is marked as VIP",
            ));
        }

        // Check Needs Reply
        if !input.user_was_last_sender && !input.is_read {
            let confidence = if input.message_count > 1 { 0.8 } else { 0.6 };
            classifications.push(Classification::ai_classified(
                input.thread_id.clone(),
                SmartViewType::NeedsReply,
                confidence,
                "Thread has unread messages from others",
            ));
        }

        // Check Waiting For
        if input.user_was_last_sender {
            let age = Utc::now() - input.last_message_date;
            if age > Duration::hours(24) && age < Duration::days(14) {
                classifications.push(Classification::ai_classified(
                    input.thread_id.clone(),
                    SmartViewType::WaitingFor,
                    0.7,
                    "User was last to reply, awaiting response",
                ));
            }
        }

        // Check Newsletter
        let email_lower = input.sender_email.to_lowercase();
        let is_newsletter = self
            .newsletter_patterns
            .iter()
            .any(|p| email_lower.contains(p));

        if is_newsletter {
            classifications.push(Classification::ai_classified(
                input.thread_id.clone(),
                SmartViewType::Newsletters,
                0.85,
                "Sender matches newsletter pattern",
            ));
        }

        // Check Attachments
        if input.has_attachments {
            classifications.push(Classification::ai_classified(
                input.thread_id.clone(),
                SmartViewType::Attachments,
                1.0,
                "Thread contains attachments",
            ));
        }

        classifications
    }

    /// Gets threads for a smart view.
    pub async fn get_threads(
        &self,
        view_type: SmartViewType,
    ) -> SmartViewResult<Vec<Classification>> {
        self.storage
            .get_threads_for_view(&self.account_id, view_type)
            .await
    }

    /// Gets counts for all smart views.
    pub async fn get_counts(&self) -> SmartViewResult<HashMap<SmartViewType, u32>> {
        let mut counts = HashMap::new();

        for view_type in SmartViewType::all() {
            let threads = self.get_threads(*view_type).await?;
            counts.insert(*view_type, threads.len() as u32);
        }

        Ok(counts)
    }

    /// Saves classifications for a thread.
    pub async fn save_classifications(
        &self,
        classifications: &[Classification],
    ) -> SmartViewResult<()> {
        for classification in classifications {
            self.storage.save_classification(classification).await?;
        }
        Ok(())
    }

    /// Manually assigns a thread to a view.
    pub async fn assign_manual(
        &self,
        thread_id: ThreadId,
        view_type: SmartViewType,
        reason: &str,
    ) -> SmartViewResult<()> {
        let classification = Classification::manual(thread_id, view_type, reason);
        self.storage.save_classification(&classification).await
    }

    /// Removes a thread from all smart views.
    pub async fn remove_thread(&self, thread_id: &ThreadId) -> SmartViewResult<()> {
        self.storage.remove_classifications(thread_id).await
    }

    /// Classifies and saves a thread.
    pub async fn classify_and_save(
        &self,
        input: &ClassificationInput,
    ) -> SmartViewResult<Vec<Classification>> {
        let classifications = self.classify_heuristic(input);
        self.save_classifications(&classifications).await?;
        Ok(classifications)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockStorage {
        classifications: Mutex<Vec<Classification>>,
        vip_contacts: Mutex<Vec<String>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                classifications: Mutex::new(Vec::new()),
                vip_contacts: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl SmartViewStorage for MockStorage {
        async fn get_classifications(
            &self,
            thread_id: &ThreadId,
        ) -> SmartViewResult<Vec<Classification>> {
            Ok(self
                .classifications
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.thread_id == *thread_id)
                .cloned()
                .collect())
        }

        async fn get_threads_for_view(
            &self,
            _account_id: &AccountId,
            view_type: SmartViewType,
        ) -> SmartViewResult<Vec<Classification>> {
            Ok(self
                .classifications
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.view_type == view_type)
                .cloned()
                .collect())
        }

        async fn save_classification(
            &self,
            classification: &Classification,
        ) -> SmartViewResult<()> {
            let mut classifications = self.classifications.lock().unwrap();
            classifications.retain(|c| {
                c.thread_id != classification.thread_id || c.view_type != classification.view_type
            });
            classifications.push(classification.clone());
            Ok(())
        }

        async fn remove_classifications(&self, thread_id: &ThreadId) -> SmartViewResult<()> {
            self.classifications
                .lock()
                .unwrap()
                .retain(|c| c.thread_id != *thread_id);
            Ok(())
        }

        async fn get_vip_contacts(&self, _account_id: &AccountId) -> SmartViewResult<Vec<String>> {
            Ok(self.vip_contacts.lock().unwrap().clone())
        }

        async fn add_vip_contact(
            &self,
            _account_id: &AccountId,
            email: &str,
        ) -> SmartViewResult<()> {
            self.vip_contacts.lock().unwrap().push(email.to_string());
            Ok(())
        }

        async fn remove_vip_contact(
            &self,
            _account_id: &AccountId,
            email: &str,
        ) -> SmartViewResult<()> {
            self.vip_contacts.lock().unwrap().retain(|e| e != email);
            Ok(())
        }
    }

    #[tokio::test]
    async fn classify_needs_reply() {
        let storage = MockStorage::new();
        let service = SmartViewService::new(storage, AccountId::from("test"));

        let input = ClassificationInput {
            thread_id: ThreadId::from("thread-1".to_string()),
            subject: Some("Question".to_string()),
            snippet: "Can you help?".to_string(),
            sender_email: "sender@example.com".to_string(),
            sender_name: None,
            user_was_last_sender: false,
            last_message_date: Utc::now(),
            is_read: false,
            has_attachments: false,
            message_count: 2,
            labels: vec![],
        };

        let classifications = service.classify_heuristic(&input);
        assert!(classifications
            .iter()
            .any(|c| c.view_type == SmartViewType::NeedsReply));
    }

    #[tokio::test]
    async fn classify_waiting_for() {
        let storage = MockStorage::new();
        let service = SmartViewService::new(storage, AccountId::from("test"));

        let input = ClassificationInput {
            thread_id: ThreadId::from("thread-1".to_string()),
            subject: Some("Sent question".to_string()),
            snippet: "I sent this".to_string(),
            sender_email: "me@example.com".to_string(),
            sender_name: None,
            user_was_last_sender: true,
            last_message_date: Utc::now() - Duration::days(2),
            is_read: true,
            has_attachments: false,
            message_count: 1,
            labels: vec![],
        };

        let classifications = service.classify_heuristic(&input);
        assert!(classifications
            .iter()
            .any(|c| c.view_type == SmartViewType::WaitingFor));
    }

    #[tokio::test]
    async fn classify_newsletter() {
        let storage = MockStorage::new();
        let service = SmartViewService::new(storage, AccountId::from("test"));

        let input = ClassificationInput {
            thread_id: ThreadId::from("thread-1".to_string()),
            subject: Some("Weekly digest".to_string()),
            snippet: "Your weekly update".to_string(),
            sender_email: "newsletter@example.com".to_string(),
            sender_name: None,
            user_was_last_sender: false,
            last_message_date: Utc::now(),
            is_read: true,
            has_attachments: false,
            message_count: 1,
            labels: vec![],
        };

        let classifications = service.classify_heuristic(&input);
        assert!(classifications
            .iter()
            .any(|c| c.view_type == SmartViewType::Newsletters));
    }

    #[tokio::test]
    async fn vip_contacts() {
        let storage = MockStorage::new();
        let mut service = SmartViewService::new(storage, AccountId::from("test"));

        service.add_vip("boss@company.com").await.unwrap();
        assert!(service.is_vip("boss@company.com"));
        assert!(service.is_vip("BOSS@COMPANY.COM")); // Case insensitive

        let input = ClassificationInput {
            thread_id: ThreadId::from("thread-1".to_string()),
            subject: Some("Important".to_string()),
            snippet: "From the boss".to_string(),
            sender_email: "boss@company.com".to_string(),
            sender_name: Some("Boss".to_string()),
            user_was_last_sender: false,
            last_message_date: Utc::now(),
            is_read: true,
            has_attachments: false,
            message_count: 1,
            labels: vec![],
        };

        let classifications = service.classify_heuristic(&input);
        assert!(classifications
            .iter()
            .any(|c| c.view_type == SmartViewType::Vip));
    }
}
