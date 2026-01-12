//! Account service for managing email accounts.
//!
//! Provides a service layer for account operations including:
//! - Account creation and configuration
//! - Credential storage and retrieval
//! - Account updates and deletion
//! - Active account management

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

    /// Invalid account configuration.
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

/// Storage abstraction for account operations.
#[async_trait]
pub trait AccountStorage: Send + Sync {
    /// Gets an account by ID.
    async fn get_account(&self, id: &AccountId) -> AccountResult<Option<Account>>;

    /// Gets an account by email.
    async fn get_by_email(&self, email: &str) -> AccountResult<Option<Account>>;

    /// Gets all accounts.
    async fn get_all_accounts(&self) -> AccountResult<Vec<Account>>;

    /// Inserts a new account.
    async fn insert_account(&self, account: &Account) -> AccountResult<()>;

    /// Updates an account.
    async fn update_account(&self, account: &Account) -> AccountResult<()>;

    /// Deletes an account.
    async fn delete_account(&self, id: &AccountId) -> AccountResult<()>;

    /// Counts total accounts.
    async fn count_accounts(&self) -> AccountResult<u32>;
}

/// Storage abstraction for credentials.
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// Stores a credential.
    async fn store(&self, key: &str, value: &str) -> AccountResult<()>;

    /// Retrieves a credential.
    async fn retrieve(&self, key: &str) -> AccountResult<Option<String>>;

    /// Deletes a credential.
    async fn delete(&self, key: &str) -> AccountResult<()>;
}

/// Request to create a new account.
#[derive(Debug, Clone)]
pub struct CreateAccountRequest {
    /// Email address.
    pub email: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Provider type.
    pub provider_type: ProviderType,
    /// Provider configuration.
    pub provider_config: ProviderConfig,
    /// Whether sync is enabled.
    pub sync_enabled: bool,
    /// Sync interval.
    pub sync_interval: Duration,
    /// Email signature.
    pub signature: Option<String>,
}

impl CreateAccountRequest {
    /// Creates a Gmail account request.
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

    /// Creates an IMAP account request.
    pub fn imap(
        email: impl Into<String>,
        imap_host: impl Into<String>,
        smtp_host: impl Into<String>,
    ) -> Self {
        Self {
            email: email.into(),
            display_name: None,
            provider_type: ProviderType::Imap,
            provider_config: ProviderConfig::Imap {
                imap_host: imap_host.into(),
                imap_port: 993,
                smtp_host: smtp_host.into(),
                smtp_port: 587,
                use_tls: true,
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
    pub fn signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = Some(sig.into());
        self
    }
}

/// Updates to apply to an account.
#[derive(Debug, Clone, Default)]
pub struct AccountUpdate {
    /// New display name.
    pub display_name: Option<String>,
    /// New sync enabled status.
    pub sync_enabled: Option<bool>,
    /// New sync interval.
    pub sync_interval: Option<Duration>,
    /// New signature.
    pub signature: Option<String>,
}

impl AccountUpdate {
    /// Creates a new empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets sync enabled.
    pub fn sync_enabled(mut self, enabled: bool) -> Self {
        self.sync_enabled = Some(enabled);
        self
    }

    /// Sets the sync interval.
    pub fn sync_interval(mut self, interval: Duration) -> Self {
        self.sync_interval = Some(interval);
        self
    }

    /// Sets the signature.
    pub fn signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = Some(sig.into());
        self
    }

    /// Returns true if this update has no changes.
    pub fn is_empty(&self) -> bool {
        self.display_name.is_none()
            && self.sync_enabled.is_none()
            && self.sync_interval.is_none()
            && self.signature.is_none()
    }
}

/// Account statistics.
#[derive(Debug, Clone, Default)]
pub struct AccountStats {
    /// Total number of accounts.
    pub total_accounts: u32,
    /// Number of Gmail accounts.
    pub gmail_accounts: u32,
    /// Number of IMAP accounts.
    pub imap_accounts: u32,
    /// Number of sync-enabled accounts.
    pub sync_enabled_accounts: u32,
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

