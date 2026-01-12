//! Email provider trait definition.
//!
//! This module defines the [`EmailProvider`] trait which abstracts over different
//! email backends (Gmail API, IMAP/SMTP, etc.). All email providers must implement
//! this trait to be used by the application's sync and email services.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    Address, EmailId, Label, LabelId, ProviderType, Thread, ThreadId, ThreadSummary,
};

/// Result type alias for email provider operations.
pub type Result<T> = std::result::Result<T, ProviderError>;

/// Errors that can occur during email provider operations.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Authentication failed or credentials expired.
    #[error("authentication failed: {0}")]
    Authentication(String),

    /// Network or connection error.
    #[error("connection error: {0}")]
    Connection(String),

    /// Rate limit exceeded.
    #[error("rate limit exceeded, retry after {retry_after_secs:?} seconds")]
    RateLimited {
        /// Seconds to wait before retrying, if known.
        retry_after_secs: Option<u64>,
    },

    /// Requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Invalid request or parameters.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Provider-specific error.
    #[error("provider error: {0}")]
    Provider(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Pagination parameters for list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pagination {
    /// Maximum number of items to return.
    pub limit: Option<u32>,
    /// Opaque cursor for the next page of results.
    pub page_token: Option<String>,
}

impl Pagination {
    /// Creates a new pagination with the specified limit.
    pub fn with_limit(limit: u32) -> Self {
        Self {
            limit: Some(limit),
            page_token: None,
        }
    }

    /// Creates pagination for the next page using the provided token.
    pub fn next_page(token: impl Into<String>) -> Self {
        Self {
            limit: None,
            page_token: Some(token.into()),
        }
    }
}

/// A change detected during sync.
///
/// Used by the sync service to apply incremental updates to local storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    /// A new email was received.
    NewEmail(NewEmailData),
    /// An existing email was updated (labels, read status, etc.).
    Updated(EmailUpdate),
    /// An email was deleted.
    Deleted(EmailId),
}

/// Data for a newly received email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEmailData {
    /// The email ID assigned by the provider.
    pub id: EmailId,
    /// The thread this email belongs to.
    pub thread_id: ThreadId,
    /// Sender address.
    pub from: Address,
    /// Recipient addresses.
    pub to: Vec<Address>,
    /// CC addresses.
    pub cc: Vec<Address>,
    /// Email subject.
    pub subject: Option<String>,
    /// Short preview of email content.
    pub snippet: String,
    /// Date the email was sent.
    pub date: DateTime<Utc>,
    /// Labels applied to this email.
    pub labels: Vec<LabelId>,
    /// Whether the email has been read.
    pub is_read: bool,
    /// Whether the email is starred.
    pub is_starred: bool,
    /// Raw email content (RFC 5322 format) if available.
    pub raw: Option<Vec<u8>>,
}

/// Updates to an existing email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailUpdate {
    /// ID of the email being updated.
    pub id: EmailId,
    /// New labels, if changed.
    pub labels: Option<Vec<LabelId>>,
    /// New read status, if changed.
    pub is_read: Option<bool>,
    /// New starred status, if changed.
    pub is_starred: Option<bool>,
}

/// A local change pending sync to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    /// Unique ID for this pending change.
    pub id: String,
    /// Type of change to apply.
    pub change_type: PendingChangeType,
    /// When this change was created.
    pub created_at: DateTime<Utc>,
}

/// Types of pending changes that can be synced to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PendingChangeType {
    /// Mark thread(s) as read or unread.
    MarkRead {
        /// Thread IDs to update.
        thread_ids: Vec<ThreadId>,
        /// Whether to mark as read (true) or unread (false).
        read: bool,
    },
    /// Star or unstar a thread.
    Star {
        /// Thread ID to update.
        thread_id: ThreadId,
        /// Whether to star (true) or unstar (false).
        starred: bool,
    },
    /// Archive thread(s).
    Archive {
        /// Thread IDs to archive.
        thread_ids: Vec<ThreadId>,
    },
    /// Move thread(s) to trash.
    Trash {
        /// Thread IDs to trash.
        thread_ids: Vec<ThreadId>,
    },
    /// Apply a label to thread(s).
    ApplyLabel {
        /// Thread IDs to update.
        thread_ids: Vec<ThreadId>,
        /// Label to apply.
        label_id: LabelId,
    },
    /// Remove a label from thread(s).
    RemoveLabel {
        /// Thread IDs to update.
        thread_ids: Vec<ThreadId>,
        /// Label to remove.
        label_id: LabelId,
    },
    /// Send an email.
    Send {
        /// The outgoing email to send.
        email: OutgoingEmail,
    },
}

