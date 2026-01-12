//! Account service for managing email accounts.
//!
//! The [`AccountService`] handles account lifecycle operations including:
//! - Adding and removing accounts
//! - Updating account settings
//! - Managing authentication credentials
//! - Tracking active account state

use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{Account, AccountId, ProviderConfig, ProviderType};

/// Errors that can occur during account operations.
#[derive(Debug, Error)]
pub enum AccountError {
    /// Account not found.
    #[error("account not found: {0}")]
    NotFound(String),

    /// Account already exists.
    #[error("account already exists: {0}")]
    AlreadyExists(String),

    /// Invalid configuration.
    #[error("invalid account configuration: {0}")]
    InvalidConfig(String),

    /// Authentication failed.
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Credential storage error.
    #[error("credential storage error: {0}")]
    CredentialError(String),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),
}

/// Result type for account operations.
pub type AccountResult<T> = Result<T, AccountError>;

/// Storage trait for account persistence.
#[async_trait]
pub trait AccountStorage: Send + Sync {
    /// Inserts a new account.
    async fn insert(&self, account: &Account) -> AccountResult<()>;

    /// Retrieves an account by ID.
    async fn get_by_id(&self, account_id: &AccountId) -> AccountResult<Option<Account>>;

    /// Retrieves an account by email.
    async fn get_by_email(&self, email: &str) -> AccountResult<Option<Account>>;

    /// Retrieves all accounts.
    async fn get_all(&self) -> AccountResult<Vec<Account>>;

    /// Updates an account.
    async fn update(&self, account: &Account) -> AccountResult<()>;

    /// Deletes an account and all associated data.
    async fn delete(&self, account_id: &AccountId) -> AccountResult<()>;

    /// Checks if an account with the given email exists.
    async fn exists_by_email(&self, email: &str) -> AccountResult<bool>;

    /// Counts total accounts.
    async fn count(&self) -> AccountResult<u32>;
}

/// Trait for secure credential storage.
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// Stores a credential.
    async fn store(&self, key: &str, value: &str) -> AccountResult<()>;

    /// Retrieves a credential.
    async fn get(&self, key: &str) -> AccountResult<Option<String>>;

    /// Deletes a credential.
    async fn delete(&self, key: &str) -> AccountResult<()>;
}

/// Request to create a new account.
#[derive(Debug, Clone)]
pub struct CreateAccountRequest {
    /// Email address for the account.
    pub email: String,
    /// Display name for the account.
    pub display_name: Option<String>,
    /// Provider type.
    pub provider_type: ProviderType,
    /// Provider-specific configuration.
    pub provider_config: ProviderConfig,
    /// Whether sync is enabled.
    pub sync_enabled: bool,
    /// Sync interval.
    pub sync_interval: Duration,
    /// Optional signature.
    pub signature: Option<String>,
}

impl CreateAccountRequest {
    /// Creates a new Gmail account request.
    pub fn gmail(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            display_name: None,
            provider_type: ProviderType::Gmail,
            provider_config: ProviderConfig::Gmail {},
            sync_enabled: true,
            sync_interval: Duration::from_secs(300),
            signature: None,
        }
    }

    /// Creates a new IMAP account request.
    pub fn imap(
        email: impl Into<String>,
        imap_host: impl Into<String>,
        imap_port: u16,
        smtp_host: impl Into<String>,
        smtp_port: u16,
        use_tls: bool,
    ) -> Self {
        Self {
            email: email.into(),
            display_name: None,
            provider_type: ProviderType::Imap,
            provider_config: ProviderConfig::Imap {
                imap_host: imap_host.into(),
                imap_port,
                smtp_host: smtp_host.into(),
                smtp_port,
                use_tls,
            },
            sync_enabled: true,
            sync_interval: Duration::from_secs(300),
            signature: None,
        }
    }

    /// Sets the display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets the sync interval.
    pub fn sync_interval(mut self, interval: Duration) -> Self {
        self.sync_interval = interval;
        self
    }

    /// Disables sync.
    pub fn sync_disabled(mut self) -> Self {
        self.sync_enabled = false;
        self
    }

    /// Sets the signature.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }
}

