//! Keychain access for secure credential storage.
//!
//! Wraps the keyring crate to provide OS-native credential storage.

use thiserror::Error;

/// Errors that can occur during keychain operations.
#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("Keychain error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("Credential not found: {0}")]
    NotFound(String),

    #[error("Failed to spawn blocking task: {0}")]
    TaskFailed(String),
}

/// Result type for keychain operations.
pub type Result<T> = std::result::Result<T, KeychainError>;

/// Provides access to the OS keychain for credential storage.
///
/// Credentials are stored using the service name as a namespace,
/// allowing multiple credentials to be stored per account.
#[derive(Debug, Clone)]
pub struct KeychainAccess {
    service_name: String,
}

impl KeychainAccess {
    /// Default service name for margin credentials.
    pub const DEFAULT_SERVICE: &'static str = "io.margin.app";

    /// Creates a new KeychainAccess with the default service name.
    pub fn new() -> Self {
        Self {
            service_name: Self::DEFAULT_SERVICE.to_string(),
        }
    }

    /// Creates a new KeychainAccess with a custom service name.
    ///
    /// Useful for testing to avoid interfering with real credentials.
    pub fn with_service(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Stores a credential in the keychain.
    ///
    /// If a credential with the same key already exists, it is overwritten.
    pub async fn store(&self, key: &str, value: &str) -> Result<()> {
        let service = self.service_name.clone();
        let key = key.to_string();
        let value = value.to_string();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &key)?;
            entry.set_password(&value)?;
            Ok(())
        })
        .await
        .map_err(|e| KeychainError::TaskFailed(e.to_string()))?
    }

    /// Retrieves a credential from the keychain.
    ///
    /// Returns `None` if no credential exists for the key.
    pub async fn retrieve(&self, key: &str) -> Result<Option<String>> {
        let service = self.service_name.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &key)?;
            match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(KeychainError::Keyring(e)),
            }
        })
        .await
        .map_err(|e| KeychainError::TaskFailed(e.to_string()))?
    }

    /// Deletes a credential from the keychain.
    ///
    /// Returns an error if the credential does not exist.
    pub async fn delete(&self, key: &str) -> Result<()> {
        let service = self.service_name.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &key)?;
            match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::Error::NoEntry) => Err(KeychainError::NotFound(key)),
                Err(e) => Err(KeychainError::Keyring(e)),
            }
        })
        .await
        .map_err(|e| KeychainError::TaskFailed(e.to_string()))?
    }

    /// Checks if a credential exists in the keychain.
    pub async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.retrieve(key).await?.is_some())
    }

    /// Returns the service name used for this keychain access.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Generates a keychain key for an account's OAuth access token.
    pub fn oauth_access_token_key(account_id: &str) -> String {
        format!("oauth.access_token.{}", account_id)
    }

    /// Generates a keychain key for an account's OAuth refresh token.
    pub fn oauth_refresh_token_key(account_id: &str) -> String {
        format!("oauth.refresh_token.{}", account_id)
    }

    /// Generates a keychain key for an account's IMAP password.
    pub fn imap_password_key(account_id: &str) -> String {
        format!("imap.password.{}", account_id)
    }

    /// Generates a keychain key for an AI provider's API key.
    pub fn ai_api_key(provider: &str) -> String {
        format!("ai.api_key.{}", provider)
    }
}

impl Default for KeychainAccess {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_service_name() {
        let keychain = KeychainAccess::new();
        assert_eq!(keychain.service_name(), KeychainAccess::DEFAULT_SERVICE);
    }

    #[test]
    fn custom_service_name() {
        let keychain = KeychainAccess::with_service("test.service");
        assert_eq!(keychain.service_name(), "test.service");
    }

    #[test]
    fn oauth_access_token_key_format() {
        let key = KeychainAccess::oauth_access_token_key("account-123");
        assert_eq!(key, "oauth.access_token.account-123");
    }

    #[test]
    fn oauth_refresh_token_key_format() {
        let key = KeychainAccess::oauth_refresh_token_key("account-123");
        assert_eq!(key, "oauth.refresh_token.account-123");
    }

    #[test]
    fn imap_password_key_format() {
        let key = KeychainAccess::imap_password_key("account-456");
        assert_eq!(key, "imap.password.account-456");
    }

    #[test]
    fn ai_api_key_format() {
        let key = KeychainAccess::ai_api_key("anthropic");
        assert_eq!(key, "ai.api_key.anthropic");
    }

    #[test]
    fn keychain_is_clone() {
        let keychain1 = KeychainAccess::new();
        let keychain2 = keychain1.clone();
        assert_eq!(keychain1.service_name(), keychain2.service_name());
    }

    // Integration tests that actually hit the keychain are skipped by default
    // because they require OS-level permissions and may leave artifacts.
    // Run with: cargo test --features keychain-integration-tests -- --ignored
    #[cfg(feature = "keychain-integration-tests")]
    mod integration {
        use super::*;

        #[tokio::test]
        #[ignore = "requires OS keychain access"]
        async fn store_retrieve_delete_cycle() {
            let keychain = KeychainAccess::with_service("io.margin.test");
            let key = "test-credential";
            let value = "test-secret-value";

            keychain.store(key, value).await.unwrap();

            let retrieved = keychain.retrieve(key).await.unwrap();
            assert_eq!(retrieved, Some(value.to_string()));

            keychain.delete(key).await.unwrap();

            let after_delete = keychain.retrieve(key).await.unwrap();
            assert_eq!(after_delete, None);
        }
    }
}