/// An email to be sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingEmail {
    /// Recipient addresses.
    pub to: Vec<Address>,
    /// CC addresses.
    pub cc: Vec<Address>,
    /// BCC addresses.
    pub bcc: Vec<Address>,
    /// Email subject.
    pub subject: String,
    /// Plain text body.
    pub body_text: String,
    /// HTML body (optional).
    pub body_html: Option<String>,
    /// Thread ID if this is a reply.
    pub in_reply_to_thread: Option<ThreadId>,
    /// Message-ID of the email being replied to.
    pub in_reply_to_message: Option<String>,
    /// Attachment data.
    pub attachments: Vec<OutgoingAttachment>,
}

/// An attachment to be sent with an outgoing email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingAttachment {
    /// Filename for the attachment.
    pub filename: String,
    /// MIME content type.
    pub content_type: String,
    /// Raw attachment data.
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
}

mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        STANDARD.encode(bytes).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD
            .decode(&s)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

/// Trait for email provider implementations.
///
/// This trait abstracts over different email backends such as Gmail API and
/// standard IMAP/SMTP. Implementations handle authentication, fetching emails,
/// sending emails, and syncing changes.
///
/// All methods are async and return [`Result`] to handle provider-specific errors.
///
/// # Example
///
/// ```ignore
/// use margin::providers::email::{EmailProvider, Pagination};
///
/// async fn list_inbox(provider: &impl EmailProvider) -> Result<()> {
///     let threads = provider
///         .fetch_threads("INBOX", Pagination::with_limit(50))
///         .await?;
///
///     for thread in threads {
///         println!("{}: {}", thread.from.display(), thread.subject.unwrap_or_default());
///     }
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Returns the type of this provider.
    fn provider_type(&self) -> ProviderType;

    /// Authenticates with the email provider.
    ///
    /// For OAuth-based providers (Gmail), this may refresh tokens if needed.
    /// For IMAP providers, this establishes a connection.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Authentication`] if credentials are invalid or expired.
    async fn authenticate(&mut self) -> Result<()>;

    /// Fetches thread summaries from a folder.
    ///
    /// # Arguments
    ///
    /// * `folder` - The folder/label to fetch from (e.g., "INBOX", "SENT")
    /// * `pagination` - Pagination parameters
    ///
    /// # Returns
    ///
    /// A list of thread summaries suitable for display in the message list.
    async fn fetch_threads(
        &self,
        folder: &str,
        pagination: Pagination,
    ) -> Result<Vec<ThreadSummary>>;

    /// Fetches a complete thread with all messages.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread identifier
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::NotFound`] if the thread does not exist.
    async fn fetch_thread(&self, thread_id: &str) -> Result<Thread>;

    /// Fetches changes since a given timestamp.
    ///
    /// Used for incremental sync to detect new emails, updates, and deletions.
    ///
    /// # Arguments
    ///
    /// * `since` - Fetch changes after this timestamp
    ///
    /// # Returns
    ///
    /// A list of changes to apply to local storage.
    async fn fetch_changes_since(&self, since: &DateTime<Utc>) -> Result<Vec<Change>>;

    /// Sends an email.
    ///
    /// # Arguments
    ///
    /// * `email` - The email to send
    ///
    /// # Returns
    ///
    /// The message ID assigned by the provider.
    async fn send_email(&self, email: &OutgoingEmail) -> Result<String>;

    /// Archives the specified threads.
    ///
    /// Removes threads from the inbox without deleting them.
    ///
    /// # Arguments
    ///
    /// * `thread_ids` - IDs of threads to archive
    async fn archive(&self, thread_ids: &[String]) -> Result<()>;

    /// Moves the specified threads to trash.
    ///
    /// # Arguments
    ///
    /// * `thread_ids` - IDs of threads to trash
    async fn trash(&self, thread_ids: &[String]) -> Result<()>;

    /// Stars or unstars a thread.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - ID of the thread
    /// * `starred` - `true` to star, `false` to unstar
    async fn star(&self, thread_id: &str, starred: bool) -> Result<()>;

    /// Marks a thread as read or unread.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - ID of the thread
    /// * `read` - `true` to mark as read, `false` to mark as unread
    async fn mark_read(&self, thread_id: &str, read: bool) -> Result<()>;

    /// Applies a label to a thread.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - ID of the thread
    /// * `label` - Label name or ID to apply
    async fn apply_label(&self, thread_id: &str, label: &str) -> Result<()>;

    /// Fetches all labels for this account.
    ///
    /// # Returns
    ///
    /// A list of all labels including system labels (INBOX, SENT, etc.)
    /// and user-created labels.
    async fn fetch_labels(&self) -> Result<Vec<Label>>;

    /// Pushes a pending change to the server.
    ///
    /// Used to sync local changes (offline edits) to the email provider.
    ///
    /// # Arguments
    ///
    /// * `change` - The pending change to push
    async fn push_change(&self, change: &PendingChange) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_with_limit() {
        let page = Pagination::with_limit(25);
        assert_eq!(page.limit, Some(25));
        assert!(page.page_token.is_none());
    }

    #[test]
    fn pagination_next_page() {
        let page = Pagination::next_page("token123");
        assert!(page.limit.is_none());
        assert_eq!(page.page_token, Some("token123".to_string()));
    }

    #[test]
    fn pagination_default() {
        let page = Pagination::default();
        assert!(page.limit.is_none());
        assert!(page.page_token.is_none());
    }

    #[test]
    fn pagination_serialization() {
        let page = Pagination::with_limit(50);
        let json = serde_json::to_string(&page).unwrap();
        let deserialized: Pagination = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.limit, Some(50));
    }

    #[test]
    fn change_new_email_serialization() {
        let change = Change::NewEmail(NewEmailData {
            id: EmailId::from("email-1"),
            thread_id: ThreadId::from("thread-1"),
            from: Address::new("sender@example.com"),
            to: vec![Address::new("recipient@example.com")],
            cc: vec![],
            subject: Some("Test Subject".to_string()),
            snippet: "Preview text...".to_string(),
            date: Utc::now(),
            labels: vec![LabelId::from("INBOX")],
            is_read: false,
            is_starred: false,
            raw: None,
        });

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: Change = serde_json::from_str(&json).unwrap();

        if let Change::NewEmail(data) = deserialized {
            assert_eq!(data.id.0, "email-1");
            assert_eq!(data.subject, Some("Test Subject".to_string()));
        } else {
            panic!("Expected NewEmail variant");
        }
    }

    #[test]
    fn change_updated_serialization() {
        let change = Change::Updated(EmailUpdate {
            id: EmailId::from("email-1"),
            labels: Some(vec![LabelId::from("INBOX"), LabelId::from("STARRED")]),
            is_read: Some(true),
            is_starred: None,
        });

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: Change = serde_json::from_str(&json).unwrap();

        if let Change::Updated(update) = deserialized {
            assert_eq!(update.id.0, "email-1");
            assert_eq!(update.is_read, Some(true));
            assert!(update.is_starred.is_none());
        } else {
            panic!("Expected Updated variant");
        }
    }

    #[test]
    fn change_deleted_serialization() {
        let change = Change::Deleted(EmailId::from("email-to-delete"));

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: Change = serde_json::from_str(&json).unwrap();

        if let Change::Deleted(id) = deserialized {
            assert_eq!(id.0, "email-to-delete");
        } else {
            panic!("Expected Deleted variant");
        }
    }

    #[test]
    fn pending_change_mark_read_serialization() {
        let change = PendingChange {
            id: "change-1".to_string(),
            change_type: PendingChangeType::MarkRead {
                thread_ids: vec![ThreadId::from("thread-1"), ThreadId::from("thread-2")],
                read: true,
            },
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: PendingChange = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "change-1");
        if let PendingChangeType::MarkRead { thread_ids, read } = deserialized.change_type {
            assert_eq!(thread_ids.len(), 2);
            assert!(read);
        } else {
            panic!("Expected MarkRead variant");
        }
    }

    #[test]
    fn pending_change_star_serialization() {
        let change = PendingChange {
            id: "change-2".to_string(),
            change_type: PendingChangeType::Star {
                thread_id: ThreadId::from("thread-1"),
                starred: true,
            },
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"type\":\"star\""));

        let deserialized: PendingChange = serde_json::from_str(&json).unwrap();
        if let PendingChangeType::Star { thread_id, starred } = deserialized.change_type {
            assert_eq!(thread_id.0, "thread-1");
            assert!(starred);
        } else {
            panic!("Expected Star variant");
        }
    }

    #[test]
    fn pending_change_archive_serialization() {
        let change = PendingChange {
            id: "change-3".to_string(),
            change_type: PendingChangeType::Archive {
                thread_ids: vec![ThreadId::from("thread-1")],
            },
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: PendingChange = serde_json::from_str(&json).unwrap();

        if let PendingChangeType::Archive { thread_ids } = deserialized.change_type {
            assert_eq!(thread_ids.len(), 1);
        } else {
            panic!("Expected Archive variant");
        }
    }

    #[test]
    fn pending_change_apply_label_serialization() {
        let change = PendingChange {
            id: "change-4".to_string(),
            change_type: PendingChangeType::ApplyLabel {
                thread_ids: vec![ThreadId::from("thread-1")],
                label_id: LabelId::from("Work"),
            },
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: PendingChange = serde_json::from_str(&json).unwrap();

        if let PendingChangeType::ApplyLabel {
            thread_ids,
            label_id,
        } = deserialized.change_type
        {
            assert_eq!(thread_ids.len(), 1);
            assert_eq!(label_id.0, "Work");
        } else {
            panic!("Expected ApplyLabel variant");
        }
    }

    #[test]
    fn outgoing_email_serialization() {
        let email = OutgoingEmail {
            to: vec![Address::with_name("recipient@example.com", "Recipient")],
            cc: vec![],
            bcc: vec![],
            subject: "Test Subject".to_string(),
            body_text: "Plain text body".to_string(),
            body_html: Some("<p>HTML body</p>".to_string()),
            in_reply_to_thread: None,
            in_reply_to_message: None,
            attachments: vec![],
        };

        let json = serde_json::to_string(&email).unwrap();
        let deserialized: OutgoingEmail = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.subject, "Test Subject");
        assert_eq!(deserialized.to.len(), 1);
        assert_eq!(deserialized.to[0].email, "recipient@example.com");
    }

    #[test]
    fn outgoing_email_with_reply_serialization() {
        let email = OutgoingEmail {
            to: vec![Address::new("recipient@example.com")],
            cc: vec![],
            bcc: vec![],
            subject: "Re: Original Subject".to_string(),
            body_text: "Reply content".to_string(),
            body_html: None,
            in_reply_to_thread: Some(ThreadId::from("thread-1")),
            in_reply_to_message: Some("<original@example.com>".to_string()),
            attachments: vec![],
        };

        let json = serde_json::to_string(&email).unwrap();
        let deserialized: OutgoingEmail = serde_json::from_str(&json).unwrap();

        assert!(deserialized.in_reply_to_thread.is_some());
        assert!(deserialized.in_reply_to_message.is_some());
    }

    #[test]
    fn outgoing_attachment_serialization() {
        let attachment = OutgoingAttachment {
            filename: "document.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            data: vec![0x25, 0x50, 0x44, 0x46], // PDF magic bytes
        };

        let json = serde_json::to_string(&attachment).unwrap();
        let deserialized: OutgoingAttachment = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.filename, "document.pdf");
        assert_eq!(deserialized.data, vec![0x25, 0x50, 0x44, 0x46]);
    }

    #[test]
    fn provider_error_display() {
        let auth_err = ProviderError::Authentication("token expired".to_string());
        assert_eq!(auth_err.to_string(), "authentication failed: token expired");

        let rate_err = ProviderError::RateLimited {
            retry_after_secs: Some(60),
        };
        assert!(rate_err.to_string().contains("rate limit"));

        let not_found = ProviderError::NotFound("thread-123".to_string());
        assert!(not_found.to_string().contains("not found"));
    }

    #[test]
    fn email_update_partial_fields() {
        let update = EmailUpdate {
            id: EmailId::from("email-1"),
            labels: None,
            is_read: Some(true),
            is_starred: None,
        };

        let json = serde_json::to_string(&update).unwrap();
        let deserialized: EmailUpdate = serde_json::from_str(&json).unwrap();

        assert!(deserialized.labels.is_none());
        assert_eq!(deserialized.is_read, Some(true));
        assert!(deserialized.is_starred.is_none());
    }
}
