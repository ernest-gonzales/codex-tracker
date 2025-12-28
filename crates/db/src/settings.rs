use rusqlite::params;

use crate::Db;
use crate::error::Result;

impl Db {
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_setting WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get::<_, String>(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO app_setting (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_context_active_minutes(&self) -> Result<u32> {
        let minutes = self
            .get_setting("context_active_minutes")?
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(60);
        Ok(minutes)
    }

    pub fn set_context_active_minutes(&self, minutes: u32) -> Result<()> {
        self.set_setting("context_active_minutes", &minutes.to_string())
    }
}
