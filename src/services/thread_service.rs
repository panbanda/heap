//! Thread service for managing email threads.
//!
//! Provides a service layer for thread operations including:
//! - Retrieving threads with various filters
//! - Updating thread metadata (starred, read status)
//! - Thread archiving and deletion
//! - Thread statistics

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{AccountId, LabelId, Thread, ThreadId, ThreadSummary};

/// Errors that can occur during thread operations.
#[derive(Debug, Error)]
pub enum ThreadError {
    /// Thread not found.
    #[error("thread not found: {0}")]
    NotFound(String),

    /// Invalid thread operation.
    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),
}

/// Result type for thread operations.
pub type ThreadResult<T> = Result<T, ThreadError>;

/// Filter options for listing threads.
#[derive(Debug, Clone, Default)]
pub struct ThreadFilter {
    /// Filter by account.
    pub account_id: Option<AccountId>,
    /// Filter by label.
    pub label_id: Option<LabelId>,
    /// Only unread threads.
    pub unread_only: bool,
    /// Only starred threads.
    pub starred_only: bool,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
}

impl ThreadFilter {
    /// Creates a new filter for an account.
    pub fn for_account(account_id: AccountId) -> Self {
        Self {
            account_id: Some(account_id),
            ..Default::default()
        }
    }

    /// Filters to only unread threads.
    pub fn unread(mut self) -> Self {
        self.unread_only = true;
        self
    }

    /// Filters to only starred threads.
    pub fn starred(mut self) -> Self {
        self.starred_only = true;
        self
    }

    /// Filters by label.
    pub fn with_label(mut self, label_id: LabelId) -> Self {
        self.label_id = Some(label_id);
        self
    }

    /// Sets the result limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the offset for pagination.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Sorting options for thread lists.
#[derive(Debug, Clone, Copy, Default)]
pub enum ThreadSort {
    /// Sort by date, newest first.
    #[default]
    DateDesc,
    /// Sort by date, oldest first.
    DateAsc,
    /// Unread threads first, then by date.
    UnreadFirst,
}

/// Storage abstraction for thread operations.
#[async_trait]
pub trait ThreadStorage: Send + Sync {
    /// Gets a thread by ID with all messages.
    async fn get_thread(&self, id: &ThreadId) -> ThreadResult<Option<Thread>>;

    /// Gets a thread summary by ID.
    async fn get_thread_summary(&self, id: &ThreadId) -> ThreadResult<Option<ThreadSummary>>;

    /// Lists thread summaries matching the filter.
    async fn list_threads(
        &self,
        filter: &ThreadFilter,
        sort: ThreadSort,
    ) -> ThreadResult<Vec<ThreadSummary>>;

    /// Updates the starred status of a thread.
    async fn set_starred(&self, id: &ThreadId, starred: bool) -> ThreadResult<()>;

    /// Updates the unread count of a thread.
    async fn set_unread_count(&self, id: &ThreadId, count: u32) -> ThreadResult<()>;

    /// Marks all messages in a thread as read.
    async fn mark_read(&self, id: &ThreadId) -> ThreadResult<()>;

    /// Marks all messages in a thread as unread.
    async fn mark_unread(&self, id: &ThreadId) -> ThreadResult<()>;

    /// Adds a label to a thread.
    async fn add_label(&self, id: &ThreadId, label_id: &LabelId) -> ThreadResult<()>;

    /// Removes a label from a thread.
    async fn remove_label(&self, id: &ThreadId, label_id: &LabelId) -> ThreadResult<()>;

    /// Archives a thread (removes from inbox).
    async fn archive(&self, id: &ThreadId) -> ThreadResult<()>;

    /// Moves a thread to trash.
    async fn trash(&self, id: &ThreadId) -> ThreadResult<()>;

    /// Permanently deletes a thread.
    async fn delete(&self, id: &ThreadId) -> ThreadResult<()>;