        // Check if account already exists
        if self.storage.get_by_email(&request.email).await?.is_some() {
            return Err(AccountError::AlreadyExists(request.email));
        }

        // Validate provider config
        validate_provider_config(&request.provider_type, &request.provider_config)?;

        // Create the account
        let account = Account {
            id: AccountId::from(format!("account-{}", uuid::Uuid::new_v4())),
            email: request.email,
            display_name: request.display_name,
            provider_type: request.provider_type,
            provider_config: request.provider_config,
            sync_enabled: request.sync_enabled,
            sync_interval: request.sync_interval,
            signature: request.signature,
        };

        self.storage.insert_account(&account).await?;

        // Set as active if this is the first account
        if self.active_account_id.is_none() {
            self.active_account_id = Some(account.id.clone());
        }

        Ok(account)
    }

    /// Gets an account by ID.
    pub async fn get_account(&self, id: &AccountId) -> AccountResult<Account> {
        self.storage
            .get_account(id)
            .await?
            .ok_or_else(|| AccountError::NotFound(id.to_string()))
    }

    /// Gets an account by email.
    pub async fn get_by_email(&self, email: &str) -> AccountResult<Account> {
        self.storage
            .get_by_email(email)
            .await?
            .ok_or_else(|| AccountError::NotFound(email.to_string()))
    }

    /// Gets all accounts.
    pub async fn get_all_accounts(&self) -> AccountResult<Vec<Account>> {
        self.storage.get_all_accounts().await
    }

    /// Updates an account.
    pub async fn update_account(
        &self,
        id: &AccountId,
        update: AccountUpdate,
    ) -> AccountResult<Account> {
        if update.is_empty() {
            return self.get_account(id).await;
        }

        let mut account = self.get_account(id).await?;

        if let Some(display_name) = update.display_name {
            account.display_name = Some(display_name);
        }
        if let Some(sync_enabled) = update.sync_enabled {
            account.sync_enabled = sync_enabled;
        }
        if let Some(sync_interval) = update.sync_interval {
            account.sync_interval = sync_interval;
        }
        if let Some(signature) = update.signature {
            account.signature = Some(signature);
        }

        self.storage.update_account(&account).await?;

        Ok(account)
    }

    /// Deletes an account.
    pub async fn delete_account(&mut self, id: &AccountId) -> AccountResult<()> {
        // Verify account exists
        self.get_account(id).await?;

        // Delete associated credentials
        let oauth_key = format!("oauth:{}", id);
        let password_key = format!("password:{}", id);
        let _ = self.credentials.delete(&oauth_key).await;
        let _ = self.credentials.delete(&password_key).await;

        // Delete the account
        self.storage.delete_account(id).await?;

        // Clear active account if it was deleted
        if self.active_account_id.as_ref() == Some(id) {
            self.active_account_id = None;
        }

        Ok(())
    }

    /// Gets the active account.
    pub async fn get_active_account(&self) -> AccountResult<Option<Account>> {
        match &self.active_account_id {
            Some(id) => Ok(Some(self.get_account(id).await?)),
            None => Ok(None),
        }
    }

    /// Sets the active account.
    pub async fn set_active_account(&mut self, id: &AccountId) -> AccountResult<()> {
        // Verify account exists
        self.get_account(id).await?;
        self.active_account_id = Some(id.clone());
        Ok(())
    }

    /// Gets the active account ID.
    pub fn active_account_id(&self) -> Option<&AccountId> {
        self.active_account_id.as_ref()
    }

