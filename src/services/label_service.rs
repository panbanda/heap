//! Label service for managing email labels and folders.
//!
//! Provides label management including:
//! - CRUD for user-created labels
//! - System label initialization
//! - Label-thread associations
//! - Color management

use thiserror::Error;

use crate::domain::{system_labels, AccountId, Label, LabelId, ThreadId};

/// Errors that can occur during label operations.
#[derive(Debug, Error)]
pub enum LabelError {
    #[error("Label not found: {0}")]
    NotFound(String),

    #[error("Label already exists: {0}")]
    AlreadyExists(String),

    #[error("Cannot modify system label: {0}")]
    SystemLabel(String),

    #[error("Invalid label name: {0}")]
    InvalidName(String),

    #[error("Invalid color format: {0}")]
    InvalidColor(String),

    #[error("Storage error: {0}")]
    Storage(String),
}

/// Result type for label operations.
pub type Result<T> = std::result::Result<T, LabelError>;

/// Sorting options for label queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelSort {
    /// Sort by name alphabetically.
    #[default]
    Name,
    /// Sort by creation order.
    CreatedOrder,
    /// Sort with system labels first.
    SystemFirst,
}

/// Storage trait for label persistence.
pub trait LabelStorage: Send + Sync {
    /// Gets a label by ID.
    fn get_by_id(&self, id: &LabelId) -> Result<Option<Label>>;

    /// Gets a label by name for an account.
    fn get_by_name(&self, account_id: &AccountId, name: &str) -> Result<Option<Label>>;

    /// Stores or updates a label.
    fn save(&self, label: &Label) -> Result<()>;

    /// Deletes a label.
    fn delete(&self, id: &LabelId) -> Result<()>;

    /// Gets all labels for an account.
    fn get_for_account(&self, account_id: &AccountId) -> Result<Vec<Label>>;

    /// Gets labels applied to a thread.
    fn get_for_thread(&self, thread_id: &ThreadId) -> Result<Vec<LabelId>>;

    /// Applies a label to a thread.
    fn apply_to_thread(&self, label_id: &LabelId, thread_id: &ThreadId) -> Result<()>;

    /// Removes a label from a thread.
    fn remove_from_thread(&self, label_id: &LabelId, thread_id: &ThreadId) -> Result<()>;

    /// Gets threads with a specific label.
    fn get_threads_with_label(&self, label_id: &LabelId) -> Result<Vec<ThreadId>>;

    /// Counts threads with a specific label.
    fn count_threads_with_label(&self, label_id: &LabelId) -> Result<usize>;
}

/// Service for managing email labels.
pub struct LabelService<S: LabelStorage> {
    storage: S,
}

impl<S: LabelStorage> LabelService<S> {
    /// Creates a new label service.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Initializes system labels for an account.
    pub fn init_system_labels(&self, account_id: &AccountId) -> Result<Vec<Label>> {
        let system_defs = [
            (system_labels::inbox(), "Inbox"),
            (system_labels::sent(), "Sent"),
            (system_labels::drafts(), "Drafts"),
            (system_labels::trash(), "Trash"),
            (system_labels::spam(), "Spam"),
            (system_labels::starred(), "Starred"),
            (system_labels::archive(), "Archive"),
        ];

        let mut labels = Vec::new();
        for (id, name) in system_defs {
            if self.storage.get_by_id(&id)?.is_none() {
                let label = Label {
                    id,
                    account_id: account_id.clone(),
                    name: name.to_string(),
                    color: None,
                    is_system: true,
                    provider_id: None,
                };
                self.storage.save(&label)?;
                labels.push(label);
            }
        }

        Ok(labels)
    }

    /// Gets a label by ID.
    pub fn get(&self, id: &LabelId) -> Result<Option<Label>> {
        self.storage.get_by_id(id)
    }

    /// Gets a label by name for an account.
    pub fn get_by_name(&self, account_id: &AccountId, name: &str) -> Result<Option<Label>> {
        self.storage.get_by_name(account_id, name)
    }

