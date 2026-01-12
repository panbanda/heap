//! Contact service for managing email contacts.
//!
//! Provides contact management including:
//! - Auto-creation from email interactions
//! - VIP status management
//! - Frequency tracking
//! - Search and filtering

use thiserror::Error;

use crate::domain::{Address, Contact};

/// Errors that can occur during contact operations.
#[derive(Debug, Error)]
pub enum ContactError {
    #[error("Contact not found: {0}")]
    NotFound(String),

    #[error("Contact already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid email address: {0}")]
    InvalidEmail(String),

    #[error("Storage error: {0}")]
    Storage(String),
}

/// Result type for contact operations.
pub type Result<T> = std::result::Result<T, ContactError>;

/// Sorting options for contact queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactSort {
    /// Sort by name alphabetically.
    #[default]
    Name,
    /// Sort by email alphabetically.
    Email,
    /// Sort by frequency (most contacted first).
    Frequency,
    /// Sort by last contacted date (most recent first).
    LastContacted,
}

/// Filter options for contact queries.
#[derive(Debug, Clone, Default)]
pub struct ContactFilter {
    /// Only include VIP contacts.
    pub vip_only: bool,
    /// Search query for name or email.
    pub search: Option<String>,
    /// Minimum frequency threshold.
    pub min_frequency: Option<u32>,
    /// Maximum results to return.
    pub limit: Option<usize>,
}

impl ContactFilter {
    /// Creates an empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters to only VIP contacts.
    pub fn vip(mut self) -> Self {
        self.vip_only = true;
        self
    }

    /// Adds a search query.
    pub fn search(mut self, query: impl Into<String>) -> Self {
        self.search = Some(query.into());
        self
    }

    /// Sets minimum frequency.
    pub fn min_frequency(mut self, freq: u32) -> Self {
        self.min_frequency = Some(freq);
        self
    }

    /// Sets maximum results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Checks if a contact matches this filter.
    pub fn matches(&self, contact: &Contact) -> bool {
        if self.vip_only && !contact.is_vip {
            return false;
        }

        if let Some(min_freq) = self.min_frequency {
            if contact.frequency < min_freq {
                return false;
            }
        }

        if let Some(ref query) = self.search {
            let query_lower = query.to_lowercase();
            let name_match = contact
                .name
                .as_ref()
                .map(|n| n.to_lowercase().contains(&query_lower))
                .unwrap_or(false);
            let email_match = contact.email.to_lowercase().contains(&query_lower);
            if !name_match && !email_match {
                return false;
            }
        }

        true
    }
}

/// Contact statistics.
#[derive(Debug, Clone, Default)]
pub struct ContactStats {
    /// Total number of contacts.
    pub total: usize,
    /// Number of VIP contacts.
    pub vip_count: usize,
    /// Average contact frequency.
    pub avg_frequency: f64,
    /// Most contacted email.
    pub most_contacted: Option<String>,
}

/// Storage trait for contact persistence.
pub trait ContactStorage: Send + Sync {
    /// Gets a contact by ID.
    fn get_by_id(&self, id: &str) -> Result<Option<Contact>>;

    /// Gets a contact by email address.
    fn get_by_email(&self, email: &str) -> Result<Option<Contact>>;

    /// Stores or updates a contact.
    fn save(&self, contact: &Contact) -> Result<()>;

    /// Deletes a contact.
    fn delete(&self, id: &str) -> Result<()>;

    /// Gets all contacts.
    fn get_all(&self) -> Result<Vec<Contact>>;

    /// Gets contacts matching a filter.
    fn query(&self, filter: &ContactFilter, sort: ContactSort) -> Result<Vec<Contact>>;
}

/// Service for managing contacts.
pub struct ContactService<S: ContactStorage> {
    storage: S,
}

impl<S: ContactStorage> ContactService<S> {
    /// Creates a new contact service.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Gets a contact by ID.
    pub fn get(&self, id: &str) -> Result<Option<Contact>> {
        self.storage.get_by_id(id)
    }