/// Updates to an existing account.
#[derive(Debug, Clone, Default)]
pub struct AccountUpdate {
    /// New display name.
    pub display_name: Option<Option<String>>,
    /// New sync enabled state.
    pub sync_enabled: Option<bool>,
    /// New sync interval.
    pub sync_interval: Option<Duration>,
    /// New signature.
    pub signature: Option<Option<String>>,
}

impl AccountUpdate {
    /// Creates a new empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the display name.
    pub fn display_name(mut self, name: Option<impl Into<String>>) -> Self {
        self.display_name = Some(name.map(|n| n.into()));
        self
    }

    /// Sets sync enabled state.
    pub fn sync_enabled(mut self, enabled: bool) -> Self {
        self.sync_enabled = Some(enabled);
        self
    }

    /// Sets sync interval.
    pub fn sync_interval(mut self, interval: Duration) -> Self {
        self.sync_interval = Some(interval);
        self
    }

    /// Sets the signature.
    pub fn signature(mut self, signature: Option<impl Into<String>>) -> Self {
        self.signature = Some(signature.map(|s| s.into()));
        self
    }
}

/// Statistics about accounts.
#[derive(Debug, Clone, Default)]
pub struct AccountStats {
    /// Total number of accounts.
    pub total_accounts: u32,
    /// Number of Gmail accounts.
    pub gmail_accounts: u32,
    /// Number of IMAP accounts.
    pub imap_accounts: u32,
    /// Number of accounts with sync enabled.
    pub sync_enabled_count: u32,
}

/// Service for managing email accounts.
pub struct AccountService<S: AccountStorage, C: CredentialStore> {
    storage: S,
    credentials: C,
    active_account_id: Option<AccountId>,
}

impl<S: AccountStorage, C: CredentialStore> AccountService<S, C> {
    /// Creates a new account service.
    pub fn new(storage: S, credentials: C) -> Self {
        Self {
            storage,
            credentials,
            active_account_id: None,
        }
    }

    /// Creates a new account.
    pub async fn create_account(
        &mut self,
        request: CreateAccountRequest,
    ) -> AccountResult<Account> {
        // Validate email format
        if !is_valid_email(&request.email) {
            return Err(AccountError::InvalidConfig(format!(
                "invalid email address: {}",
                request.email
            )));
        }

        // Check for duplicate
        if self.storage.exists_by_email(&request.email).await? {
            return Err(AccountError::AlreadyExists(request.email));
        }

        // Validate provider config
        validate_provider_config(&request.provider_type, &request.provider_config)?;

        let account = Account {
            id: AccountId::from(format!("acct-{}", uuid::Uuid::new_v4())),
            email: request.email,
            display_name: request.display_name,
            provider_type: request.provider_type,
            provider_config: request.provider_config,
            sync_enabled: request.sync_enabled,
            sync_interval: request.sync_interval,
            signature: request.signature,
        };

        self.storage.insert(&account).await?;

        // Set as active if this is the first account
        if self.active_account_id.is_none() {
            self.active_account_id = Some(account.id.clone());
        }

        Ok(account)
    }

    /// Retrieves an account by ID.
    pub async fn get_account(&self, account_id: &AccountId) -> AccountResult<Option<Account>> {
        self.storage.get_by_id(account_id).await
    }

    /// Retrieves an account by email.
    pub async fn get_account_by_email(&self, email: &str) -> AccountResult<Option<Account>> {
        self.storage.get_by_email(email).await
    }

    /// Retrieves all accounts.
    pub async fn get_all_accounts(&self) -> AccountResult<Vec<Account>> {
        self.storage.get_all().await
    }

    /// Updates an existing account.
    pub async fn update_account(
        &self,
        account_id: &AccountId,
        update: AccountUpdate,
    ) -> AccountResult<Account> {
        let mut account = self
            .storage
            .get_by_id(account_id)
            .await?
            .ok_or_else(|| AccountError::NotFound(account_id.0.clone()))?;

        if let Some(display_name) = update.display_name {
            account.display_name = display_name;
        }
        if let Some(sync_enabled) = update.sync_enabled {
            account.sync_enabled = sync_enabled;
        }
        if let Some(sync_interval) = update.sync_interval {
            account.sync_interval = sync_interval;
        }
        if let Some(signature) = update.signature {
            account.signature = signature;
        }

        self.storage.update(&account).await?;

        Ok(account)
    }