    /// Creates a new user label.
    pub fn create(&self, account_id: &AccountId, name: &str, color: Option<&str>) -> Result<Label> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LabelError::InvalidName("Name cannot be empty".to_string()));
        }

        if let Some(c) = color {
            validate_color(c)?;
        }

        if self.storage.get_by_name(account_id, name)?.is_some() {
            return Err(LabelError::AlreadyExists(name.to_string()));
        }

        let label = Label {
            id: LabelId::from(format!("label-{}", uuid::Uuid::new_v4())),
            account_id: account_id.clone(),
            name: name.to_string(),
            color: color.map(String::from),
            is_system: false,
            provider_id: None,
        };

        self.storage.save(&label)?;
        Ok(label)
    }

    /// Renames a label.
    pub fn rename(&self, id: &LabelId, new_name: &str) -> Result<Label> {
        let mut label = self
            .storage
            .get_by_id(id)?
            .ok_or_else(|| LabelError::NotFound(id.to_string()))?;

        if label.is_system {
            return Err(LabelError::SystemLabel(id.to_string()));
        }

        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(LabelError::InvalidName("Name cannot be empty".to_string()));
        }

        if let Some(existing) = self.storage.get_by_name(&label.account_id, new_name)? {
            if existing.id != *id {
                return Err(LabelError::AlreadyExists(new_name.to_string()));
            }
        }

        label.name = new_name.to_string();
        self.storage.save(&label)?;
        Ok(label)
    }

    /// Updates a label's color.
    pub fn set_color(&self, id: &LabelId, color: Option<&str>) -> Result<Label> {
        let mut label = self
            .storage
            .get_by_id(id)?
            .ok_or_else(|| LabelError::NotFound(id.to_string()))?;

        if let Some(c) = color {
            validate_color(c)?;
        }

        label.color = color.map(String::from);
        self.storage.save(&label)?;
        Ok(label)
    }

    /// Deletes a user label.
    pub fn delete(&self, id: &LabelId) -> Result<()> {
        let label = self
            .storage
            .get_by_id(id)?
            .ok_or_else(|| LabelError::NotFound(id.to_string()))?;

        if label.is_system {
            return Err(LabelError::SystemLabel(id.to_string()));
        }

        self.storage.delete(id)
    }

    /// Gets all labels for an account.
    pub fn list(&self, account_id: &AccountId, sort: LabelSort) -> Result<Vec<Label>> {
        let mut labels = self.storage.get_for_account(account_id)?;

        match sort {
            LabelSort::Name => {
                labels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            LabelSort::CreatedOrder => {
                // Keep natural order from storage
            }
            LabelSort::SystemFirst => {
                labels.sort_by(|a, b| match (a.is_system, b.is_system) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });
            }
        }

        Ok(labels)
    }

    /// Gets only user-created labels.
    pub fn list_user_labels(&self, account_id: &AccountId) -> Result<Vec<Label>> {
        let labels = self.storage.get_for_account(account_id)?;
        Ok(labels.into_iter().filter(|l| !l.is_system).collect())
    }

    /// Gets only system labels.
    pub fn list_system_labels(&self, account_id: &AccountId) -> Result<Vec<Label>> {
        let labels = self.storage.get_for_account(account_id)?;
        Ok(labels.into_iter().filter(|l| l.is_system).collect())
    }

    /// Applies a label to a thread.
    pub fn apply_to_thread(&self, label_id: &LabelId, thread_id: &ThreadId) -> Result<()> {
        if self.storage.get_by_id(label_id)?.is_none() {
            return Err(LabelError::NotFound(label_id.to_string()));
        }
        self.storage.apply_to_thread(label_id, thread_id)
    }

    /// Removes a label from a thread.
    pub fn remove_from_thread(&self, label_id: &LabelId, thread_id: &ThreadId) -> Result<()> {
        self.storage.remove_from_thread(label_id, thread_id)
    }

    /// Gets all labels for a thread.
    pub fn get_thread_labels(&self, thread_id: &ThreadId) -> Result<Vec<Label>> {
        let label_ids = self.storage.get_for_thread(thread_id)?;
        let mut labels = Vec::new();
        for id in label_ids {
            if let Some(label) = self.storage.get_by_id(&id)? {
                labels.push(label);
            }
        }
        Ok(labels)
    }

    /// Sets labels for a thread (replaces existing).
    pub fn set_thread_labels(&self, thread_id: &ThreadId, label_ids: &[LabelId]) -> Result<()> {
        let current = self.storage.get_for_thread(thread_id)?;

        // Remove labels not in new set
        for id in &current {
            if !label_ids.contains(id) {
                self.storage.remove_from_thread(id, thread_id)?;
            }
        }

        // Add new labels
        for id in label_ids {
            if !current.contains(id) {
                self.storage.apply_to_thread(id, thread_id)?;
            }
        }

        Ok(())
    }

    /// Gets threads with a specific label.
    pub fn get_threads(&self, label_id: &LabelId) -> Result<Vec<ThreadId>> {
        self.storage.get_threads_with_label(label_id)
    }

    /// Counts threads with a label.
    pub fn thread_count(&self, label_id: &LabelId) -> Result<usize> {
        self.storage.count_threads_with_label(label_id)
    }

    /// Gets label counts for an account (unread counts could be added).
    pub fn get_counts(&self, account_id: &AccountId) -> Result<Vec<(LabelId, usize)>> {
        let labels = self.storage.get_for_account(account_id)?;
        let mut counts = Vec::new();
        for label in labels {
            let count = self.storage.count_threads_with_label(&label.id)?;
            counts.push((label.id, count));
        }
        Ok(counts)
    }
}