    /// Gets a contact by email address.
    pub fn get_by_email(&self, email: &str) -> Result<Option<Contact>> {
        self.storage.get_by_email(&normalize_email(email))
    }

    /// Creates a new contact.
    pub fn create(&self, email: &str, name: Option<&str>) -> Result<Contact> {
        let normalized = normalize_email(email);
        if !is_valid_email(&normalized) {
            return Err(ContactError::InvalidEmail(email.to_string()));
        }

        if self.storage.get_by_email(&normalized)?.is_some() {
            return Err(ContactError::AlreadyExists(normalized));
        }

        let contact = match name {
            Some(n) => Contact::with_name(&normalized, n),
            None => Contact::new(&normalized),
        };

        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Gets or creates a contact from an email address.
    pub fn get_or_create(&self, address: &Address) -> Result<Contact> {
        let normalized = normalize_email(&address.email);
        if let Some(existing) = self.storage.get_by_email(&normalized)? {
            return Ok(existing);
        }

        let contact = match &address.name {
            Some(n) => Contact::with_name(&normalized, n),
            None => Contact::new(&normalized),
        };

        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Updates a contact's name.
    pub fn update_name(&self, id: &str, name: Option<&str>) -> Result<Contact> {
        let mut contact = self
            .storage
            .get_by_id(id)?
            .ok_or_else(|| ContactError::NotFound(id.to_string()))?;

        contact.name = name.map(String::from);
        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Updates a contact's notes.
    pub fn update_notes(&self, id: &str, notes: Option<&str>) -> Result<Contact> {
        let mut contact = self
            .storage
            .get_by_id(id)?
            .ok_or_else(|| ContactError::NotFound(id.to_string()))?;

        contact.notes = notes.map(String::from);
        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Sets VIP status for a contact.
    pub fn set_vip(&self, id: &str, is_vip: bool) -> Result<Contact> {
        let mut contact = self
            .storage
            .get_by_id(id)?
            .ok_or_else(|| ContactError::NotFound(id.to_string()))?;

        contact.is_vip = is_vip;
        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Records an interaction with a contact (increments frequency).
    pub fn record_interaction(&self, email: &str) -> Result<Contact> {
        let normalized = normalize_email(email);
        let mut contact = self
            .storage
            .get_by_email(&normalized)?
            .ok_or_else(|| ContactError::NotFound(normalized.clone()))?;

        contact.record_interaction();
        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Records an interaction, creating the contact if needed.
    pub fn record_interaction_or_create(&self, address: &Address) -> Result<Contact> {
        let normalized = normalize_email(&address.email);
        let mut contact = match self.storage.get_by_email(&normalized)? {
            Some(c) => c,
            None => {
                let c = match &address.name {
                    Some(n) => Contact::with_name(&normalized, n),
                    None => Contact::new(&normalized),
                };
                self.storage.save(&c)?;
                c
            }
        };

        contact.record_interaction();
        self.storage.save(&contact)?;
        Ok(contact)
    }

    /// Deletes a contact.
    pub fn delete(&self, id: &str) -> Result<()> {
        if self.storage.get_by_id(id)?.is_none() {
            return Err(ContactError::NotFound(id.to_string()));
        }
        self.storage.delete(id)
    }

    /// Queries contacts with filter and sort options.
    pub fn query(&self, filter: &ContactFilter, sort: ContactSort) -> Result<Vec<Contact>> {
        self.storage.query(filter, sort)
    }

    /// Gets all VIP contacts.
    pub fn get_vips(&self) -> Result<Vec<Contact>> {
        self.query(&ContactFilter::new().vip(), ContactSort::Name)
    }

    /// Gets most frequently contacted contacts.
    pub fn get_frequent(&self, limit: usize) -> Result<Vec<Contact>> {
        self.query(&ContactFilter::new().limit(limit), ContactSort::Frequency)
    }

    /// Gets recently contacted contacts.
    pub fn get_recent(&self, limit: usize) -> Result<Vec<Contact>> {
        self.query(
            &ContactFilter::new().limit(limit),
            ContactSort::LastContacted,
        )
    }

    /// Searches contacts by name or email.
    pub fn search(&self, query: &str) -> Result<Vec<Contact>> {
        self.query(&ContactFilter::new().search(query), ContactSort::Name)
    }

    /// Gets contact statistics.
    pub fn stats(&self) -> Result<ContactStats> {
        let all = self.storage.get_all()?;
        let total = all.len();
        let vip_count = all.iter().filter(|c| c.is_vip).count();

        let avg_frequency = if total > 0 {
            all.iter().map(|c| c.frequency as f64).sum::<f64>() / total as f64
        } else {
            0.0
        };

        let most_contacted = all
            .iter()
            .max_by_key(|c| c.frequency)
            .map(|c| c.email.clone());

        Ok(ContactStats {
            total,
            vip_count,
            avg_frequency,
            most_contacted,
        })
    }

    /// Gets count of all contacts.
    pub fn count(&self) -> Result<usize> {
        Ok(self.storage.get_all()?.len())
    }

    /// Gets count of VIP contacts.
    pub fn vip_count(&self) -> Result<usize> {
        Ok(self.storage.get_all()?.iter().filter(|c| c.is_vip).count())
    }
}

/// Normalizes an email address (lowercase, trim).
fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Basic email validation.
fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    struct MockStorage {
        contacts: RwLock<HashMap<String, Contact>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                contacts: RwLock::new(HashMap::new()),
            }
        }
    }

    impl ContactStorage for MockStorage {
        fn get_by_id(&self, id: &str) -> Result<Option<Contact>> {
            Ok(self.contacts.read().unwrap().get(id).cloned())
        }

        fn get_by_email(&self, email: &str) -> Result<Option<Contact>> {
            Ok(self
                .contacts
                .read()
                .unwrap()
                .values()
                .find(|c| c.email == email)
                .cloned())
        }

        fn save(&self, contact: &Contact) -> Result<()> {
            self.contacts
                .write()
                .unwrap()
                .insert(contact.id.clone(), contact.clone());
            Ok(())
        }

        fn delete(&self, id: &str) -> Result<()> {
            self.contacts.write().unwrap().remove(id);
            Ok(())
        }

        fn get_all(&self) -> Result<Vec<Contact>> {
            Ok(self.contacts.read().unwrap().values().cloned().collect())
        }

        fn query(&self, filter: &ContactFilter, sort: ContactSort) -> Result<Vec<Contact>> {
            let contacts = self.contacts.read().unwrap();
            let mut filtered: Vec<Contact> = contacts
                .values()
                .filter(|c| filter.matches(c))
                .cloned()
                .collect();

            match sort {
                ContactSort::Name => {
                    filtered.sort_by(|a, b| a.display_name().cmp(b.display_name()));
                }
                ContactSort::Email => {
                    filtered.sort_by(|a, b| a.email.cmp(&b.email));
                }
                ContactSort::Frequency => {
                    filtered.sort_by(|a, b| b.frequency.cmp(&a.frequency));
                }
                ContactSort::LastContacted => {
                    filtered.sort_by(|a, b| b.last_contacted.cmp(&a.last_contacted));
                }
            }

            if let Some(limit) = filter.limit {
                filtered.truncate(limit);
            }

            Ok(filtered)
        }
    }

    #[test]
    fn normalize_email_works() {
        assert_eq!(normalize_email("Test@Example.COM"), "test@example.com");
        assert_eq!(normalize_email("  foo@bar.com  "), "foo@bar.com");
    }

    #[test]
    fn is_valid_email_works() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("foo.bar@domain.co.uk"));
        assert!(!is_valid_email("notanemail"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("test@"));
        assert!(!is_valid_email("test@nodot"));
        assert!(!is_valid_email(""));
    }

    #[test]
    fn create_contact() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let contact = service
            .create("test@example.com", Some("Test User"))
            .unwrap();
        assert_eq!(contact.email, "test@example.com");
        assert_eq!(contact.name, Some("Test User".to_string()));
    }

    #[test]
    fn create_duplicate_fails() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        service.create("test@example.com", None).unwrap();
        let result = service.create("test@example.com", None);
        assert!(matches!(result, Err(ContactError::AlreadyExists(_))));
    }