    /// Counts threads matching the filter.
    async fn count_threads(&self, filter: &ThreadFilter) -> ThreadResult<u32>;
}

/// Statistics about threads for an account.
#[derive(Debug, Clone, Default)]
pub struct ThreadStats {
    /// Total number of threads.
    pub total_threads: u32,
    /// Number of unread threads.
    pub unread_threads: u32,
    /// Number of starred threads.
    pub starred_threads: u32,
}

/// Service for managing email threads.
pub struct ThreadService<S: ThreadStorage> {
    storage: S,
}

impl<S: ThreadStorage> ThreadService<S> {
    /// Creates a new thread service.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Gets a thread by ID with all messages.
    pub async fn get_thread(&self, id: &ThreadId) -> ThreadResult<Thread> {
        self.storage
            .get_thread(id)
            .await?
            .ok_or_else(|| ThreadError::NotFound(id.to_string()))
    }

    /// Gets a thread summary by ID.
    pub async fn get_thread_summary(&self, id: &ThreadId) -> ThreadResult<ThreadSummary> {
        self.storage
            .get_thread_summary(id)
            .await?
            .ok_or_else(|| ThreadError::NotFound(id.to_string()))
    }

    /// Lists threads for an account.
    pub async fn list_threads(
        &self,
        account_id: AccountId,
        sort: ThreadSort,
    ) -> ThreadResult<Vec<ThreadSummary>> {
        let filter = ThreadFilter::for_account(account_id);
        self.storage.list_threads(&filter, sort).await
    }

    /// Lists threads with a custom filter.
    pub async fn list_threads_filtered(
        &self,
        filter: &ThreadFilter,
        sort: ThreadSort,
    ) -> ThreadResult<Vec<ThreadSummary>> {
        self.storage.list_threads(filter, sort).await
    }

    /// Lists unread threads for an account.
    pub async fn list_unread(&self, account_id: AccountId) -> ThreadResult<Vec<ThreadSummary>> {
        let filter = ThreadFilter::for_account(account_id).unread();
        self.storage
            .list_threads(&filter, ThreadSort::DateDesc)
            .await
    }

    /// Lists starred threads for an account.
    pub async fn list_starred(&self, account_id: AccountId) -> ThreadResult<Vec<ThreadSummary>> {
        let filter = ThreadFilter::for_account(account_id).starred();
        self.storage
            .list_threads(&filter, ThreadSort::DateDesc)
            .await
    }

    /// Lists threads with a specific label.
    pub async fn list_by_label(
        &self,
        account_id: AccountId,
        label_id: LabelId,
    ) -> ThreadResult<Vec<ThreadSummary>> {
        let filter = ThreadFilter::for_account(account_id).with_label(label_id);
        self.storage
            .list_threads(&filter, ThreadSort::DateDesc)
            .await
    }

    /// Stars a thread.
    pub async fn star(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.set_starred(id, true).await
    }

    /// Unstars a thread.
    pub async fn unstar(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.set_starred(id, false).await
    }

    /// Toggles the starred status of a thread.
    pub async fn toggle_star(&self, id: &ThreadId) -> ThreadResult<bool> {
        let summary = self.get_thread_summary(id).await?;
        let new_starred = !summary.is_starred;
        self.storage.set_starred(id, new_starred).await?;
        Ok(new_starred)
    }

    /// Marks a thread as read.
    pub async fn mark_read(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.mark_read(id).await
    }

    /// Marks a thread as unread.
    pub async fn mark_unread(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.mark_unread(id).await
    }

    /// Toggles the read status of a thread.
    pub async fn toggle_read(&self, id: &ThreadId) -> ThreadResult<bool> {
        let summary = self.get_thread_summary(id).await?;
        let is_read = summary.unread_count == 0;
        if is_read {
            self.storage.mark_unread(id).await?;
        } else {
            self.storage.mark_read(id).await?;
        }
        Ok(!is_read)
    }

    /// Adds a label to a thread.
    pub async fn add_label(&self, id: &ThreadId, label_id: &LabelId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.add_label(id, label_id).await
    }

    /// Removes a label from a thread.
    pub async fn remove_label(&self, id: &ThreadId, label_id: &LabelId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.remove_label(id, label_id).await
    }

    /// Archives a thread (removes from inbox, keeps in All Mail).
    pub async fn archive(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.archive(id).await
    }

    /// Moves a thread to trash.
    pub async fn trash(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.trash(id).await
    }

