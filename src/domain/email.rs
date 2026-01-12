//! Email domain types.
//!
//! Represents individual email messages and related structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AccountId, EmailId, LabelId, MessageId, ThreadId};

/// An individual email message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    /// Unique identifier for this email.
    pub id: EmailId,
    /// Account this email belongs to.
    pub account_id: AccountId,
    /// Thread (conversation) this email belongs to.
    pub thread_id: ThreadId,
    /// RFC 5322 Message-ID header.
    pub message_id: MessageId,
    /// Message-ID of the email this is replying to.
    pub in_reply_to: Option<MessageId>,
    /// Chain of Message-IDs for threading.
    pub references: Vec<MessageId>,
    /// Sender address.
    pub from: Address,
    /// Primary recipient addresses.
    pub to: Vec<Address>,
    /// Carbon copy recipient addresses.
    pub cc: Vec<Address>,
    /// Blind carbon copy recipient addresses.
    pub bcc: Vec<Address>,
    /// Email subject line.
    pub subject: Option<String>,
    /// Plain text body content.
    pub body_text: Option<String>,
    /// HTML body content.
    pub body_html: Option<String>,
    /// Short preview of the email content.
    pub snippet: String,
    /// Date and time the email was sent.
    pub date: DateTime<Utc>,
    /// Whether the email has been read.
    pub is_read: bool,
    /// Whether the email is starred/flagged.
    pub is_starred: bool,
    /// Whether this is a draft.
    pub is_draft: bool,
    /// Labels applied to this email.
    pub labels: Vec<LabelId>,
    /// File attachments.
    pub attachments: Vec<Attachment>,
}

/// An email address with optional display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// Email address.
    pub email: String,
    /// Display name (e.g., "John Doe").
    pub name: Option<String>,
}

impl Address {
    /// Creates a new address with just an email.
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: None,
        }
    }

    /// Creates a new address with email and display name.
    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: Some(name.into()),
        }
    }

    /// Returns the display representation of this address.
    ///
    /// If a name is present, returns `"Name <email>"`, otherwise just the email.
    pub fn display(&self) -> String {
        match &self.name {
            Some(name) => format!("{} <{}>", name, self.email),
            None => self.email.clone(),
        }
    }
}

/// A file attachment on an email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Unique identifier for this attachment.
    pub id: String,
    /// Original filename.
    pub filename: String,
    /// MIME content type.
    pub content_type: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Whether this is an inline attachment (e.g., embedded image).
    pub is_inline: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_display_with_name() {
        let addr = Address::with_name("test@example.com", "Test User");
        assert_eq!(addr.display(), "Test User <test@example.com>");
    }

    #[test]
    fn address_display_without_name() {
        let addr = Address::new("test@example.com");
        assert_eq!(addr.display(), "test@example.com");
    }

    #[test]
    fn address_equality() {
        let addr1 = Address::new("test@example.com");
        let addr2 = Address::new("test@example.com");
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn attachment_serialization() {
        let attachment = Attachment {
            id: "att-1".to_string(),
            filename: "document.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 1024,
            is_inline: false,
        };

        let json = serde_json::to_string(&attachment).unwrap();
        let deserialized: Attachment = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.filename, "document.pdf");
        assert_eq!(deserialized.size_bytes, 1024);
    }

    #[test]
    fn email_with_references() {
        let email = Email {
            id: EmailId::from("email-1"),
            account_id: AccountId::from("account-1"),
            thread_id: ThreadId::from("thread-1"),
            message_id: MessageId::from("<msg-3@example.com>"),
            in_reply_to: Some(MessageId::from("<msg-2@example.com>")),
            references: vec![
                MessageId::from("<msg-1@example.com>"),
                MessageId::from("<msg-2@example.com>"),
            ],
            from: Address::with_name("sender@example.com", "Sender"),
            to: vec![Address::new("recipient@example.com")],
            cc: vec![],
            bcc: vec![],
            subject: Some("Re: Test".to_string()),
            body_text: Some("Reply content".to_string()),
            body_html: None,
            snippet: "Reply content".to_string(),
            date: Utc::now(),
            is_read: false,
            is_starred: false,
            is_draft: false,
            labels: vec![LabelId::from("INBOX")],
            attachments: vec![],
        };

        assert_eq!(email.references.len(), 2);
        assert!(email.in_reply_to.is_some());
    }
}
