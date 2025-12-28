use rusqlite::Connection;

#[test]
fn migrate_backfills_codex_home() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("backfill.sqlite");
    let codex_home = "/tmp/codex-home";
    {
        let conn = Connection::open(&db_path).expect("open conn");
        let migration = include_str!("../migrations/0001_init.sql");
        conn.execute_batch(migration).expect("migrate 0001");
        conn.execute(
            "INSERT INTO app_setting (key, value) VALUES ('codex_home', ?1)",
            [codex_home],
        )
        .expect("insert app setting");
        conn.execute(
            r#"
            INSERT INTO usage_event (
              id, ts, model, input_tokens, cached_input_tokens, output_tokens,
              reasoning_output_tokens, total_tokens, context_used, context_window,
              cost_usd, source, request_id, raw_json
            ) VALUES (
              'e1', '2025-12-19T19:00:00Z', 'gpt-5.2', 10, 0, 2, 0, 12, 12, 100, NULL, 'source-a', NULL, NULL
            )
            "#,
            [],
        )
        .expect("insert usage event");
        conn.execute(
            r#"
            INSERT INTO ingest_cursor (
              codex_home, file_path, inode, mtime, byte_offset, last_event_key, updated_at
            ) VALUES (
              ?1, 'log.ndjson', NULL, NULL, 123, 'e1', '2025-12-19T19:10:00Z'
            )
            "#,
            [codex_home],
        )
        .expect("insert cursor");
    }

    let mut db = tracker_db::Db::open(&db_path).expect("open db");
    db.migrate().expect("migrate db");

    let conn = Connection::open(&db_path).expect("open conn");
    let home_id: i64 = conn
        .query_row("SELECT id FROM codex_home LIMIT 1", [], |row| row.get(0))
        .expect("load home id");
    let stored_path: String = conn
        .query_row("SELECT path FROM codex_home LIMIT 1", [], |row| row.get(0))
        .expect("load home path");
    assert_eq!(stored_path, codex_home);

    let active_id: String = conn
        .query_row(
            "SELECT value FROM app_setting WHERE key = 'active_codex_home_id'",
            [],
            |row| row.get(0),
        )
        .expect("active home");
    assert_eq!(active_id, home_id.to_string());

    let event_home_id: i64 = conn
        .query_row(
            "SELECT codex_home_id FROM usage_event WHERE id = 'e1'",
            [],
            |row| row.get(0),
        )
        .expect("usage home");
    assert_eq!(event_home_id, home_id);

    let cursor_home_id: i64 = conn
        .query_row(
            "SELECT codex_home_id FROM ingest_cursor WHERE file_path = 'log.ndjson'",
            [],
            |row| row.get(0),
        )
        .expect("cursor home");
    assert_eq!(cursor_home_id, home_id);

    let session_id: String = conn
        .query_row(
            "SELECT session_id FROM usage_event WHERE id = 'e1'",
            [],
            |row| row.get(0),
        )
        .expect("session id");
    assert_eq!(session_id, "source-a");
}