    /// Permanently deletes a thread.
    pub async fn delete(&self, id: &ThreadId) -> ThreadResult<()> {
        // Verify thread exists
        self.get_thread_summary(id).await?;
        self.storage.delete(id).await
    }

    /// Gets thread statistics for an account.
    pub async fn get_stats(&self, account_id: AccountId) -> ThreadResult<ThreadStats> {
        let total_filter = ThreadFilter::for_account(account_id.clone());
        let unread_filter = ThreadFilter::for_account(account_id.clone()).unread();
        let starred_filter = ThreadFilter::for_account(account_id).starred();

        let total_threads = self.storage.count_threads(&total_filter).await?;
        let unread_threads = self.storage.count_threads(&unread_filter).await?;
        let starred_threads = self.storage.count_threads(&starred_filter).await?;

        Ok(ThreadStats {
            total_threads,
            unread_threads,
            starred_threads,
        })
    }

    /// Counts threads matching a filter.
    pub async fn count(&self, filter: &ThreadFilter) -> ThreadResult<u32> {
        self.storage.count_threads(filter).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Address;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockStorage {
        threads: Mutex<HashMap<ThreadId, ThreadSummary>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                threads: Mutex::new(HashMap::new()),
            }
        }

        fn with_thread(self, summary: ThreadSummary) -> Self {
            self.threads
                .lock()
                .unwrap()
                .insert(summary.id.clone(), summary);
            self
        }
    }

    fn make_summary(id: &str, account_id: &str) -> ThreadSummary {
        ThreadSummary {
            id: ThreadId::from(id),
            account_id: AccountId::from(account_id),
            subject: Some("Test Subject".to_string()),
            snippet: "Preview text...".to_string(),
            from: Address::new("sender@example.com"),
            last_message_date: Utc::now(),
            message_count: 1,
            unread_count: 1,
            is_starred: false,
            labels: vec![LabelId::from("INBOX")],
        }
    }

    #[async_trait]
    impl ThreadStorage for MockStorage {
        async fn get_thread(&self, id: &ThreadId) -> ThreadResult<Option<Thread>> {
            let threads = self.threads.lock().unwrap();
            Ok(threads.get(id).map(|s| Thread {
                id: s.id.clone(),
                account_id: s.account_id.clone(),
                subject: s.subject.clone(),
                snippet: s.snippet.clone(),
                participants: vec![s.from.clone()],
                messages: vec![],
                last_message_date: s.last_message_date,
                unread_count: s.unread_count,
                is_starred: s.is_starred,
                labels: s.labels.clone(),
            }))
        }

        async fn get_thread_summary(&self, id: &ThreadId) -> ThreadResult<Option<ThreadSummary>> {
            let threads = self.threads.lock().unwrap();
            Ok(threads.get(id).cloned())
        }