    /// Deletes an account and all associated data.
    pub async fn delete_account(&mut self, account_id: &AccountId) -> AccountResult<()> {
        // Check if account exists
        if self.storage.get_by_id(account_id).await?.is_none() {
            return Err(AccountError::NotFound(account_id.0.clone()));
        }

        // Delete credentials
        self.credentials
            .delete(&format!("oauth_{}", account_id.0))
            .await
            .ok();
        self.credentials
            .delete(&format!("password_{}", account_id.0))
            .await
            .ok();

        // Delete account data
        self.storage.delete(account_id).await?;

        // Clear active account if it was deleted
        if self.active_account_id.as_ref() == Some(account_id) {
            // Try to set another account as active
            let accounts = self.storage.get_all().await?;
            self.active_account_id = accounts.first().map(|a| a.id.clone());
        }

        Ok(())
    }

    /// Returns the currently active account.
    pub fn active_account_id(&self) -> Option<&AccountId> {
        self.active_account_id.as_ref()
    }

    /// Sets the active account.
    pub async fn set_active_account(&mut self, account_id: &AccountId) -> AccountResult<()> {
        // Verify account exists
        if self.storage.get_by_id(account_id).await?.is_none() {
            return Err(AccountError::NotFound(account_id.0.clone()));
        }

        self.active_account_id = Some(account_id.clone());
        Ok(())
    }

    /// Returns the currently active account.
    pub async fn get_active_account(&self) -> AccountResult<Option<Account>> {
        match &self.active_account_id {
            Some(id) => self.storage.get_by_id(id).await,
            None => Ok(None),
        }
    }

    /// Stores OAuth tokens for an account.
    pub async fn store_oauth_tokens(
        &self,
        account_id: &AccountId,
        access_token: &str,
        refresh_token: Option<&str>,
    ) -> AccountResult<()> {
        let key = format!("oauth_{}", account_id.0);
        let value = serde_json::json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
        });

        self.credentials.store(&key, &value.to_string()).await
    }

    /// Retrieves OAuth tokens for an account.
    pub async fn get_oauth_tokens(
        &self,
        account_id: &AccountId,
    ) -> AccountResult<Option<(String, Option<String>)>> {
        let key = format!("oauth_{}", account_id.0);

        match self.credentials.get(&key).await? {
            Some(value) => {
                let parsed: serde_json::Value = serde_json::from_str(&value)
                    .map_err(|e| AccountError::CredentialError(e.to_string()))?;
                let access_token = parsed["access_token"]
                    .as_str()
                    .ok_or_else(|| AccountError::CredentialError("missing access_token".into()))?
                    .to_string();
                let refresh_token = parsed["refresh_token"].as_str().map(|s| s.to_string());
                Ok(Some((access_token, refresh_token)))
            }
            None => Ok(None),
        }
    }

    /// Stores password for an IMAP account.
    pub async fn store_password(
        &self,
        account_id: &AccountId,
        password: &str,
    ) -> AccountResult<()> {
        let key = format!("password_{}", account_id.0);
        self.credentials.store(&key, password).await
    }

    /// Retrieves password for an IMAP account.
    pub async fn get_password(&self, account_id: &AccountId) -> AccountResult<Option<String>> {
        let key = format!("password_{}", account_id.0);
        self.credentials.get(&key).await
    }

    /// Returns account statistics.
    pub async fn get_stats(&self) -> AccountResult<AccountStats> {
        let accounts = self.storage.get_all().await?;

        let mut stats = AccountStats {
            total_accounts: accounts.len() as u32,
            ..Default::default()
        };

        for account in accounts {
            match account.provider_type {
                ProviderType::Gmail => stats.gmail_accounts += 1,
                ProviderType::Imap => stats.imap_accounts += 1,
            }
            if account.sync_enabled {
                stats.sync_enabled_count += 1;
            }
        }

        Ok(stats)
    }

    /// Returns the total number of accounts.
    pub async fn count(&self) -> AccountResult<u32> {
        self.storage.count().await
    }
}

/// Validates an email address format.
fn is_valid_email(email: &str) -> bool {
    // Basic validation: contains @ with text on both sides
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    !parts[0].is_empty() && !parts[1].is_empty() && parts[1].contains('.')
}

