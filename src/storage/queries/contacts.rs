//! Contact database queries.
//!
//! CRUD operations for contacts extracted from email interactions.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::domain::Contact;

/// Inserts or updates a contact.
pub fn upsert(conn: &Connection, contact: &Contact) -> Result<()> {
    conn.execute(
        "INSERT INTO contacts (id, email, name, frequency, last_contacted, is_vip, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))
         ON CONFLICT(email) DO UPDATE SET
             name = COALESCE(?3, name),
             frequency = frequency + 1,
             last_contacted = ?5,
             updated_at = datetime('now')",
        params![
            contact.id,
            contact.email,
            contact.name,
            contact.frequency,
            contact.last_contacted.map(|dt| dt.to_rfc3339()),
            contact.is_vip,
            contact.notes,
        ],
    )?;
    Ok(())
}

/// Gets a contact by ID.
pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Contact>> {
    conn.query_row(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts WHERE id = ?1",
        params![id],
        row_to_contact,
    )
    .optional()
}

/// Gets a contact by email.
pub fn get_by_email(conn: &Connection, email: &str) -> Result<Option<Contact>> {
    conn.query_row(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts WHERE email = ?1",
        params![email],
        row_to_contact,
    )
    .optional()
}

/// Gets all contacts ordered by frequency.
pub fn get_all_by_frequency(conn: &Connection) -> Result<Vec<Contact>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts ORDER BY frequency DESC",
    )?;

    let contacts = stmt.query_map([], row_to_contact)?;
    contacts.collect()
}

/// Gets all contacts ordered by name.
pub fn get_all_by_name(conn: &Connection) -> Result<Vec<Contact>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts ORDER BY COALESCE(name, email)",
    )?;

    let contacts = stmt.query_map([], row_to_contact)?;
    contacts.collect()
}

/// Gets VIP contacts.
pub fn get_vip(conn: &Connection) -> Result<Vec<Contact>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts WHERE is_vip = 1 ORDER BY COALESCE(name, email)",
    )?;

    let contacts = stmt.query_map([], row_to_contact)?;
    contacts.collect()
}

/// Gets recently contacted contacts.
pub fn get_recent(conn: &Connection, limit: u32) -> Result<Vec<Contact>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts WHERE last_contacted IS NOT NULL
         ORDER BY last_contacted DESC LIMIT ?1",
    )?;

    let contacts = stmt.query_map(params![limit], row_to_contact)?;
    contacts.collect()
}

/// Gets frequently contacted contacts.
pub fn get_frequent(conn: &Connection, limit: u32) -> Result<Vec<Contact>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts ORDER BY frequency DESC LIMIT ?1",
    )?;

    let contacts = stmt.query_map(params![limit], row_to_contact)?;
    contacts.collect()
}

/// Searches contacts by email or name.
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<Contact>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, email, name, frequency, last_contacted, is_vip, notes
         FROM contacts
         WHERE email LIKE ?1 OR name LIKE ?1
         ORDER BY frequency DESC LIMIT ?2",
    )?;

    let contacts = stmt.query_map(params![pattern, limit], row_to_contact)?;
    contacts.collect()
}

/// Updates a contact's name.
pub fn set_name(conn: &Connection, id: &str, name: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![name, id],
    )?;
    Ok(())
}

/// Sets a contact as VIP.
pub fn set_vip(conn: &Connection, id: &str, is_vip: bool) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET is_vip = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![is_vip, id],
    )?;
    Ok(())
}

/// Updates a contact's notes.
pub fn set_notes(conn: &Connection, id: &str, notes: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET notes = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![notes, id],
    )?;
    Ok(())
}

/// Increments the contact frequency and updates last contacted.
pub fn record_interaction(conn: &Connection, email: &str) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET frequency = frequency + 1, last_contacted = datetime('now'), updated_at = datetime('now')
         WHERE email = ?1",
        params![email],
    )?;
    Ok(())
}

/// Deletes a contact.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM contacts WHERE id = ?1", params![id])?;
    Ok(())
}

/// Deletes a contact by email.
pub fn delete_by_email(conn: &Connection, email: &str) -> Result<()> {
    conn.execute("DELETE FROM contacts WHERE email = ?1", params![email])?;
    Ok(())
}

/// Counts total contacts.
pub fn count(conn: &Connection) -> Result<u32> {
    conn.query_row("SELECT COUNT(*) FROM contacts", [], |row| row.get(0))
}

/// Counts VIP contacts.
pub fn count_vip(conn: &Connection) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM contacts WHERE is_vip = 1",
        [],
        |row| row.get(0),
    )
}

