//! Label database queries.
//!
//! CRUD operations for email labels and folders.

use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::domain::{AccountId, Label, LabelId};

/// Inserts a new label.
pub fn insert(conn: &Connection, label: &Label) -> Result<()> {
    conn.execute(
        "INSERT INTO labels (id, account_id, name, color, is_system, provider_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        params![
            label.id.0.as_str(),
            label.account_id.0.as_str(),
            label.name,
            label.color,
            label.is_system,
            label.provider_id,
        ],
    )?;
    Ok(())
}

/// Gets a label by ID.
pub fn get_by_id(conn: &Connection, id: &LabelId) -> Result<Option<Label>> {
    conn.query_row(
        "SELECT id, account_id, name, color, is_system, provider_id
         FROM labels WHERE id = ?1",
        params![id.0.as_str()],
        |row| {
            Ok(Label {
                id: LabelId::from(row.get::<_, String>(0)?),
                account_id: AccountId::from(row.get::<_, String>(1)?),
                name: row.get(2)?,
                color: row.get(3)?,
                is_system: row.get(4)?,
                provider_id: row.get(5)?,
            })
        },
    )
    .optional()
}

/// Gets all labels for an account.
pub fn get_by_account(conn: &Connection, account_id: &AccountId) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, color, is_system, provider_id
         FROM labels WHERE account_id = ?1 ORDER BY name",
    )?;

    let labels = stmt.query_map(params![account_id.0.as_str()], |row| {
        Ok(Label {
            id: LabelId::from(row.get::<_, String>(0)?),
            account_id: AccountId::from(row.get::<_, String>(1)?),
            name: row.get(2)?,
            color: row.get(3)?,
            is_system: row.get(4)?,
            provider_id: row.get(5)?,
        })
    })?;

    labels.collect()
}

/// Gets system labels for an account.
pub fn get_system_labels(conn: &Connection, account_id: &AccountId) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, color, is_system, provider_id
         FROM labels WHERE account_id = ?1 AND is_system = 1 ORDER BY name",
    )?;

    let labels = stmt.query_map(params![account_id.0.as_str()], |row| {
        Ok(Label {
            id: LabelId::from(row.get::<_, String>(0)?),
            account_id: AccountId::from(row.get::<_, String>(1)?),
            name: row.get(2)?,
            color: row.get(3)?,
            is_system: row.get(4)?,
            provider_id: row.get(5)?,
        })
    })?;

    labels.collect()
}

/// Gets user-created labels for an account.
pub fn get_user_labels(conn: &Connection, account_id: &AccountId) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, color, is_system, provider_id
         FROM labels WHERE account_id = ?1 AND is_system = 0 ORDER BY name",
    )?;

    let labels = stmt.query_map(params![account_id.0.as_str()], |row| {
        Ok(Label {
            id: LabelId::from(row.get::<_, String>(0)?),
            account_id: AccountId::from(row.get::<_, String>(1)?),
            name: row.get(2)?,
            color: row.get(3)?,
            is_system: row.get(4)?,
            provider_id: row.get(5)?,
        })
    })?;

    labels.collect()
}

/// Gets a label by provider ID.
pub fn get_by_provider_id(
    conn: &Connection,
    account_id: &AccountId,
    provider_id: &str,
) -> Result<Option<Label>> {
    conn.query_row(
        "SELECT id, account_id, name, color, is_system, provider_id
         FROM labels WHERE account_id = ?1 AND provider_id = ?2",
        params![account_id.0.as_str(), provider_id],
        |row| {
            Ok(Label {
                id: LabelId::from(row.get::<_, String>(0)?),
                account_id: AccountId::from(row.get::<_, String>(1)?),
                name: row.get(2)?,
                color: row.get(3)?,
                is_system: row.get(4)?,
                provider_id: row.get(5)?,
            })
        },
    )
    .optional()
}

/// Updates a label's name.
pub fn set_name(conn: &Connection, id: &LabelId, name: &str) -> Result<()> {
    conn.execute(
        "UPDATE labels SET name = ?1 WHERE id = ?2",
        params![name, id.0.as_str()],
    )?;
    Ok(())
}

/// Updates a label's color.
pub fn set_color(conn: &Connection, id: &LabelId, color: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE labels SET color = ?1 WHERE id = ?2",
        params![color, id.0.as_str()],
    )?;
    Ok(())
}

