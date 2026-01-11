//! Sync service for email synchronization.
//!
//! The [`SyncService`] manages synchronization between remote email providers
//! and local storage, including background sync and offline queue processing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::domain::{AccountId, Email, EmailId};

/// Change from a remote email provider.
#[derive(Debug, Clone)]
pub enum Change {
    /// A new email was received.
    NewEmail(Box<Email>),
    /// An existing email was updated.
    Updated(EmailId, EmailUpdates),
    /// An email was deleted.
    Deleted(EmailId),
}

/// Updates to an email's metadata.
#[derive(Debug, Clone, Default)]
pub struct EmailUpdates {
    /// New read status.
    pub is_read: Option<bool>,
    /// New starred status.
    pub is_starred: Option<bool>,
    /// Labels to add.
    pub add_labels: Vec<String>,
    /// Labels to remove.
    pub remove_labels: Vec<String>,
}

/// A pending local change to sync to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    /// Unique identifier for this change.
    pub id: String,
    /// Account this change belongs to.
    pub account_id: AccountId,
    /// Type of change.
    pub change_type: PendingChangeType,
    /// When this change was created.
    pub created_at: DateTime<Utc>,
}

/// Type of pending change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingChangeType {
    /// Archive threads.
    Archive { thread_ids: Vec<String> },
    /// Trash threads.
    Trash { thread_ids: Vec<String> },
    /// Star/unstar a thread.
    Star { thread_id: String, starred: bool },
    /// Mark thread as read/unread.
    MarkRead { thread_id: String, read: bool },
    /// Apply a label.
    ApplyLabel { thread_id: String, label: String },
    /// Remove a label.
    RemoveLabel { thread_id: String, label: String },
    /// Send an email.
    SendEmail { draft_id: String },
}

/// Sync state for an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Last successful sync time.
    pub last_sync: Option<DateTime<Utc>>,
    /// Gmail-specific: last history ID.
    pub last_history_id: Option<String>,
    /// IMAP-specific: last UIDVALIDITY.
    pub last_uid_validity: Option<u32>,
    /// IMAP-specific: last UID.
    pub last_uid: Option<u32>,
}

impl SyncState {
    /// Creates a sync state for the current time.
    pub fn now() -> Self {
        Self {
            last_sync: Some(Utc::now()),
            last_history_id: None,
            last_uid_validity: None,
            last_uid: None,
        }
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of new emails received.
    pub emails_received: usize,
    /// Number of emails synced to server.
    pub emails_sent: usize,
    /// Number of changes applied locally.
    pub changes_applied: usize,
    /// Number of pending changes synced.
    pub pending_synced: usize,
    /// Errors encountered (non-fatal).
    pub errors: Vec<String>,
    /// Duration of the sync operation.
    pub duration_ms: u64,
}

impl SyncResult {
    /// Returns true if the sync completed without errors.
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Status of a sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Sync has never run.
    Never,
    /// Sync is currently in progress.
    InProgress,
    /// Last sync completed successfully.
    Success,
    /// Last sync failed.
    Failed,
    /// Offline, sync paused.
    Offline,
}

/// Settings for sync behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    /// Enable background sync.
    pub background_sync_enabled: bool,
    /// Interval between background syncs.
    pub sync_interval: Duration,
    /// Maximum retries on failure.
    pub max_retries: u32,
    /// Retry delay.
    pub retry_delay: Duration,
    /// Sync on app launch.
    pub sync_on_launch: bool,
    /// Maximum emails to fetch per sync.
    pub max_emails_per_sync: usize,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            background_sync_enabled: true,
            sync_interval: Duration::from_secs(300), // 5 minutes
            max_retries: 3,
            retry_delay: Duration::from_secs(30),
            sync_on_launch: true,
            max_emails_per_sync: 500,
        }
    }
}

/// Email provider trait for sync operations.
#[async_trait::async_trait]
pub trait SyncProvider: Send + Sync {
    /// Fetches changes since the last sync.
    async fn fetch_changes_since(&self, state: &SyncState) -> Result<Vec<Change>>;

    /// Pushes a local change to the server.
    async fn push_change(&self, change: &PendingChange) -> Result<()>;

    /// Gets the current sync state from the server.
    async fn get_current_state(&self) -> Result<SyncState>;
}

/// Storage trait for sync persistence.
#[async_trait::async_trait]
pub trait SyncStorage: Send + Sync {
    /// Gets the sync state for an account.
    async fn get_sync_state(&self, account_id: &AccountId) -> Result<SyncState>;

    /// Updates the sync state for an account.
    async fn update_sync_state(&self, account_id: &AccountId, state: SyncState) -> Result<()>;

    /// Gets pending changes for an account.
    async fn get_pending_changes(&self, account_id: &AccountId) -> Result<Vec<PendingChange>>;

    /// Marks a change as synced.
    async fn mark_change_synced(&self, change_id: &str) -> Result<()>;

    /// Inserts an email.
    async fn insert_email(&self, email: &Email) -> Result<()>;

    /// Updates an email.
    async fn update_email(&self, email_id: &EmailId, updates: &EmailUpdates) -> Result<()>;

