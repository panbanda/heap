//! SQL schema definitions as const strings.
//!
//! Contains the complete SQLite schema for the margin email client.

/// SQL to create the accounts table.
pub const CREATE_ACCOUNTS: &str = r#"
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    display_name TEXT,
    provider_type TEXT NOT NULL,
    provider_config TEXT NOT NULL,
    sync_enabled INTEGER DEFAULT 1,
    sync_interval_seconds INTEGER DEFAULT 300,
    signature TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// SQL to create the emails table.
pub const CREATE_EMAILS: &str = r#"
CREATE TABLE IF NOT EXISTS emails (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    thread_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    in_reply_to TEXT,
    references_json TEXT,
    from_address TEXT NOT NULL,
    from_name TEXT,
    to_addresses TEXT NOT NULL,
    cc_addresses TEXT,
    bcc_addresses TEXT,
    subject TEXT,
    body_text TEXT,
    body_html TEXT,
    snippet TEXT,
    date TEXT NOT NULL,
    is_read INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0,
    is_draft INTEGER DEFAULT 0,
    labels TEXT,
    raw_headers TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// SQL to create email indexes.
pub const CREATE_EMAIL_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_emails_account ON emails(account_id);
CREATE INDEX IF NOT EXISTS idx_emails_thread ON emails(thread_id);
CREATE INDEX IF NOT EXISTS idx_emails_date ON emails(date DESC);
CREATE INDEX IF NOT EXISTS idx_emails_from ON emails(from_address)
"#;

/// SQL to create the threads table.
pub const CREATE_THREADS: &str = r#"
CREATE TABLE IF NOT EXISTS threads (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    subject TEXT,
    snippet TEXT,
    participant_emails TEXT NOT NULL,
    participant_names TEXT,
    last_message_date TEXT NOT NULL,
    message_count INTEGER DEFAULT 1,
    unread_count INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0,
    labels TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// SQL to create thread indexes.
pub const CREATE_THREAD_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_threads_account ON threads(account_id);
CREATE INDEX IF NOT EXISTS idx_threads_date ON threads(last_message_date DESC)
"#;

/// SQL to create the labels table.
pub const CREATE_LABELS: &str = r#"
CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    name TEXT NOT NULL,
    color TEXT,
    is_system INTEGER DEFAULT 0,
    provider_id TEXT,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the attachments table.
pub const CREATE_ATTACHMENTS: &str = r#"
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    email_id TEXT NOT NULL REFERENCES emails(id),
    filename TEXT NOT NULL,
    content_type TEXT,
    size_bytes INTEGER,
    content_id TEXT,
    is_inline INTEGER DEFAULT 0,
    local_path TEXT,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the drafts table.
pub const CREATE_DRAFTS: &str = r#"
CREATE TABLE IF NOT EXISTS drafts (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    reply_to_thread_id TEXT,
    reply_to_message_id TEXT,
    to_addresses TEXT,
    cc_addresses TEXT,
    bcc_addresses TEXT,
    subject TEXT,
    body_markdown TEXT,
    body_html TEXT,
    attachments TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// SQL to create the contacts table.
pub const CREATE_CONTACTS: &str = r#"
CREATE TABLE IF NOT EXISTS contacts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    frequency INTEGER DEFAULT 1,
    last_contacted TEXT,
    is_vip INTEGER DEFAULT 0,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// SQL to create the contacts index.
pub const CREATE_CONTACTS_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_contacts_email ON contacts(email)
"#;

/// SQL to create the screener_entries table.
pub const CREATE_SCREENER_ENTRIES: &str = r#"
CREATE TABLE IF NOT EXISTS screener_entries (
    id TEXT PRIMARY KEY,
    sender_email TEXT NOT NULL,
    sender_name TEXT,
    first_email_id TEXT REFERENCES emails(id),
    status TEXT NOT NULL,
    ai_analysis TEXT,
    decided_at TEXT,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the screener_rules table.
pub const CREATE_SCREENER_RULES: &str = r#"
CREATE TABLE IF NOT EXISTS screener_rules (
    id TEXT PRIMARY KEY,
    rule_type TEXT NOT NULL,
    pattern TEXT NOT NULL,
    action TEXT NOT NULL,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the snoozed table.
pub const CREATE_SNOOZED: &str = r#"
CREATE TABLE IF NOT EXISTS snoozed (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES threads(id),
    snooze_until TEXT NOT NULL,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the snoozed index.
pub const CREATE_SNOOZED_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_snoozed_until ON snoozed(snooze_until)
"#;

/// SQL to create the sync_state table.
pub const CREATE_SYNC_STATE: &str = r#"
CREATE TABLE IF NOT EXISTS sync_state (
    account_id TEXT PRIMARY KEY REFERENCES accounts(id),
    last_sync TEXT,
    last_history_id TEXT,
    last_uid_validity INTEGER,
    last_uid INTEGER
)
"#;

/// SQL to create the pending_changes table.
pub const CREATE_PENDING_CHANGES: &str = r#"
CREATE TABLE IF NOT EXISTS pending_changes (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    change_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the embeddings table.
pub const CREATE_EMBEDDINGS: &str = r#"
CREATE TABLE IF NOT EXISTS embeddings (
    email_id TEXT PRIMARY KEY REFERENCES emails(id),
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL
)
"#;

/// SQL to create the telemetry_events table.
pub const CREATE_TELEMETRY_EVENTS: &str = r#"
CREATE TABLE IF NOT EXISTS telemetry_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    payload TEXT,
    timestamp TEXT NOT NULL
)
"#;

/// SQL to create the telemetry index.
pub const CREATE_TELEMETRY_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_telemetry_timestamp ON telemetry_events(timestamp)
"#;

/// SQL to create the daily_stats table.
pub const CREATE_DAILY_STATS: &str = r#"
CREATE TABLE IF NOT EXISTS daily_stats (
    date TEXT NOT NULL,
    account_id TEXT,
    emails_received INTEGER DEFAULT 0,
    emails_sent INTEGER DEFAULT 0,
    emails_archived INTEGER DEFAULT 0,
    emails_trashed INTEGER DEFAULT 0,
    time_in_app_seconds INTEGER DEFAULT 0,
    sessions INTEGER DEFAULT 0,
    ai_summaries INTEGER DEFAULT 0,
    ai_drafts INTEGER DEFAULT 0,
    ai_searches INTEGER DEFAULT 0,
    ai_tokens_used INTEGER DEFAULT 0,
    PRIMARY KEY (date, account_id)
)
"#;

/// SQL to create the settings table.
pub const CREATE_SETTINGS: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// SQL to create the FTS5 virtual table for email search.
pub const CREATE_EMAILS_FTS: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS emails_fts USING fts5(
    subject,
    body_text,
    from_address,
    from_name,
    to_addresses,
    content='emails',
    content_rowid='rowid'
)
"#;

/// SQL to create triggers to keep FTS in sync with emails table.
pub const CREATE_EMAILS_FTS_TRIGGERS: &str = r#"
CREATE TRIGGER IF NOT EXISTS emails_ai AFTER INSERT ON emails BEGIN
    INSERT INTO emails_fts(rowid, subject, body_text, from_address, from_name, to_addresses)
    VALUES (NEW.rowid, NEW.subject, NEW.body_text, NEW.from_address, NEW.from_name, NEW.to_addresses);
END;

CREATE TRIGGER IF NOT EXISTS emails_ad AFTER DELETE ON emails BEGIN
    INSERT INTO emails_fts(emails_fts, rowid, subject, body_text, from_address, from_name, to_addresses)
    VALUES ('delete', OLD.rowid, OLD.subject, OLD.body_text, OLD.from_address, OLD.from_name, OLD.to_addresses);
END;

CREATE TRIGGER IF NOT EXISTS emails_au AFTER UPDATE ON emails BEGIN
    INSERT INTO emails_fts(emails_fts, rowid, subject, body_text, from_address, from_name, to_addresses)
    VALUES ('delete', OLD.rowid, OLD.subject, OLD.body_text, OLD.from_address, OLD.from_name, OLD.to_addresses);
    INSERT INTO emails_fts(rowid, subject, body_text, from_address, from_name, to_addresses)
    VALUES (NEW.rowid, NEW.subject, NEW.body_text, NEW.from_address, NEW.from_name, NEW.to_addresses);
END
"#;

/// Returns all schema creation statements in order.
pub fn all_migrations() -> Vec<&'static str> {
    vec![
        CREATE_ACCOUNTS,
        CREATE_EMAILS,
        CREATE_EMAIL_INDEXES,
        CREATE_THREADS,
        CREATE_THREAD_INDEXES,
        CREATE_LABELS,
        CREATE_ATTACHMENTS,
        CREATE_DRAFTS,
        CREATE_CONTACTS,
        CREATE_CONTACTS_INDEX,
        CREATE_SCREENER_ENTRIES,
        CREATE_SCREENER_RULES,
        CREATE_SNOOZED,
        CREATE_SNOOZED_INDEX,
        CREATE_SYNC_STATE,
        CREATE_PENDING_CHANGES,
        CREATE_EMBEDDINGS,
        CREATE_TELEMETRY_EVENTS,
        CREATE_TELEMETRY_INDEX,
        CREATE_DAILY_STATS,
        CREATE_SETTINGS,
        CREATE_EMAILS_FTS,
        CREATE_EMAILS_FTS_TRIGGERS,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_migrations_returns_statements() {
        let migrations = all_migrations();
        assert!(!migrations.is_empty());
        assert!(migrations.len() >= 20);
    }

    #[test]
    fn create_accounts_is_valid_sql() {
        assert!(CREATE_ACCOUNTS.contains("CREATE TABLE"));
        assert!(CREATE_ACCOUNTS.contains("accounts"));
        assert!(CREATE_ACCOUNTS.contains("id TEXT PRIMARY KEY"));
    }

    #[test]
    fn create_emails_has_foreign_key() {
        assert!(CREATE_EMAILS.contains("REFERENCES accounts(id)"));
    }

    #[test]
    fn indexes_use_if_not_exists() {
        assert!(CREATE_EMAIL_INDEXES.contains("IF NOT EXISTS"));
        assert!(CREATE_THREAD_INDEXES.contains("IF NOT EXISTS"));
    }
}