    #[test]
    fn create_invalid_email_fails() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let result = service.create("notanemail", None);
        assert!(matches!(result, Err(ContactError::InvalidEmail(_))));
    }

    #[test]
    fn get_or_create() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let addr = Address::with_name("test@example.com", "Test User");

        let contact1 = service.get_or_create(&addr).unwrap();
        let contact2 = service.get_or_create(&addr).unwrap();

        assert_eq!(contact1.id, contact2.id);
        assert_eq!(service.count().unwrap(), 1);
    }

    #[test]
    fn set_vip() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let contact = service.create("test@example.com", None).unwrap();
        assert!(!contact.is_vip);

        let updated = service.set_vip(&contact.id, true).unwrap();
        assert!(updated.is_vip);

        let vips = service.get_vips().unwrap();
        assert_eq!(vips.len(), 1);
    }

    #[test]
    fn record_interaction() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let contact = service.create("test@example.com", None).unwrap();
        assert_eq!(contact.frequency, 1);

        let updated = service.record_interaction("test@example.com").unwrap();
        assert_eq!(updated.frequency, 2);
        assert!(updated.last_contacted.is_some());
    }

    #[test]
    fn record_interaction_or_create() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let addr = Address::with_name("new@example.com", "New Contact");
        let contact = service.record_interaction_or_create(&addr).unwrap();

        assert_eq!(contact.frequency, 2); // Initial 1 + interaction
        assert_eq!(contact.name, Some("New Contact".to_string()));
    }

    #[test]
    fn search_contacts() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        service
            .create("alice@example.com", Some("Alice Smith"))
            .unwrap();
        service
            .create("bob@example.com", Some("Bob Jones"))
            .unwrap();
        service
            .create("carol@domain.org", Some("Carol White"))
            .unwrap();

        let results = service.search("alice").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].email, "alice@example.com");

        let results = service.search("example.com").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn filter_by_frequency() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        service.create("low@example.com", None).unwrap();
        service.create("high@example.com", None).unwrap();

        // Increment c2's frequency
        service.record_interaction("high@example.com").unwrap();
        service.record_interaction("high@example.com").unwrap();

        let filter = ContactFilter::new().min_frequency(3);
        let results = service.query(&filter, ContactSort::Frequency).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].email, "high@example.com");
    }

    #[test]
    fn contact_stats() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        service.create("a@example.com", None).unwrap();
        let b = service.create("b@example.com", None).unwrap();
        service.create("c@example.com", None).unwrap();

        service.set_vip(&b.id, true).unwrap();
        service.record_interaction("b@example.com").unwrap();
        service.record_interaction("b@example.com").unwrap();

        let stats = service.stats().unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.vip_count, 1);
        assert_eq!(stats.most_contacted, Some("b@example.com".to_string()));
    }

    #[test]
    fn delete_contact() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let contact = service.create("test@example.com", None).unwrap();
        assert_eq!(service.count().unwrap(), 1);

        service.delete(&contact.id).unwrap();
        assert_eq!(service.count().unwrap(), 0);
    }

    #[test]
    fn delete_nonexistent_fails() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let result = service.delete("nonexistent");
        assert!(matches!(result, Err(ContactError::NotFound(_))));
    }

    #[test]
    fn update_name() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let contact = service.create("test@example.com", None).unwrap();
        assert!(contact.name.is_none());

        let updated = service.update_name(&contact.id, Some("New Name")).unwrap();
        assert_eq!(updated.name, Some("New Name".to_string()));

        let cleared = service.update_name(&contact.id, None).unwrap();
        assert!(cleared.name.is_none());
    }

    #[test]
    fn update_notes() {
        let storage = MockStorage::new();
        let service = ContactService::new(storage);

        let contact = service.create("test@example.com", None).unwrap();
        let updated = service
            .update_notes(&contact.id, Some("Important contact"))
            .unwrap();
        assert_eq!(updated.notes, Some("Important contact".to_string()));
    }
}