    /// Stores OAuth tokens for an account.
    pub async fn store_oauth_tokens(
        &self,
        account_id: &AccountId,
        access_token: &str,
        refresh_token: &str,
    ) -> AccountResult<()> {
        let key = format!("oauth:{}", account_id);
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
    ) -> AccountResult<Option<(String, String)>> {
        let key = format!("oauth:{}", account_id);
        match self.credentials.retrieve(&key).await? {
            Some(value) => {
                let parsed: serde_json::Value = serde_json::from_str(&value)
                    .map_err(|e| AccountError::CredentialError(e.to_string()))?;
                let access_token = parsed["access_token"]
                    .as_str()
                    .ok_or_else(|| AccountError::CredentialError("missing access_token".into()))?
                    .to_string();
                let refresh_token = parsed["refresh_token"]
                    .as_str()
                    .ok_or_else(|| AccountError::CredentialError("missing refresh_token".into()))?
                    .to_string();
                Ok(Some((access_token, refresh_token)))
            }
            None => Ok(None),
        }
    }

    /// Stores a password for an account.
    pub async fn store_password(
        &self,
        account_id: &AccountId,
        password: &str,
    ) -> AccountResult<()> {
        let key = format!("password:{}", account_id);
        self.credentials.store(&key, password).await
    }

    /// Retrieves a password for an account.
    pub async fn get_password(&self, account_id: &AccountId) -> AccountResult<Option<String>> {
        let key = format!("password:{}", account_id);
        self.credentials.retrieve(&key).await
    }

    /// Gets account statistics.
    pub async fn get_stats(&self) -> AccountResult<AccountStats> {
        let accounts = self.storage.get_all_accounts().await?;

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
                stats.sync_enabled_accounts += 1;
            }
        }

        Ok(stats)
    }

    /// Counts total accounts.
    pub async fn count(&self) -> AccountResult<u32> {
        self.storage.count_accounts().await
    }
}

/// Validates an email address format.
fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];

    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

