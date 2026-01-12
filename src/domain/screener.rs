//! Screener domain types.
//!
//! Represents the email screener system for filtering unknown senders.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::EmailId;

/// An entry in the screener queue for an unknown sender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerEntry {
    /// Unique identifier for this entry.
    pub id: String,
    /// Email address of the sender.
    pub sender_email: String,
    /// Display name of the sender.
    pub sender_name: Option<String>,
    /// ID of the first email from this sender.
    pub first_email_id: Option<EmailId>,
    /// Current screening status.
    pub status: ScreenerStatus,
    /// AI analysis of the sender.
    pub ai_analysis: Option<SenderAnalysis>,
    /// When a decision was made.
    pub decided_at: Option<DateTime<Utc>>,
    /// When this entry was created.
    pub created_at: DateTime<Utc>,
}

/// Status of a screener entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenerStatus {
    /// Awaiting review.
    Pending,
    /// Sender approved, emails go to inbox.
    Approved,
    /// Sender rejected, emails blocked.
    Rejected,
}

/// A rule for automatic screening decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerRule {
    /// Unique identifier for this rule.
    pub id: String,
    /// Type of rule.
    pub rule_type: RuleType,
    /// Pattern to match (domain, email pattern, etc.).
    pub pattern: String,
    /// Action to take when matched.
    pub action: ScreenerAction,
    /// When this rule was created.
    pub created_at: DateTime<Utc>,
}

/// Type of screener rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    /// Allow all emails from a domain.
    DomainAllow,
    /// Block all emails from a domain.
    DomainBlock,
    /// Pattern-based matching (regex or glob).
    Pattern,
}

/// Categorization of a sender type by AI analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SenderType {
    /// A person the user has corresponded with before.
    KnownContact,
    /// A newsletter or mailing list.
    Newsletter,
    /// Marketing or promotional email.
    Marketing,
    /// Recruiter or job-related.
    Recruiter,
    /// Customer support or transactional.
    Support,
    /// Cannot be categorized.
    Unknown,
}

/// Action to take for a screened sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenerAction {
    /// Approve the sender, deliver to inbox.
    Approve,
    /// Reject the sender, block future emails.
    Reject,
    /// Keep in review queue for manual decision.
    Review,
}

/// AI-generated analysis of a sender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderAnalysis {
    /// Likely type of sender.
    pub likely_type: SenderType,
    /// Explanation of the classification.
    pub reasoning: String,
    /// Suggested action based on analysis.
    pub suggested_action: ScreenerAction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screener_entry_serialization() {
        let entry = ScreenerEntry {
            id: "entry-1".to_string(),
            sender_email: "unknown@example.com".to_string(),
            sender_name: Some("Unknown Sender".to_string()),
            first_email_id: Some(EmailId::from("email-1")),
            status: ScreenerStatus::Pending,
            ai_analysis: None,
            decided_at: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ScreenerEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.sender_email, "unknown@example.com");
        assert_eq!(deserialized.status, ScreenerStatus::Pending);
    }

    #[test]
    fn screener_rule_serialization() {
        let rule = ScreenerRule {
            id: "rule-1".to_string(),
            rule_type: RuleType::DomainAllow,
            pattern: "trusted.com".to_string(),
            action: ScreenerAction::Approve,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: ScreenerRule = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.rule_type, RuleType::DomainAllow);
        assert_eq!(deserialized.action, ScreenerAction::Approve);
    }

    #[test]
    fn sender_analysis_serialization() {
        let analysis = SenderAnalysis {
            likely_type: SenderType::Newsletter,
            reasoning: "Contains unsubscribe link and weekly format".to_string(),
            suggested_action: ScreenerAction::Review,
        };

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: SenderAnalysis = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.likely_type, SenderType::Newsletter);
        assert_eq!(deserialized.suggested_action, ScreenerAction::Review);
    }

    #[test]
    fn screener_status_equality() {
        assert_eq!(ScreenerStatus::Pending, ScreenerStatus::Pending);
        assert_ne!(ScreenerStatus::Pending, ScreenerStatus::Approved);
    }

    #[test]
    fn sender_type_variants() {
        let types = [
            SenderType::KnownContact,
            SenderType::Newsletter,
            SenderType::Marketing,
            SenderType::Recruiter,
            SenderType::Support,
            SenderType::Unknown,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let deserialized: SenderType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, deserialized);
        }
    }

    #[test]
    fn rule_type_variants() {
        assert_eq!(
            serde_json::to_string(&RuleType::DomainAllow).unwrap(),
            "\"domain_allow\""
        );
        assert_eq!(
            serde_json::to_string(&RuleType::DomainBlock).unwrap(),
            "\"domain_block\""
        );
        assert_eq!(
            serde_json::to_string(&RuleType::Pattern).unwrap(),
            "\"pattern\""
        );
    }
}