        async fn list_threads(
            &self,
            filter: &ThreadFilter,
            _sort: ThreadSort,
        ) -> ThreadResult<Vec<ThreadSummary>> {
            let threads = self.threads.lock().unwrap();
            let mut result: Vec<_> = threads
                .values()
                .filter(|t| {
                    if let Some(ref account_id) = filter.account_id {
                        if &t.account_id != account_id {
                            return false;
                        }
                    }
                    if filter.unread_only && t.unread_count == 0 {
                        return false;
                    }
                    if filter.starred_only && !t.is_starred {
                        return false;
                    }
                    if let Some(ref label_id) = filter.label_id {
                        if !t.labels.contains(label_id) {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect();

            // Apply pagination
            if let Some(offset) = filter.offset {
                result = result.into_iter().skip(offset as usize).collect();
            }
            if let Some(limit) = filter.limit {
                result.truncate(limit as usize);
            }

            Ok(result)
        }

        async fn set_starred(&self, id: &ThreadId, starred: bool) -> ThreadResult<()> {
            let mut threads = self.threads.lock().unwrap();
            if let Some(thread) = threads.get_mut(id) {
                thread.is_starred = starred;
                Ok(())
            } else {
                Err(ThreadError::NotFound(id.to_string()))
            }
        }

        async fn set_unread_count(&self, id: &ThreadId, count: u32) -> ThreadResult<()> {
            let mut threads = self.threads.lock().unwrap();
            if let Some(thread) = threads.get_mut(id) {
                thread.unread_count = count;
                Ok(())
            } else {
                Err(ThreadError::NotFound(id.to_string()))
            }
        }

        async fn mark_read(&self, id: &ThreadId) -> ThreadResult<()> {
            self.set_unread_count(id, 0).await
        }

        async fn mark_unread(&self, id: &ThreadId) -> ThreadResult<()> {
            self.set_unread_count(id, 1).await
        }

        async fn add_label(&self, id: &ThreadId, label_id: &LabelId) -> ThreadResult<()> {
            let mut threads = self.threads.lock().unwrap();
            if let Some(thread) = threads.get_mut(id) {
                if !thread.labels.contains(label_id) {
                    thread.labels.push(label_id.clone());
                }
                Ok(())
            } else {
                Err(ThreadError::NotFound(id.to_string()))
            }
        }

        async fn remove_label(&self, id: &ThreadId, label_id: &LabelId) -> ThreadResult<()> {
            let mut threads = self.threads.lock().unwrap();
            if let Some(thread) = threads.get_mut(id) {
                thread.labels.retain(|l| l != label_id);
                Ok(())
            } else {
                Err(ThreadError::NotFound(id.to_string()))
            }
        }

        async fn archive(&self, id: &ThreadId) -> ThreadResult<()> {
            self.remove_label(id, &LabelId::from("INBOX")).await
        }

        async fn trash(&self, id: &ThreadId) -> ThreadResult<()> {
            self.add_label(id, &LabelId::from("TRASH")).await
        }

        async fn delete(&self, id: &ThreadId) -> ThreadResult<()> {
            let mut threads = self.threads.lock().unwrap();
            if threads.remove(id).is_some() {
                Ok(())
            } else {
                Err(ThreadError::NotFound(id.to_string()))
            }
        }

        async fn count_threads(&self, filter: &ThreadFilter) -> ThreadResult<u32> {
            let threads = self.list_threads(filter, ThreadSort::DateDesc).await?;
            Ok(threads.len() as u32)
        }
    }

    #[tokio::test]
    async fn get_thread_not_found() {
        let storage = MockStorage::new();
        let service = ThreadService::new(storage);

        let result = service.get_thread(&ThreadId::from("nonexistent")).await;
        assert!(matches!(result, Err(ThreadError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_thread_success() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let thread = service
            .get_thread(&ThreadId::from("thread-1"))
            .await
            .unwrap();
        assert_eq!(thread.id, ThreadId::from("thread-1"));
    }

    #[tokio::test]
    async fn list_threads_by_account() {
        let summary1 = make_summary("thread-1", "account-1");
        let summary2 = make_summary("thread-2", "account-1");
        let summary3 = make_summary("thread-3", "account-2");

        let storage = MockStorage::new()
            .with_thread(summary1)
            .with_thread(summary2)
            .with_thread(summary3);
        let service = ThreadService::new(storage);

        let threads = service
            .list_threads(AccountId::from("account-1"), ThreadSort::DateDesc)
            .await
            .unwrap();
        assert_eq!(threads.len(), 2);
    }

    #[tokio::test]
    async fn list_unread_threads() {
        let mut summary1 = make_summary("thread-1", "account-1");
        summary1.unread_count = 0; // read
        let summary2 = make_summary("thread-2", "account-1"); // unread

        let storage = MockStorage::new()
            .with_thread(summary1)
            .with_thread(summary2);
        let service = ThreadService::new(storage);

        let threads = service
            .list_unread(AccountId::from("account-1"))
            .await
            .unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, ThreadId::from("thread-2"));
    }

    #[tokio::test]
    async fn list_starred_threads() {
        let summary1 = make_summary("thread-1", "account-1");
        let mut summary2 = make_summary("thread-2", "account-1");
        summary2.is_starred = true;

        let storage = MockStorage::new()
            .with_thread(summary1)
            .with_thread(summary2);
        let service = ThreadService::new(storage);

        let threads = service
            .list_starred(AccountId::from("account-1"))
            .await
            .unwrap();
        assert_eq!(threads.len(), 1);
        assert!(threads[0].is_starred);
    }

    #[tokio::test]
    async fn star_and_unstar() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        // Initially not starred
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(!thread.is_starred);

        // Star it
        service.star(&id).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(thread.is_starred);

        // Unstar it
        service.unstar(&id).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(!thread.is_starred);
    }

    #[tokio::test]
    async fn toggle_star() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        // Toggle on
        let is_starred = service.toggle_star(&id).await.unwrap();
        assert!(is_starred);

        // Toggle off
        let is_starred = service.toggle_star(&id).await.unwrap();
        assert!(!is_starred);
    }

    #[tokio::test]
    async fn mark_read_and_unread() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        // Initially unread
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(thread.has_unread());

        // Mark read
        service.mark_read(&id).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(!thread.has_unread());

        // Mark unread
        service.mark_unread(&id).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(thread.has_unread());
    }

    #[tokio::test]
    async fn toggle_read() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        // Initially unread, toggle to read
        let is_read = service.toggle_read(&id).await.unwrap();
        assert!(is_read);

        // Toggle back to unread
        let is_read = service.toggle_read(&id).await.unwrap();
        assert!(!is_read);
    }

    #[tokio::test]
    async fn add_and_remove_label() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");
        let label = LabelId::from("IMPORTANT");

        // Add label
        service.add_label(&id, &label).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(thread.labels.contains(&label));

        // Remove label
        service.remove_label(&id, &label).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(!thread.labels.contains(&label));
    }

