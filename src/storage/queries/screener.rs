//! Screener database queries.
//!
//! CRUD operations for the email screener system.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::domain::EmailId;
use crate::domain::{
    RuleType, ScreenerAction, ScreenerEntry, ScreenerRule, ScreenerStatus, SenderAnalysis,
};

/// Inserts a new screener entry.
pub fn insert_entry(conn: &Connection, entry: &ScreenerEntry) -> Result<()> {
    let ai_analysis_json = entry
        .ai_analysis
        .as_ref()
        .map(|a| serde_json::to_string(a).unwrap_or_default());

    conn.execute(
        "INSERT INTO screener_entries (id, sender_email, sender_name, first_email_id, status, ai_analysis, decided_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            entry.id,
            entry.sender_email,
            entry.sender_name,
            entry.first_email_id.as_ref().map(|id| id.0.as_str()),
            status_to_str(&entry.status),
            ai_analysis_json,
            entry.decided_at.map(|dt| dt.to_rfc3339()),
            entry.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Gets a screener entry by ID.
pub fn get_entry_by_id(conn: &Connection, id: &str) -> Result<Option<ScreenerEntry>> {
    conn.query_row(
        "SELECT id, sender_email, sender_name, first_email_id, status, ai_analysis, decided_at, created_at
         FROM screener_entries WHERE id = ?1",
        params![id],
        row_to_entry,
    )
    .optional()
}

/// Gets a screener entry by sender email.
pub fn get_entry_by_sender(conn: &Connection, sender_email: &str) -> Result<Option<ScreenerEntry>> {
    conn.query_row(
        "SELECT id, sender_email, sender_name, first_email_id, status, ai_analysis, decided_at, created_at
         FROM screener_entries WHERE sender_email = ?1",
        params![sender_email],
        row_to_entry,
    )
    .optional()
}

/// Gets all pending screener entries.
pub fn get_pending_entries(conn: &Connection) -> Result<Vec<ScreenerEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, sender_email, sender_name, first_email_id, status, ai_analysis, decided_at, created_at
         FROM screener_entries WHERE status = 'pending' ORDER BY created_at DESC",
    )?;

    let entries = stmt.query_map([], row_to_entry)?;
    entries.collect()
}

/// Gets screener entries by status.
pub fn get_entries_by_status(
    conn: &Connection,
    status: &ScreenerStatus,
) -> Result<Vec<ScreenerEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, sender_email, sender_name, first_email_id, status, ai_analysis, decided_at, created_at
         FROM screener_entries WHERE status = ?1 ORDER BY created_at DESC",
    )?;

    let entries = stmt.query_map(params![status_to_str(status)], row_to_entry)?;
    entries.collect()
}

/// Updates the status of a screener entry.
pub fn set_entry_status(conn: &Connection, id: &str, status: &ScreenerStatus) -> Result<()> {
    let decided_at = match status {
        ScreenerStatus::Pending => None,
        _ => Some(Utc::now().to_rfc3339()),
    };

    conn.execute(
        "UPDATE screener_entries SET status = ?1, decided_at = ?2 WHERE id = ?3",
        params![status_to_str(status), decided_at, id],
    )?;
    Ok(())
}

/// Updates the AI analysis for a screener entry.
pub fn set_entry_analysis(conn: &Connection, id: &str, analysis: &SenderAnalysis) -> Result<()> {
    let json = serde_json::to_string(analysis).unwrap_or_default();
    conn.execute(
        "UPDATE screener_entries SET ai_analysis = ?1 WHERE id = ?2",
        params![json, id],
    )?;
    Ok(())
}

/// Deletes a screener entry.
pub fn delete_entry(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM screener_entries WHERE id = ?1", params![id])?;
    Ok(())
}

/// Counts pending screener entries.
pub fn count_pending(conn: &Connection) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM screener_entries WHERE status = 'pending'",
        [],
        |row| row.get(0),
    )
}

/// Counts entries by status.
pub fn count_by_status(conn: &Connection, status: &ScreenerStatus) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM screener_entries WHERE status = ?1",
        params![status_to_str(status)],
        |row| row.get(0),
    )
}

// --- Rule operations ---

