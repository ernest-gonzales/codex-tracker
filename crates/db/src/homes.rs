use std::env;
use std::path::PathBuf;

use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use tracker_core::CodexHome;

use crate::Db;
use crate::error::Result;
use crate::helpers::row_to_codex_home;

impl Db {
    pub fn list_homes(&self) -> Result<Vec<CodexHome>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, label, path, created_at, last_seen_at
            FROM codex_home
            ORDER BY created_at ASC, id ASC
            "#,
        )?;
        let rows = stmt
            .query_map([], row_to_codex_home)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_home_by_id(&self, id: i64) -> Result<Option<CodexHome>> {
        self.conn
            .query_row(
                r#"
                SELECT id, label, path, created_at, last_seen_at
                FROM codex_home
                WHERE id = ?1
                "#,
                params![id],
                row_to_codex_home,
            )
            .optional()
            .map_err(crate::error::DbError::from)
    }

    pub fn get_home_by_path(&self, path: &str) -> Result<Option<CodexHome>> {
        self.conn
            .query_row(
                r#"
                SELECT id, label, path, created_at, last_seen_at
                FROM codex_home
                WHERE path = ?1
                "#,
                params![path],
                row_to_codex_home,
            )
            .optional()
            .map_err(crate::error::DbError::from)
    }

    pub fn add_home(&self, path: &str, label: Option<&str>) -> Result<CodexHome> {
        let now = Utc::now().to_rfc3339();
        let label = label.unwrap_or("Home");
        self.conn.execute(
            r#"
            INSERT INTO codex_home (label, path, created_at, last_seen_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![label, path, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_home_by_id(id)?
            .ok_or_else(|| crate::error::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn get_or_create_home(&self, path: &str, label: Option<&str>) -> Result<CodexHome> {
        if let Some(home) = self.get_home_by_path(path)? {
            return Ok(home);
        }
        let inserted = self.add_home(path, label);
        if let Ok(home) = inserted {
            return Ok(home);
        }
        if let Some(home) = self.get_home_by_path(path)? {
            return Ok(home);
        }
        Err(inserted.err().unwrap_or(crate::error::DbError::Sqlite(
            rusqlite::Error::QueryReturnedNoRows,
        )))
    }

    pub fn set_active_home(&self, home_id: i64) -> Result<()> {
        self.set_setting("active_codex_home_id", &home_id.to_string())
    }

    pub fn get_active_home(&self) -> Result<Option<CodexHome>> {
        let value = self.get_setting("active_codex_home_id")?;
        let Some(value) = value else {
            return Ok(None);
        };
        let id = value.parse::<i64>().ok();
        let Some(id) = id else {
            return Ok(None);
        };
        self.get_home_by_id(id)
    }

    pub fn ensure_active_home(&mut self) -> Result<CodexHome> {
        if let Some(home) = self.get_active_home()? {
            return Ok(home);
        }
        let path = load_codex_home_path(&self.conn)?;
        let home = self.get_or_create_home(&path, Some("Default"))?;
        self.set_active_home(home.id)?;
        Ok(home)
    }

    pub fn update_home_last_seen(&self, home_id: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE codex_home SET last_seen_at = ?1 WHERE id = ?2",
            params![now, home_id],
        )?;
        Ok(())
    }

    pub fn delete_home(&mut self, home_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM usage_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM message_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM usage_limit_snapshot WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM ingest_cursor WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute("DELETE FROM codex_home WHERE id = ?1", params![home_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn clear_home_data(&mut self, home_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM usage_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM message_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM usage_limit_snapshot WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM ingest_cursor WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn count_usage_events(&self, home_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM usage_event WHERE codex_home_id = ?1",
                params![home_id],
                |row| row.get(0),
            )
            .map_err(crate::error::DbError::from)
    }

    pub fn count_message_events(&self, home_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM message_event WHERE codex_home_id = ?1",
                params![home_id],
                |row| row.get(0),
            )
            .map_err(crate::error::DbError::from)
    }

    pub fn count_ingest_cursors(&self, home_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM ingest_cursor WHERE codex_home_id = ?1",
                params![home_id],
                |row| row.get(0),
            )
            .map_err(crate::error::DbError::from)
    }
}

pub(crate) fn load_codex_home_path(conn: &rusqlite::Connection) -> Result<String> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM app_setting WHERE key = 'codex_home'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(path) = stored {
        return Ok(path);
    }
    Ok(default_codex_home_path())
}

fn default_codex_home_path() -> String {
    if let Ok(path) = env::var("CODEX_HOME") {
        return path;
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".codex")
            .to_string_lossy()
            .to_string();
    }
    ".codex".to_string()
}