/// Deletes a label.
pub fn delete(conn: &Connection, id: &LabelId) -> Result<()> {
    conn.execute("DELETE FROM labels WHERE id = ?1", params![id.0.as_str()])?;
    Ok(())
}

/// Deletes all labels for an account.
pub fn delete_by_account(conn: &Connection, account_id: &AccountId) -> Result<()> {
    conn.execute(
        "DELETE FROM labels WHERE account_id = ?1",
        params![account_id.0.as_str()],
    )?;
    Ok(())
}

/// Counts labels for an account.
pub fn count_by_account(conn: &Connection, account_id: &AccountId) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM labels WHERE account_id = ?1",
        params![account_id.0.as_str()],
        |row| row.get(0),
    )
}

/// Checks if a label with the given name exists for an account.
pub fn exists_by_name(conn: &Connection, account_id: &AccountId, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM labels WHERE account_id = ?1 AND name = ?2)",
        params![account_id.0.as_str(), name],
        |row| row.get(0),
    )
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

    fn make_label(id: &str, account_id: &str, name: &str) -> Label {
        Label {
            id: LabelId::from(id),
            account_id: AccountId::from(account_id),
            name: name.to_string(),
            color: None,
            is_system: false,
            provider_id: None,
        }
    }

    #[test]
    fn insert_and_get() {
        let conn = setup();
        let label = make_label("label-1", "account-1", "Work");

        insert(&conn, &label).unwrap();
        let fetched = get_by_id(&conn, &label.id).unwrap().unwrap();

        assert_eq!(fetched.name, "Work");
        assert!(!fetched.is_system);
    }

    #[test]
    fn list_labels_by_account() {
        let conn = setup();

        insert(&conn, &make_label("l1", "acc-1", "Work")).unwrap();
        insert(&conn, &make_label("l2", "acc-1", "Personal")).unwrap();
        insert(&conn, &make_label("l3", "acc-2", "Other")).unwrap();

        let labels = super::get_by_account(&conn, &AccountId::from("acc-1")).unwrap();
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn system_vs_user_labels() {
        let conn = setup();

        let mut system = make_label("l1", "acc-1", "INBOX");
        system.is_system = true;
        insert(&conn, &system).unwrap();

        let user = make_label("l2", "acc-1", "Custom");
        insert(&conn, &user).unwrap();

        let system_labels = get_system_labels(&conn, &AccountId::from("acc-1")).unwrap();
        assert_eq!(system_labels.len(), 1);
        assert_eq!(system_labels[0].name, "INBOX");

        let user_labels = get_user_labels(&conn, &AccountId::from("acc-1")).unwrap();
        assert_eq!(user_labels.len(), 1);
        assert_eq!(user_labels[0].name, "Custom");
    }

    #[test]
    fn update_name_and_color() {
        let conn = setup();
        let label = make_label("label-1", "account-1", "Old Name");
        insert(&conn, &label).unwrap();

        set_name(&conn, &label.id, "New Name").unwrap();
        set_color(&conn, &label.id, Some("#ff0000")).unwrap();

        let fetched = get_by_id(&conn, &label.id).unwrap().unwrap();
        assert_eq!(fetched.name, "New Name");
        assert_eq!(fetched.color, Some("#ff0000".to_string()));
    }

    #[test]
    fn delete_label() {
        let conn = setup();
        let label = make_label("label-1", "account-1", "ToDelete");
        insert(&conn, &label).unwrap();

        delete(&conn, &label.id).unwrap();

        let fetched = get_by_id(&conn, &label.id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn count_labels() {
        let conn = setup();

        insert(&conn, &make_label("l1", "acc-1", "A")).unwrap();
        insert(&conn, &make_label("l2", "acc-1", "B")).unwrap();

        let count = count_by_account(&conn, &AccountId::from("acc-1")).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn exists_by_name_check() {
        let conn = setup();
        let label = make_label("l1", "acc-1", "Unique");
        insert(&conn, &label).unwrap();

        assert!(exists_by_name(&conn, &AccountId::from("acc-1"), "Unique").unwrap());
        assert!(!exists_by_name(&conn, &AccountId::from("acc-1"), "NonExistent").unwrap());
    }
}