/// Inserts a new screener rule.
pub fn insert_rule(conn: &Connection, rule: &ScreenerRule) -> Result<()> {
    conn.execute(
        "INSERT INTO screener_rules (id, rule_type, pattern, action, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            rule.id,
            rule_type_to_str(&rule.rule_type),
            rule.pattern,
            action_to_str(&rule.action),
            rule.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Gets a rule by ID.
pub fn get_rule_by_id(conn: &Connection, id: &str) -> Result<Option<ScreenerRule>> {
    conn.query_row(
        "SELECT id, rule_type, pattern, action, created_at
         FROM screener_rules WHERE id = ?1",
        params![id],
        row_to_rule,
    )
    .optional()
}

/// Gets all screener rules.
pub fn get_all_rules(conn: &Connection) -> Result<Vec<ScreenerRule>> {
    let mut stmt = conn.prepare(
        "SELECT id, rule_type, pattern, action, created_at
         FROM screener_rules ORDER BY created_at DESC",
    )?;

    let rules = stmt.query_map([], row_to_rule)?;
    rules.collect()
}

/// Gets rules by type.
pub fn get_rules_by_type(conn: &Connection, rule_type: &RuleType) -> Result<Vec<ScreenerRule>> {
    let mut stmt = conn.prepare(
        "SELECT id, rule_type, pattern, action, created_at
         FROM screener_rules WHERE rule_type = ?1 ORDER BY created_at DESC",
    )?;

    let rules = stmt.query_map(params![rule_type_to_str(rule_type)], row_to_rule)?;
    rules.collect()
}

/// Finds a matching rule for an email address.
pub fn find_matching_rule(conn: &Connection, email: &str) -> Result<Option<ScreenerRule>> {
    // Extract domain from email
    let domain = email.split('@').nth(1).unwrap_or("");

    // Check domain rules first
    if let Some(rule) = conn
        .query_row(
            "SELECT id, rule_type, pattern, action, created_at
         FROM screener_rules WHERE rule_type IN ('domain_allow', 'domain_block') AND pattern = ?1",
            params![domain],
            row_to_rule,
        )
        .optional()?
    {
        return Ok(Some(rule));
    }

    // Check pattern rules
    let mut stmt = conn.prepare(
        "SELECT id, rule_type, pattern, action, created_at
         FROM screener_rules WHERE rule_type = 'pattern'",
    )?;

    let rules: Vec<ScreenerRule> = stmt
        .query_map([], row_to_rule)?
        .filter_map(|r| r.ok())
        .collect();

    for rule in rules {
        if email.contains(&rule.pattern) || domain.contains(&rule.pattern) {
            return Ok(Some(rule));
        }
    }

    Ok(None)
}

/// Deletes a rule.
pub fn delete_rule(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM screener_rules WHERE id = ?1", params![id])?;
    Ok(())
}

/// Counts total rules.
pub fn count_rules(conn: &Connection) -> Result<u32> {
    conn.query_row("SELECT COUNT(*) FROM screener_rules", [], |row| row.get(0))
}

// --- Helper functions ---

fn status_to_str(status: &ScreenerStatus) -> &'static str {
    match status {
        ScreenerStatus::Pending => "pending",
        ScreenerStatus::Approved => "approved",
        ScreenerStatus::Rejected => "rejected",
    }
}

fn str_to_status(s: &str) -> ScreenerStatus {
    match s {
        "approved" => ScreenerStatus::Approved,
        "rejected" => ScreenerStatus::Rejected,
        _ => ScreenerStatus::Pending,
    }
}

fn rule_type_to_str(rt: &RuleType) -> &'static str {
    match rt {
        RuleType::DomainAllow => "domain_allow",
        RuleType::DomainBlock => "domain_block",
        RuleType::Pattern => "pattern",
    }
}

fn str_to_rule_type(s: &str) -> RuleType {
    match s {
        "domain_allow" => RuleType::DomainAllow,
        "domain_block" => RuleType::DomainBlock,
        _ => RuleType::Pattern,
    }
}

fn action_to_str(action: &ScreenerAction) -> &'static str {
    match action {
        ScreenerAction::Approve => "approve",
        ScreenerAction::Reject => "reject",
        ScreenerAction::Review => "review",
    }
}

fn str_to_action(s: &str) -> ScreenerAction {
    match s {
        "approve" => ScreenerAction::Approve,
        "reject" => ScreenerAction::Reject,
        _ => ScreenerAction::Review,
    }
}