    #[tokio::test]
    async fn archive_thread() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        // Initially in inbox
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(thread.labels.contains(&LabelId::from("INBOX")));

        // Archive
        service.archive(&id).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(!thread.labels.contains(&LabelId::from("INBOX")));
    }

    #[tokio::test]
    async fn trash_thread() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        // Trash
        service.trash(&id).await.unwrap();
        let thread = service.get_thread_summary(&id).await.unwrap();
        assert!(thread.labels.contains(&LabelId::from("TRASH")));
    }

    #[tokio::test]
    async fn delete_thread() {
        let summary = make_summary("thread-1", "account-1");
        let storage = MockStorage::new().with_thread(summary);
        let service = ThreadService::new(storage);

        let id = ThreadId::from("thread-1");

        service.delete(&id).await.unwrap();

        let result = service.get_thread(&id).await;
        assert!(matches!(result, Err(ThreadError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_thread_stats() {
        let summary1 = make_summary("thread-1", "account-1");
        let mut summary2 = make_summary("thread-2", "account-1");
        summary2.unread_count = 0;
        let mut summary3 = make_summary("thread-3", "account-1");
        summary3.is_starred = true;

        let storage = MockStorage::new()
            .with_thread(summary1)
            .with_thread(summary2)
            .with_thread(summary3);
        let service = ThreadService::new(storage);

        let stats = service
            .get_stats(AccountId::from("account-1"))
            .await
            .unwrap();
        assert_eq!(stats.total_threads, 3);
        assert_eq!(stats.unread_threads, 2); // thread-1 and thread-3
        assert_eq!(stats.starred_threads, 1);
    }

    #[tokio::test]
    async fn filter_by_label() {
        let summary1 = make_summary("thread-1", "account-1");
        let mut summary2 = make_summary("thread-2", "account-1");
        summary2.labels = vec![LabelId::from("IMPORTANT")];

        let storage = MockStorage::new()
            .with_thread(summary1)
            .with_thread(summary2);
        let service = ThreadService::new(storage);

        let threads = service
            .list_by_label(AccountId::from("account-1"), LabelId::from("INBOX"))
            .await
            .unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, ThreadId::from("thread-1"));
    }

    #[tokio::test]
    async fn thread_filter_builders() {
        let filter = ThreadFilter::for_account(AccountId::from("acc-1"))
            .unread()
            .starred()
            .with_label(LabelId::from("INBOX"))
            .limit(10)
            .offset(5);

        assert_eq!(filter.account_id, Some(AccountId::from("acc-1")));
        assert!(filter.unread_only);
        assert!(filter.starred_only);
        assert_eq!(filter.label_id, Some(LabelId::from("INBOX")));
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.offset, Some(5));
    }
}