/// Validates provider configuration matches provider type.
fn validate_provider_config(
    provider_type: &ProviderType,
    config: &ProviderConfig,
) -> AccountResult<()> {
    match (provider_type, config) {
        (ProviderType::Gmail, ProviderConfig::Gmail { .. }) => Ok(()),
        (
            ProviderType::Imap,
            ProviderConfig::Imap {
                imap_host,
                smtp_host,
                ..
            },
        ) => {
            if imap_host.is_empty() || smtp_host.is_empty() {
                return Err(AccountError::InvalidConfig(
                    "IMAP and SMTP hosts are required".into(),
                ));
            }
            Ok(())
        }
        _ => Err(AccountError::InvalidConfig(
            "provider type does not match configuration".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockStorage {
        accounts: Mutex<HashMap<String, Account>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                accounts: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl AccountStorage for MockStorage {
        async fn insert(&self, account: &Account) -> AccountResult<()> {
            self.accounts
                .lock()
                .unwrap()
                .insert(account.id.0.clone(), account.clone());
            Ok(())
        }

        async fn get_by_id(&self, account_id: &AccountId) -> AccountResult<Option<Account>> {
            Ok(self.accounts.lock().unwrap().get(&account_id.0).cloned())
        }

        async fn get_by_email(&self, email: &str) -> AccountResult<Option<Account>> {
            Ok(self
                .accounts
                .lock()
                .unwrap()
                .values()
                .find(|a| a.email == email)
                .cloned())
        }

        async fn get_all(&self) -> AccountResult<Vec<Account>> {
            Ok(self.accounts.lock().unwrap().values().cloned().collect())
        }

        async fn update(&self, account: &Account) -> AccountResult<()> {
            self.accounts
                .lock()
                .unwrap()
                .insert(account.id.0.clone(), account.clone());
            Ok(())
        }

        async fn delete(&self, account_id: &AccountId) -> AccountResult<()> {
            self.accounts.lock().unwrap().remove(&account_id.0);
            Ok(())
        }

        async fn exists_by_email(&self, email: &str) -> AccountResult<bool> {
            Ok(self
                .accounts
                .lock()
                .unwrap()
                .values()
                .any(|a| a.email == email))
        }

        async fn count(&self) -> AccountResult<u32> {
            Ok(self.accounts.lock().unwrap().len() as u32)
        }
    }

    struct MockCredentials {
        store: Mutex<HashMap<String, String>>,
    }

    impl MockCredentials {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl CredentialStore for MockCredentials {
        async fn store(&self, key: &str, value: &str) -> AccountResult<()> {
            self.store
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn get(&self, key: &str) -> AccountResult<Option<String>> {
            Ok(self.store.lock().unwrap().get(key).cloned())
        }

        async fn delete(&self, key: &str) -> AccountResult<()> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    fn create_service() -> AccountService<MockStorage, MockCredentials> {
        AccountService::new(MockStorage::new(), MockCredentials::new())
    }

    #[tokio::test]
    async fn create_gmail_account() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com").display_name("Test User");

        let account = service.create_account(request).await.unwrap();

        assert_eq!(account.email, "test@gmail.com");
        assert_eq!(account.display_name, Some("Test User".to_string()));
        assert_eq!(account.provider_type, ProviderType::Gmail);
        assert!(account.sync_enabled);
    }

    #[tokio::test]
    async fn create_imap_account() {
        let mut service = create_service();

        let request = CreateAccountRequest::imap(
            "test@example.com",
            "imap.example.com",
            993,
            "smtp.example.com",
            587,
            true,
        );

        let account = service.create_account(request).await.unwrap();

        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.provider_type, ProviderType::Imap);
    }

    #[tokio::test]
    async fn reject_duplicate_email() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        service.create_account(request.clone()).await.unwrap();

        let result = service.create_account(request).await;
        assert!(matches!(result, Err(AccountError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn reject_invalid_email() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("invalid-email");
        let result = service.create_account(request).await;

        assert!(matches!(result, Err(AccountError::InvalidConfig(_))));
    }

    #[tokio::test]
    async fn get_account_by_id() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let created = service.create_account(request).await.unwrap();

        let retrieved = service.get_account(&created.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, "test@gmail.com");
    }

    #[tokio::test]
    async fn update_account_settings() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let created = service.create_account(request).await.unwrap();

        let update = AccountUpdate::new()
            .display_name(Some("New Name"))
            .sync_enabled(false)
            .signature(Some("-- Sent from margin"));

        let updated = service.update_account(&created.id, update).await.unwrap();

        assert_eq!(updated.display_name, Some("New Name".to_string()));
        assert!(!updated.sync_enabled);
        assert_eq!(updated.signature, Some("-- Sent from margin".to_string()));
    }

    #[tokio::test]
    async fn delete_account() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let created = service.create_account(request).await.unwrap();

        service.delete_account(&created.id).await.unwrap();

        let retrieved = service.get_account(&created.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn active_account_management() {
        let mut service = create_service();

        // First account becomes active automatically
        let first = service
            .create_account(CreateAccountRequest::gmail("first@gmail.com"))
            .await
            .unwrap();

        assert_eq!(service.active_account_id(), Some(&first.id));

        // Second account doesn't change active
        let second = service
            .create_account(CreateAccountRequest::gmail("second@gmail.com"))
            .await
            .unwrap();

        assert_eq!(service.active_account_id(), Some(&first.id));

        // Can switch active account
        service.set_active_account(&second.id).await.unwrap();
        assert_eq!(service.active_account_id(), Some(&second.id));
    }

    #[tokio::test]
    async fn store_and_retrieve_oauth_tokens() {
        let mut service = create_service();

        let account = service
            .create_account(CreateAccountRequest::gmail("test@gmail.com"))
            .await
            .unwrap();

        service
            .store_oauth_tokens(&account.id, "access123", Some("refresh456"))
            .await
            .unwrap();

        let tokens = service.get_oauth_tokens(&account.id).await.unwrap();
        assert!(tokens.is_some());

        let (access, refresh) = tokens.unwrap();
        assert_eq!(access, "access123");
        assert_eq!(refresh, Some("refresh456".to_string()));
    }

    #[tokio::test]
    async fn store_and_retrieve_password() {
        let mut service = create_service();

        let account = service
            .create_account(CreateAccountRequest::imap(
                "test@example.com",
                "imap.example.com",
                993,
                "smtp.example.com",
                587,
                true,
            ))
            .await
            .unwrap();

        service
            .store_password(&account.id, "secret123")
            .await
            .unwrap();

        let password = service.get_password(&account.id).await.unwrap();
        assert_eq!(password, Some("secret123".to_string()));
    }

    #[tokio::test]
    async fn account_stats() {
        let mut service = create_service();

        service
            .create_account(CreateAccountRequest::gmail("gmail@example.com"))
            .await
            .unwrap();

        service
            .create_account(
                CreateAccountRequest::imap(
                    "imap@example.com",
                    "imap.example.com",
                    993,
                    "smtp.example.com",
                    587,
                    true,
                )
                .sync_disabled(),
            )
            .await
            .unwrap();

        let stats = service.get_stats().await.unwrap();

        assert_eq!(stats.total_accounts, 2);
        assert_eq!(stats.gmail_accounts, 1);
        assert_eq!(stats.imap_accounts, 1);
        assert_eq!(stats.sync_enabled_count, 1);
    }

    #[test]
    fn email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@subdomain.example.com"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("test@"));
        assert!(!is_valid_email("test@com"));
    }

    #[test]
    fn provider_config_validation() {
        assert!(validate_provider_config(&ProviderType::Gmail, &ProviderConfig::Gmail {}).is_ok());

        assert!(validate_provider_config(
            &ProviderType::Imap,
            &ProviderConfig::Imap {
                imap_host: "imap.example.com".into(),
                imap_port: 993,
                smtp_host: "smtp.example.com".into(),
                smtp_port: 587,
                use_tls: true,
            }
        )
        .is_ok());

        // Mismatched type and config
        assert!(validate_provider_config(
            &ProviderType::Gmail,
            &ProviderConfig::Imap {
                imap_host: "imap.example.com".into(),
                imap_port: 993,
                smtp_host: "smtp.example.com".into(),
                smtp_port: 587,
                use_tls: true,
            }
        )
        .is_err());

        // Empty hosts
        assert!(validate_provider_config(
            &ProviderType::Imap,
            &ProviderConfig::Imap {
                imap_host: "".into(),
                imap_port: 993,
                smtp_host: "smtp.example.com".into(),
                smtp_port: 587,
                use_tls: true,
            }
        )
        .is_err());
    }
}
