//! Thread domain types.
//!
//! Represents email threads (conversations) which group related messages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AccountId, Address, Email, LabelId, ThreadId};

/// A complete email thread with all messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Unique identifier for this thread.
    pub id: ThreadId,
    /// Account this thread belongs to.
    pub account_id: AccountId,
    /// Thread subject (from first message).
    pub subject: Option<String>,
    /// Short preview of the latest message.
    pub snippet: String,
    /// All participants in the thread.
    pub participants: Vec<Address>,
    /// All messages in the thread, ordered by date.
    pub messages: Vec<Email>,
    /// Date of the most recent message.
    pub last_message_date: DateTime<Utc>,
    /// Number of unread messages.
    pub unread_count: u32,
    /// Whether any message in the thread is starred.
    pub is_starred: bool,
    /// Labels applied to this thread.
    pub labels: Vec<LabelId>,
}

/// A lightweight summary of a thread for list display.
///
/// Contains only the essential information needed for rendering
/// in the message list, avoiding the cost of loading full message bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    /// Unique identifier for this thread.
    pub id: ThreadId,
    /// Account this thread belongs to.
    pub account_id: AccountId,
    /// Thread subject.
    pub subject: Option<String>,
    /// Short preview of the latest message.
    pub snippet: String,
    /// Primary sender for display.
    pub from: Address,
    /// Date of the most recent message.
    pub last_message_date: DateTime<Utc>,
    /// Total number of messages in the thread.
    pub message_count: u32,
    /// Number of unread messages.
    pub unread_count: u32,
    /// Whether any message in the thread is starred.
    pub is_starred: bool,
    /// Labels applied to this thread.
    pub labels: Vec<LabelId>,
}

impl ThreadSummary {
    /// Returns true if the thread has unread messages.
    pub fn has_unread(&self) -> bool {
        self.unread_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary() -> ThreadSummary {
        ThreadSummary {
            id: ThreadId::from("thread-1"),
            account_id: AccountId::from("account-1"),
            subject: Some("Test Subject".to_string()),
            snippet: "This is a preview...".to_string(),
            from: Address::with_name("sender@example.com", "Sender"),
            last_message_date: Utc::now(),
            message_count: 3,
            unread_count: 1,
            is_starred: false,
            labels: vec![LabelId::from("INBOX")],
        }
    }

    #[test]
    fn thread_summary_has_unread() {
        let summary = make_summary();
        assert!(summary.has_unread());
    }

    #[test]
    fn thread_summary_no_unread() {
        let mut summary = make_summary();
        summary.unread_count = 0;
        assert!(!summary.has_unread());
    }

    #[test]
    fn thread_summary_serialization() {
        let summary = make_summary();
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: ThreadSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.message_count, 3);
        assert_eq!(deserialized.subject, Some("Test Subject".to_string()));
    }

    #[test]
    fn thread_with_messages() {
        use super::super::{EmailId, MessageId};

        let thread = Thread {
            id: ThreadId::from("thread-1"),
            account_id: AccountId::from("account-1"),
            subject: Some("Discussion".to_string()),
            snippet: "Latest reply...".to_string(),
            participants: vec![
                Address::new("alice@example.com"),
                Address::new("bob@example.com"),
            ],
            messages: vec![Email {
                id: EmailId::from("email-1"),
                account_id: AccountId::from("account-1"),
                thread_id: ThreadId::from("thread-1"),
                message_id: MessageId::from("<msg-1@example.com>"),
                in_reply_to: None,
                references: vec![],
                from: Address::new("alice@example.com"),
                to: vec![Address::new("bob@example.com")],
                cc: vec![],
                bcc: vec![],
                subject: Some("Discussion".to_string()),
                body_text: Some("Hello".to_string()),
                body_html: None,
                snippet: "Hello".to_string(),
                date: Utc::now(),
                is_read: true,
                is_starred: false,
                is_draft: false,
                labels: vec![],
                attachments: vec![],
            }],
            last_message_date: Utc::now(),
            unread_count: 0,
            is_starred: false,
            labels: vec![LabelId::from("INBOX")],
        };

        assert_eq!(thread.participants.len(), 2);
        assert_eq!(thread.messages.len(), 1);
    }
}