    /// Deletes an email.
    async fn delete_email(&self, email_id: &EmailId) -> Result<()>;
}

/// Event emitted by the sync service.
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// Sync started for an account.
    Started(AccountId),
    /// Progress update.
    Progress {
        account_id: AccountId,
        processed: usize,
        total: usize,
    },
    /// Sync completed.
    Completed(AccountId, SyncResult),
    /// Sync failed.
    Failed(AccountId, String),
}

/// Sync service for managing email synchronization.
///
/// Handles both manual and background synchronization between remote
/// email providers and local storage.
///
/// # Thread Safety
///
/// SyncService uses atomic operations and locks for thread-safe access.
/// Background sync runs in a separate task and can be stopped at any time.
///
/// # Example
///
/// ```ignore
/// let service = SyncService::new(storage, settings);
/// service.register_provider(account_id, provider).await;
///
/// // Manual sync
/// let result = service.sync_account(&account_id).await?;
///
/// // Background sync
/// service.start_background_sync().await;
/// ```
pub struct SyncService<S: SyncStorage> {
    /// Registered sync providers by account ID.
    providers: RwLock<HashMap<AccountId, Arc<dyn SyncProvider>>>,
    /// Storage layer.
    storage: Arc<S>,
    /// Sync settings.
    settings: RwLock<SyncSettings>,
    /// Current sync status by account.
    status: RwLock<HashMap<AccountId, SyncStatus>>,
    /// Flag to stop background sync.
    stop_flag: AtomicBool,
    /// Event sender for sync events.
    event_sender: broadcast::Sender<SyncEvent>,
}