fn row_to_entry(row: &rusqlite::Row) -> Result<ScreenerEntry> {
    let ai_analysis_json: Option<String> = row.get(5)?;
    let decided_at_str: Option<String> = row.get(6)?;
    let created_at_str: String = row.get(7)?;
    let first_email_id: Option<String> = row.get(3)?;

    Ok(ScreenerEntry {
        id: row.get(0)?,
        sender_email: row.get(1)?,
        sender_name: row.get(2)?,
        first_email_id: first_email_id.map(EmailId::from),
        status: str_to_status(row.get::<_, String>(4)?.as_str()),
        ai_analysis: ai_analysis_json.and_then(|j| serde_json::from_str(&j).ok()),
        decided_at: decided_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        }),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

fn row_to_rule(row: &rusqlite::Row) -> Result<ScreenerRule> {
    let created_at_str: String = row.get(4)?;

    Ok(ScreenerRule {
        id: row.get(0)?,
        rule_type: str_to_rule_type(row.get::<_, String>(1)?.as_str()),
        pattern: row.get(2)?,
        action: str_to_action(row.get::<_, String>(3)?.as_str()),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        for migration in super::super::super::schema::all_migrations() {
            conn.execute_batch(migration).unwrap();
        }
        conn
    }

    fn make_entry(id: &str, sender_email: &str) -> ScreenerEntry {
        ScreenerEntry {
            id: id.to_string(),
            sender_email: sender_email.to_string(),
            sender_name: None,
            first_email_id: None,
            status: ScreenerStatus::Pending,
            ai_analysis: None,
            decided_at: None,
            created_at: Utc::now(),
        }
    }

    fn make_rule(
        id: &str,
        rule_type: RuleType,
        pattern: &str,
        action: ScreenerAction,
    ) -> ScreenerRule {
        ScreenerRule {
            id: id.to_string(),
            rule_type,
            pattern: pattern.to_string(),
            action,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn insert_and_get_entry() {
        let conn = setup();
        let entry = make_entry("e1", "unknown@example.com");

        insert_entry(&conn, &entry).unwrap();
        let fetched = get_entry_by_id(&conn, "e1").unwrap().unwrap();

        assert_eq!(fetched.sender_email, "unknown@example.com");
        assert_eq!(fetched.status, ScreenerStatus::Pending);
    }

    #[test]
    fn lookup_entry_by_sender() {
        let conn = setup();
        let entry = make_entry("e1", "test@example.com");
        insert_entry(&conn, &entry).unwrap();

        let fetched = super::get_entry_by_sender(&conn, "test@example.com")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, "e1");
    }

    #[test]
    fn update_entry_status() {
        let conn = setup();
        let entry = make_entry("e1", "test@example.com");
        insert_entry(&conn, &entry).unwrap();

        set_entry_status(&conn, "e1", &ScreenerStatus::Approved).unwrap();

        let fetched = get_entry_by_id(&conn, "e1").unwrap().unwrap();
        assert_eq!(fetched.status, ScreenerStatus::Approved);
        assert!(fetched.decided_at.is_some());
    }

    #[test]
    fn list_pending_entries() {
        let conn = setup();

        insert_entry(&conn, &make_entry("e1", "a@example.com")).unwrap();
        insert_entry(&conn, &make_entry("e2", "b@example.com")).unwrap();

        set_entry_status(&conn, "e1", &ScreenerStatus::Approved).unwrap();

        let pending = super::get_pending_entries(&conn).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "e2");
    }

    #[test]
    fn count_pending_entries() {
        let conn = setup();

        insert_entry(&conn, &make_entry("e1", "a@example.com")).unwrap();
        insert_entry(&conn, &make_entry("e2", "b@example.com")).unwrap();

        assert_eq!(super::count_pending(&conn).unwrap(), 2);

        set_entry_status(&conn, "e1", &ScreenerStatus::Rejected).unwrap();
        assert_eq!(super::count_pending(&conn).unwrap(), 1);
    }

    #[test]
    fn insert_and_get_rule() {
        let conn = setup();
        let rule = make_rule(
            "r1",
            RuleType::DomainAllow,
            "trusted.com",
            ScreenerAction::Approve,
        );

        insert_rule(&conn, &rule).unwrap();
        let fetched = get_rule_by_id(&conn, "r1").unwrap().unwrap();

        assert_eq!(fetched.pattern, "trusted.com");
        assert_eq!(fetched.rule_type, RuleType::DomainAllow);
    }

    #[test]
    fn find_matching_domain_rule() {
        let conn = setup();

        let rule = make_rule(
            "r1",
            RuleType::DomainAllow,
            "trusted.com",
            ScreenerAction::Approve,
        );
        insert_rule(&conn, &rule).unwrap();

        let matched = find_matching_rule(&conn, "user@trusted.com").unwrap();
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().pattern, "trusted.com");

        let no_match = find_matching_rule(&conn, "user@other.com").unwrap();
        assert!(no_match.is_none());
    }

    #[test]
    fn filter_rules_by_type() {
        let conn = setup();

        insert_rule(
            &conn,
            &make_rule(
                "r1",
                RuleType::DomainAllow,
                "a.com",
                ScreenerAction::Approve,
            ),
        )
        .unwrap();
        insert_rule(
            &conn,
            &make_rule("r2", RuleType::DomainBlock, "b.com", ScreenerAction::Reject),
        )
        .unwrap();
        insert_rule(
            &conn,
            &make_rule(
                "r3",
                RuleType::DomainAllow,
                "c.com",
                ScreenerAction::Approve,
            ),
        )
        .unwrap();

        let allow_rules = super::get_rules_by_type(&conn, &RuleType::DomainAllow).unwrap();
        assert_eq!(allow_rules.len(), 2);

        let block_rules = super::get_rules_by_type(&conn, &RuleType::DomainBlock).unwrap();
        assert_eq!(block_rules.len(), 1);
    }

    #[test]
    fn remove_rule() {
        let conn = setup();
        let rule = make_rule(
            "r1",
            RuleType::DomainAllow,
            "test.com",
            ScreenerAction::Approve,
        );
        insert_rule(&conn, &rule).unwrap();

        super::delete_rule(&conn, "r1").unwrap();
        let fetched = get_rule_by_id(&conn, "r1").unwrap();
        assert!(fetched.is_none());
    }
}