/// Checks if a contact exists by email.
pub fn exists_by_email(conn: &Connection, email: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM contacts WHERE email = ?1)",
        params![email],
        |row| row.get(0),
    )
}

fn row_to_contact(row: &rusqlite::Row) -> Result<Contact> {
    let last_contacted: Option<String> = row.get(4)?;
    Ok(Contact {
        id: row.get(0)?,
        email: row.get(1)?,
        name: row.get(2)?,
        frequency: row.get(3)?,
        last_contacted: last_contacted.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        }),
        is_vip: row.get(5)?,
        notes: row.get(6)?,
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

    fn make_contact(id: &str, email: &str) -> Contact {
        Contact {
            id: id.to_string(),
            email: email.to_string(),
            name: None,
            frequency: 1,
            last_contacted: None,
            is_vip: false,
            notes: None,
        }
    }

    #[test]
    fn upsert_and_get() {
        let conn = setup();
        let contact = make_contact("c1", "test@example.com");

        upsert(&conn, &contact).unwrap();
        let fetched = get_by_email(&conn, "test@example.com").unwrap().unwrap();

        assert_eq!(fetched.email, "test@example.com");
    }

    #[test]
    fn upsert_increments_frequency() {
        let conn = setup();
        let contact = make_contact("c1", "test@example.com");

        upsert(&conn, &contact).unwrap();
        upsert(&conn, &contact).unwrap();
        upsert(&conn, &contact).unwrap();

        let fetched = get_by_email(&conn, "test@example.com").unwrap().unwrap();
        assert_eq!(fetched.frequency, 3);
    }

    #[test]
    fn get_by_id_works() {
        let conn = setup();
        let contact = make_contact("contact-123", "test@example.com");

        upsert(&conn, &contact).unwrap();
        let fetched = get_by_id(&conn, "contact-123").unwrap().unwrap();

        assert_eq!(fetched.email, "test@example.com");
    }

    #[test]
    fn vip_operations() {
        let conn = setup();
        let contact = make_contact("c1", "vip@example.com");
        upsert(&conn, &contact).unwrap();

        // Not VIP initially
        let fetched = get_by_id(&conn, "c1").unwrap().unwrap();
        assert!(!fetched.is_vip);

        // Set as VIP
        set_vip(&conn, "c1", true).unwrap();
        let fetched = get_by_id(&conn, "c1").unwrap().unwrap();
        assert!(fetched.is_vip);

        // Get VIP list
        let vips = get_vip(&conn).unwrap();
        assert_eq!(vips.len(), 1);
    }

    #[test]
    fn search_contacts() {
        let conn = setup();

        let mut c1 = make_contact("c1", "alice@example.com");
        c1.name = Some("Alice Smith".to_string());
        upsert(&conn, &c1).unwrap();

        let mut c2 = make_contact("c2", "bob@example.com");
        c2.name = Some("Bob Jones".to_string());
        upsert(&conn, &c2).unwrap();

        let results = search(&conn, "alice", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].email, "alice@example.com");

        let results = search(&conn, "example", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn update_name() {
        let conn = setup();
        let contact = make_contact("c1", "test@example.com");
        upsert(&conn, &contact).unwrap();

        set_name(&conn, "c1", Some("Test User")).unwrap();
        let fetched = get_by_id(&conn, "c1").unwrap().unwrap();
        assert_eq!(fetched.name, Some("Test User".to_string()));
    }

    #[test]
    fn delete_contact() {
        let conn = setup();
        let contact = make_contact("c1", "test@example.com");
        upsert(&conn, &contact).unwrap();

        delete(&conn, "c1").unwrap();
        let fetched = get_by_id(&conn, "c1").unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn count_contacts() {
        let conn = setup();

        upsert(&conn, &make_contact("c1", "a@example.com")).unwrap();
        upsert(&conn, &make_contact("c2", "b@example.com")).unwrap();

        let total = count(&conn).unwrap();
        assert_eq!(total, 2);
    }

    #[test]
    fn frequent_contacts() {
        let conn = setup();

        let mut c1 = make_contact("c1", "frequent@example.com");
        c1.frequency = 100;
        upsert(&conn, &c1).unwrap();

        let c2 = make_contact("c2", "rare@example.com");
        upsert(&conn, &c2).unwrap();

        let frequent = get_frequent(&conn, 1).unwrap();
        assert_eq!(frequent.len(), 1);
        assert_eq!(frequent[0].email, "frequent@example.com");
    }
}