/// Validates provider configuration.
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
            "provider type and config mismatch".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockStorage {
        accounts: Mutex<HashMap<AccountId, Account>>,
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
        async fn get_account(&self, id: &AccountId) -> AccountResult<Option<Account>> {
            let accounts = self.accounts.lock().unwrap();
            Ok(accounts.get(id).cloned())
        }

        async fn get_by_email(&self, email: &str) -> AccountResult<Option<Account>> {
            let accounts = self.accounts.lock().unwrap();
            Ok(accounts.values().find(|a| a.email == email).cloned())
        }

        async fn get_all_accounts(&self) -> AccountResult<Vec<Account>> {
            let accounts = self.accounts.lock().unwrap();
            Ok(accounts.values().cloned().collect())
        }

        async fn insert_account(&self, account: &Account) -> AccountResult<()> {
            let mut accounts = self.accounts.lock().unwrap();
            accounts.insert(account.id.clone(), account.clone());
            Ok(())
        }

        async fn update_account(&self, account: &Account) -> AccountResult<()> {
            let mut accounts = self.accounts.lock().unwrap();
            accounts.insert(account.id.clone(), account.clone());
            Ok(())
        }

        async fn delete_account(&self, id: &AccountId) -> AccountResult<()> {
            let mut accounts = self.accounts.lock().unwrap();
            accounts.remove(id);
            Ok(())
        }

        async fn count_accounts(&self) -> AccountResult<u32> {
            let accounts = self.accounts.lock().unwrap();
            Ok(accounts.len() as u32)
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
            let mut store = self.store.lock().unwrap();
            store.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn retrieve(&self, key: &str) -> AccountResult<Option<String>> {
            let store = self.store.lock().unwrap();
            Ok(store.get(key).cloned())
        }

        async fn delete(&self, key: &str) -> AccountResult<()> {
            let mut store = self.store.lock().unwrap();
            store.remove(key);
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
    }

    #[tokio::test]
    async fn create_imap_account() {
        let mut service = create_service();

        let request =
            CreateAccountRequest::imap("test@example.com", "imap.example.com", "smtp.example.com")
                .signature("Best regards");

        let account = service.create_account(request).await.unwrap();

        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.provider_type, ProviderType::Imap);
        assert_eq!(account.signature, Some("Best regards".to_string()));
    }

    #[tokio::test]
    async fn create_account_duplicate() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        service.create_account(request.clone()).await.unwrap();

        let result = service.create_account(request).await;
        assert!(matches!(result, Err(AccountError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn create_account_invalid_email() {
        let mut service = create_service();

        let mut request = CreateAccountRequest::gmail("invalid");
        request.email = "invalid".to_string();

        let result = service.create_account(request).await;
        assert!(matches!(result, Err(AccountError::InvalidConfig(_))));
    }

    #[tokio::test]
    async fn get_account() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let created = service.create_account(request).await.unwrap();

        let fetched = service.get_account(&created.id).await.unwrap();
        assert_eq!(fetched.email, "test@gmail.com");
    }

    #[tokio::test]
    async fn get_account_not_found() {
        let service = create_service();

        let result = service.get_account(&AccountId::from("nonexistent")).await;
        assert!(matches!(result, Err(AccountError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_account() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let created = service.create_account(request).await.unwrap();

        let update = AccountUpdate::new()
            .display_name("New Name")
            .sync_enabled(false);

        let updated = service.update_account(&created.id, update).await.unwrap();

        assert_eq!(updated.display_name, Some("New Name".to_string()));
        assert!(!updated.sync_enabled);
    }

    #[tokio::test]
    async fn delete_account() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let created = service.create_account(request).await.unwrap();

        service.delete_account(&created.id).await.unwrap();

        let result = service.get_account(&created.id).await;
        assert!(matches!(result, Err(AccountError::NotFound(_))));
    }

    #[tokio::test]
    async fn active_account() {
        let mut service = create_service();

        // No active account initially
        let active = service.get_active_account().await.unwrap();
        assert!(active.is_none());

        // First account becomes active
        let request = CreateAccountRequest::gmail("first@gmail.com");
        let first = service.create_account(request).await.unwrap();

        let active = service.get_active_account().await.unwrap();
        assert_eq!(active.unwrap().id, first.id);

        // Create second account and switch to it
        let request = CreateAccountRequest::gmail("second@gmail.com");
        let second = service.create_account(request).await.unwrap();

        service.set_active_account(&second.id).await.unwrap();
        let active = service.get_active_account().await.unwrap();
        assert_eq!(active.unwrap().id, second.id);
    }

    #[tokio::test]
    async fn oauth_tokens() {
        let mut service = create_service();

        let request = CreateAccountRequest::gmail("test@gmail.com");
        let account = service.create_account(request).await.unwrap();

        // Store tokens
        service
            .store_oauth_tokens(&account.id, "access123", "refresh456")
            .await
            .unwrap();

        // Retrieve tokens
        let tokens = service.get_oauth_tokens(&account.id).await.unwrap();
        let (access, refresh) = tokens.unwrap();
        assert_eq!(access, "access123");
        assert_eq!(refresh, "refresh456");
    }

    #[tokio::test]
    async fn password_storage() {
        let mut service = create_service();

        let request =
            CreateAccountRequest::imap("test@example.com", "imap.example.com", "smtp.example.com");
        let account = service.create_account(request).await.unwrap();

        // Store password
        service
            .store_password(&account.id, "secret123")
            .await
            .unwrap();

        // Retrieve password
        let password = service.get_password(&account.id).await.unwrap();
        assert_eq!(password, Some("secret123".to_string()));
    }

    #[tokio::test]
    async fn get_stats() {
        let mut service = create_service();

        service
            .create_account(CreateAccountRequest::gmail("g1@gmail.com"))
            .await
            .unwrap();
        service
            .create_account(CreateAccountRequest::gmail("g2@gmail.com").sync_disabled())
            .await
            .unwrap();
        service
            .create_account(CreateAccountRequest::imap(
                "test@example.com",
                "imap.example.com",
                "smtp.example.com",
            ))
            .await
            .unwrap();

        let stats = service.get_stats().await.unwrap();

        assert_eq!(stats.total_accounts, 3);
        assert_eq!(stats.gmail_accounts, 2);
        assert_eq!(stats.imap_accounts, 1);
        assert_eq!(stats.sync_enabled_accounts, 2);
    }

    #[tokio::test]
    async fn email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@domain.org"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@domain"));
    }

    #[tokio::test]
    async fn provider_config_validation() {
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