/// Validates a hex color string.
fn validate_color(color: &str) -> Result<()> {
    let color = color.trim();
    if color.is_empty() {
        return Ok(()); // Empty is ok, treated as None
    }

    if !color.starts_with('#') {
        return Err(LabelError::InvalidColor(
            "Color must start with #".to_string(),
        ));
    }

    let hex = &color[1..];
    if hex.len() != 3 && hex.len() != 6 {
        return Err(LabelError::InvalidColor(
            "Color must be 3 or 6 hex digits".to_string(),
        ));
    }

    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(LabelError::InvalidColor(
            "Color must contain only hex digits".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::RwLock;

    struct MockStorage {
        labels: RwLock<HashMap<String, Label>>,
        thread_labels: RwLock<HashMap<String, HashSet<String>>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                labels: RwLock::new(HashMap::new()),
                thread_labels: RwLock::new(HashMap::new()),
            }
        }
    }

    impl LabelStorage for MockStorage {
        fn get_by_id(&self, id: &LabelId) -> Result<Option<Label>> {
            Ok(self.labels.read().unwrap().get(&id.0).cloned())
        }

        fn get_by_name(&self, account_id: &AccountId, name: &str) -> Result<Option<Label>> {
            Ok(self
                .labels
                .read()
                .unwrap()
                .values()
                .find(|l| l.account_id == *account_id && l.name == name)
                .cloned())
        }

        fn save(&self, label: &Label) -> Result<()> {
            self.labels
                .write()
                .unwrap()
                .insert(label.id.0.clone(), label.clone());
            Ok(())
        }

        fn delete(&self, id: &LabelId) -> Result<()> {
            self.labels.write().unwrap().remove(&id.0);
            Ok(())
        }

        fn get_for_account(&self, account_id: &AccountId) -> Result<Vec<Label>> {
            Ok(self
                .labels
                .read()
                .unwrap()
                .values()
                .filter(|l| l.account_id == *account_id)
                .cloned()
                .collect())
        }

        fn get_for_thread(&self, thread_id: &ThreadId) -> Result<Vec<LabelId>> {
            Ok(self
                .thread_labels
                .read()
                .unwrap()
                .get(&thread_id.0)
                .map(|ids| ids.iter().map(|s| LabelId::from(s.clone())).collect())
                .unwrap_or_default())
        }

        fn apply_to_thread(&self, label_id: &LabelId, thread_id: &ThreadId) -> Result<()> {
            self.thread_labels
                .write()
                .unwrap()
                .entry(thread_id.0.clone())
                .or_default()
                .insert(label_id.0.clone());
            Ok(())
        }

        fn remove_from_thread(&self, label_id: &LabelId, thread_id: &ThreadId) -> Result<()> {
            if let Some(ids) = self.thread_labels.write().unwrap().get_mut(&thread_id.0) {
                ids.remove(&label_id.0);
            }
            Ok(())
        }

        fn get_threads_with_label(&self, label_id: &LabelId) -> Result<Vec<ThreadId>> {
            Ok(self
                .thread_labels
                .read()
                .unwrap()
                .iter()
                .filter(|(_, labels)| labels.contains(&label_id.0))
                .map(|(tid, _)| ThreadId::from(tid.clone()))
                .collect())
        }

        fn count_threads_with_label(&self, label_id: &LabelId) -> Result<usize> {
            Ok(self
                .thread_labels
                .read()
                .unwrap()
                .values()
                .filter(|labels| labels.contains(&label_id.0))
                .count())
        }
    }

    fn make_account_id(s: &str) -> AccountId {
        AccountId::from(s.to_string())
    }

    fn make_thread_id(s: &str) -> ThreadId {
        ThreadId::from(s.to_string())
    }

    #[test]
    fn validate_color_works() {
        assert!(validate_color("#fff").is_ok());
        assert!(validate_color("#ffffff").is_ok());
        assert!(validate_color("#ABC123").is_ok());
        assert!(validate_color("").is_ok());
        assert!(validate_color("fff").is_err());
        assert!(validate_color("#ff").is_err());
        assert!(validate_color("#fffffff").is_err());
        assert!(validate_color("#gggggg").is_err());
    }

    #[test]
    fn init_system_labels() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let labels = service.init_system_labels(&account).unwrap();
        assert_eq!(labels.len(), 7);
        assert!(labels.iter().all(|l| l.is_system));

        // Second init should not create duplicates
        let labels2 = service.init_system_labels(&account).unwrap();
        assert_eq!(labels2.len(), 0);
    }

    #[test]
    fn create_user_label() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let label = service.create(&account, "Work", Some("#0066cc")).unwrap();
        assert_eq!(label.name, "Work");
        assert_eq!(label.color, Some("#0066cc".to_string()));
        assert!(!label.is_system);
    }

    #[test]
    fn create_duplicate_fails() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        service.create(&account, "Work", None).unwrap();
        let result = service.create(&account, "Work", None);
        assert!(matches!(result, Err(LabelError::AlreadyExists(_))));
    }

    #[test]
    fn create_invalid_name_fails() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let result = service.create(&account, "", None);
        assert!(matches!(result, Err(LabelError::InvalidName(_))));

        let result = service.create(&account, "   ", None);
        assert!(matches!(result, Err(LabelError::InvalidName(_))));
    }

    #[test]
    fn create_invalid_color_fails() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let result = service.create(&account, "Work", Some("invalid"));
        assert!(matches!(result, Err(LabelError::InvalidColor(_))));
    }

    #[test]
    fn rename_label() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let label = service.create(&account, "Work", None).unwrap();
        let renamed = service.rename(&label.id, "Business").unwrap();
        assert_eq!(renamed.name, "Business");
    }

    #[test]
    fn rename_system_label_fails() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        service.init_system_labels(&account).unwrap();
        let result = service.rename(&system_labels::inbox(), "My Inbox");
        assert!(matches!(result, Err(LabelError::SystemLabel(_))));
    }

    #[test]
    fn set_color() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let label = service.create(&account, "Work", None).unwrap();
        assert!(label.color.is_none());

        let updated = service.set_color(&label.id, Some("#ff0000")).unwrap();
        assert_eq!(updated.color, Some("#ff0000".to_string()));

        let cleared = service.set_color(&label.id, None).unwrap();
        assert!(cleared.color.is_none());
    }

    #[test]
    fn delete_user_label() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let label = service.create(&account, "Work", None).unwrap();
        service.delete(&label.id).unwrap();

        assert!(service.get(&label.id).unwrap().is_none());
    }

    #[test]
    fn delete_system_label_fails() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        service.init_system_labels(&account).unwrap();
        let result = service.delete(&system_labels::inbox());
        assert!(matches!(result, Err(LabelError::SystemLabel(_))));
    }

    #[test]
    fn list_labels_sorted() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        service.init_system_labels(&account).unwrap();
        service.create(&account, "Zebra", None).unwrap();
        service.create(&account, "Alpha", None).unwrap();

        let by_name = service.list(&account, LabelSort::Name).unwrap();
        assert_eq!(by_name[0].name, "Alpha");

        let system_first = service.list(&account, LabelSort::SystemFirst).unwrap();
        assert!(system_first[0].is_system);
    }

    #[test]
    fn apply_and_remove_labels() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");
        let thread = make_thread_id("thread-1");

        let label = service.create(&account, "Work", None).unwrap();

        service.apply_to_thread(&label.id, &thread).unwrap();
        let labels = service.get_thread_labels(&thread).unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "Work");

        service.remove_from_thread(&label.id, &thread).unwrap();
        let labels = service.get_thread_labels(&thread).unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn set_thread_labels() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");
        let thread = make_thread_id("thread-1");

        let work = service.create(&account, "Work", None).unwrap();
        let personal = service.create(&account, "Personal", None).unwrap();
        let urgent = service.create(&account, "Urgent", None).unwrap();

        // Set initial labels
        service
            .set_thread_labels(&thread, &[work.id.clone(), personal.id.clone()])
            .unwrap();

        let labels = service.get_thread_labels(&thread).unwrap();
        assert_eq!(labels.len(), 2);

        // Replace with different labels
        service
            .set_thread_labels(&thread, &[urgent.id.clone()])
            .unwrap();

        let labels = service.get_thread_labels(&thread).unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "Urgent");
    }

    #[test]
    fn thread_count() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let label = service.create(&account, "Work", None).unwrap();

        assert_eq!(service.thread_count(&label.id).unwrap(), 0);

        service
            .apply_to_thread(&label.id, &make_thread_id("t1"))
            .unwrap();
        service
            .apply_to_thread(&label.id, &make_thread_id("t2"))
            .unwrap();

        assert_eq!(service.thread_count(&label.id).unwrap(), 2);
    }

    #[test]
    fn get_threads_with_label() {
        let storage = MockStorage::new();
        let service = LabelService::new(storage);
        let account = make_account_id("account-1");

        let label = service.create(&account, "Work", None).unwrap();

        service
            .apply_to_thread(&label.id, &make_thread_id("t1"))
            .unwrap();
        service
            .apply_to_thread(&label.id, &make_thread_id("t2"))
            .unwrap();

        let threads = service.get_threads(&label.id).unwrap();
        assert_eq!(threads.len(), 2);
    }
}