impl<S: SyncStorage + 'static> SyncService<S> {
    /// Creates a new SyncService.
    pub fn new(storage: Arc<S>, settings: SyncSettings) -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            providers: RwLock::new(HashMap::new()),
            storage,
            settings: RwLock::new(settings),
            status: RwLock::new(HashMap::new()),
            stop_flag: AtomicBool::new(false),
            event_sender,
        }
    }

    /// Registers a sync provider for an account.
    pub async fn register_provider(&self, account_id: AccountId, provider: Arc<dyn SyncProvider>) {
        let mut providers = self.providers.write().await;
        providers.insert(account_id.clone(), provider);

        let mut status = self.status.write().await;
        status.insert(account_id, SyncStatus::Never);
    }

    /// Unregisters a sync provider.
    pub async fn unregister_provider(&self, account_id: &AccountId) {
        let mut providers = self.providers.write().await;
        providers.remove(account_id);

        let mut status = self.status.write().await;
        status.remove(account_id);
    }

    /// Updates sync settings.
    pub async fn update_settings(&self, settings: SyncSettings) {
        let mut current = self.settings.write().await;
        *current = settings;
    }

    /// Subscribes to sync events.
    pub fn subscribe(&self) -> broadcast::Receiver<SyncEvent> {
        self.event_sender.subscribe()
    }

    /// Synchronizes a single account.
    ///
    /// Fetches changes from the remote provider, applies them locally,
    /// and pushes any pending local changes to the server.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account to sync
    ///
    /// # Returns
    ///
    /// A sync result with statistics.
    pub async fn sync_account(&self, account_id: &AccountId) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        // Update status
        {
            let mut status = self.status.write().await;
            status.insert(account_id.clone(), SyncStatus::InProgress);
        }

        let _ = self
            .event_sender
            .send(SyncEvent::Started(account_id.clone()));

        let result = self.do_sync(account_id).await;

        // Update status based on result
        {
            let mut status = self.status.write().await;
            match &result {
                Ok(_) => {
                    status.insert(account_id.clone(), SyncStatus::Success);
                }
                Err(_) => {
                    status.insert(account_id.clone(), SyncStatus::Failed);
                }
            }
        }

        match result {
            Ok(mut sync_result) => {
                sync_result.duration_ms = start.elapsed().as_millis() as u64;
                let _ = self.event_sender.send(SyncEvent::Completed(
                    account_id.clone(),
                    sync_result.clone(),
                ));
                Ok(sync_result)
            }
            Err(e) => {
                let _ = self
                    .event_sender
                    .send(SyncEvent::Failed(account_id.clone(), e.to_string()));
                Err(e)
            }
        }
    }

    /// Internal sync implementation.
    async fn do_sync(&self, account_id: &AccountId) -> Result<SyncResult> {
        let providers = self.providers.read().await;
        let provider = providers
            .get(account_id)
            .ok_or_else(|| anyhow::anyhow!("No provider for account: {}", account_id))?;

        // Get local state
        let local_state = self.storage.get_sync_state(account_id).await?;

        // Fetch changes from server
        let changes = provider.fetch_changes_since(&local_state).await?;
        let changes_count = changes.len();

        // Apply changes locally
        let mut errors = Vec::new();
        for change in changes {
            if let Err(e) = self.apply_change(&change).await {
                errors.push(format!("Failed to apply change: {}", e));
            }
        }

        // Push local changes
        let pending = self.storage.get_pending_changes(account_id).await?;
        let _pending_count = pending.len();
        let mut synced_count = 0;

        for change in pending {
            match provider.push_change(&change).await {
                Ok(()) => {
                    self.storage.mark_change_synced(&change.id).await?;
                    synced_count += 1;
                }
                Err(e) => {
                    errors.push(format!("Failed to push change: {}", e));
                }
            }
        }

        // Update sync state
        let new_state = provider
            .get_current_state()
            .await
            .unwrap_or_else(|_| SyncState::now());
        self.storage
            .update_sync_state(account_id, new_state)
            .await?;

        Ok(SyncResult {
            emails_received: changes_count,
            emails_sent: synced_count,
            changes_applied: changes_count,
            pending_synced: synced_count,
            errors,
            duration_ms: 0, // Filled in by caller
        })
    }

    /// Applies a change to local storage.
    async fn apply_change(&self, change: &Change) -> Result<()> {
        match change {
            Change::NewEmail(email) => {
                self.storage.insert_email(email).await?;
            }
            Change::Updated(email_id, updates) => {
                self.storage.update_email(email_id, updates).await?;
            }
            Change::Deleted(email_id) => {
                self.storage.delete_email(email_id).await?;
            }
        }
        Ok(())
    }

    /// Starts background synchronization.
    ///
    /// Spawns a task that periodically syncs all accounts.
    /// Call [`stop_background_sync`](Self::stop_background_sync) to stop.
    ///
    /// Note: This method requires `self` to be wrapped in `Arc` for the spawned task.
    /// In practice, the caller should use `Arc<SyncService>` and call this appropriately.
    pub fn start_background_sync(self: Arc<Self>) {
        self.stop_flag.store(false, Ordering::SeqCst);

        let service = Arc::clone(&self);

        tokio::spawn(async move {
            let settings = service.settings.read().await;
            if !settings.background_sync_enabled {
                return;
            }

            let interval = settings.sync_interval;
            drop(settings);

            loop {
                if service.stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                // Get current account IDs
                let providers = service.providers.read().await;
                let account_ids: Vec<AccountId> = providers.keys().cloned().collect();
                drop(providers);

                // Sync each account
                for account_id in &account_ids {
                    if service.stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    let _ = service.sync_account(account_id).await;
                }

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Stops background synchronization.
    pub fn stop_background_sync(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    /// Returns whether background sync is running.
    pub fn is_background_sync_running(&self) -> bool {
        !self.stop_flag.load(Ordering::SeqCst)
    }

    /// Gets the sync status for an account.
    pub async fn get_sync_status(&self, account_id: &AccountId) -> SyncStatus {
        let status = self.status.read().await;
        status.get(account_id).copied().unwrap_or(SyncStatus::Never)
    }

    /// Gets the sync status for all accounts.
    pub async fn get_all_sync_status(&self) -> HashMap<AccountId, SyncStatus> {
        self.status.read().await.clone()
    }

    /// Syncs all registered accounts.
    pub async fn sync_all(&self) -> Vec<(AccountId, Result<SyncResult>)> {
        let providers = self.providers.read().await;
        let account_ids: Vec<AccountId> = providers.keys().cloned().collect();
        drop(providers);

        let mut results = Vec::new();
        for account_id in account_ids {
            let result = self.sync_account(&account_id).await;
            results.push((account_id, result));
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_state_now() {
        let state = SyncState::now();
        assert!(state.last_sync.is_some());
        assert!(state.last_history_id.is_none());
    }

    #[test]
    fn sync_result_is_success() {
        let success = SyncResult {
            emails_received: 10,
            emails_sent: 2,
            changes_applied: 10,
            pending_synced: 2,
            errors: vec![],
            duration_ms: 1500,
        };
        assert!(success.is_success());

        let failure = SyncResult {
            emails_received: 5,
            emails_sent: 0,
            changes_applied: 5,
            pending_synced: 0,
            errors: vec!["Connection failed".to_string()],
            duration_ms: 500,
        };
        assert!(!failure.is_success());
    }

    #[test]
    fn sync_settings_default() {
        let settings = SyncSettings::default();
        assert!(settings.background_sync_enabled);
        assert_eq!(settings.sync_interval, Duration::from_secs(300));
        assert_eq!(settings.max_retries, 3);
        assert!(settings.sync_on_launch);
    }

    #[test]
    fn sync_status_serialization() {
        let status = SyncStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"in_progress\"");

        let deserialized: SyncStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SyncStatus::InProgress);
    }

    #[test]
    fn pending_change_serialization() {
        let change = PendingChange {
            id: "change-1".to_string(),
            account_id: AccountId::from("account-1"),
            change_type: PendingChangeType::Star {
                thread_id: "thread-1".to_string(),
                starred: true,
            },
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: PendingChange = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "change-1");
        match deserialized.change_type {
            PendingChangeType::Star { starred, .. } => assert!(starred),
            _ => panic!("Wrong change type"),
        }
    }

    #[test]
    fn email_updates_default() {
        let updates = EmailUpdates::default();
        assert!(updates.is_read.is_none());
        assert!(updates.is_starred.is_none());
        assert!(updates.add_labels.is_empty());
        assert!(updates.remove_labels.is_empty());
    }
}
