use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration as StdDuration, Instant};
use std::{env, fs};

use chrono::{DateTime, Utc};
use rayon::prelude::*;
use tracker_core::{MessageEvent, PricingRule, UsageEvent, UsageLimitSnapshot, UsageTotals};
use tracker_db::{Db, IngestCursor};
use walkdir::WalkDir;

use crate::parser::{
    compute_cost_for_event, delta_usage, extract_effort_if_turn_context,
    extract_limit_snapshots_from_value, extract_message_event_from_value, extract_model,
    extract_usage_event_from_value, parse_json_line,
};
use crate::types::{IngestIssue, IngestStats, Result};

fn is_log_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("log") | Some("jsonl") | Some("ndjson")
    )
}

fn is_plain_log(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("log")
    )
}

fn looks_like_jsonl(file: &mut File) -> io::Result<bool> {
    file.seek(SeekFrom::Start(0))?;
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    for _ in 0..5 {
        buf.clear();
        if reader.read_line(&mut buf)? == 0 {
            break;
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        return Ok(line.starts_with('{'));
    }
    Ok(false)
}

struct FileTask {
    path: PathBuf,
    file_path: String,
    inode: Option<u64>,
    mtime: Option<String>,
    start_offset: u64,
    seed_model: Option<String>,
    seed_effort: Option<String>,
    prev_usage: Option<UsageTotals>,
}

struct ParsedFile {
    file_path: String,
    inode: Option<u64>,
    mtime: Option<String>,
    start_offset: u64,
    bytes_read: u64,
    events: Vec<UsageEvent>,
    message_events: Vec<MessageEvent>,
    limit_snapshots: Vec<UsageLimitSnapshot>,
    issues: Vec<IngestIssue>,
    last_model: Option<String>,
    last_effort: Option<String>,
    last_event_key: Option<String>,
    skipped: bool,
    parse_duration: StdDuration,
}

fn parse_file(
    task: FileTask,
    pricing: &[PricingRule],
    has_pricing: bool,
    timing_enabled: bool,
) -> ParsedFile {
    let file_start = Instant::now();
    let mut issues = Vec::new();
    let mut bytes_read = 0u64;
    let mut events = Vec::new();
    let mut limit_snapshots = Vec::new();
    let mut message_events = Vec::new();
    let mut current_model = task.seed_model;
    let mut current_effort = task.seed_effort;
    let mut prev_usage = task.prev_usage;

    let mut file = match File::open(&task.path) {
        Ok(file) => file,
        Err(err) => {
            issues.push(IngestIssue {
                file_path: task.file_path.clone(),
                message: err.to_string(),
            });
            return ParsedFile {
                file_path: task.file_path,
                inode: task.inode,
                mtime: task.mtime,
                start_offset: task.start_offset,
                bytes_read,
                events,
                message_events,
                limit_snapshots,
                issues,
                last_model: current_model,
                last_effort: current_effort,
                last_event_key: None,
                skipped: true,
                parse_duration: file_start.elapsed(),
            };
        }
    };

    if is_plain_log(&task.path) {
        match looks_like_jsonl(&mut file) {
            Ok(true) => {}
            Ok(false) => {
                return ParsedFile {
                    file_path: task.file_path,
                    inode: task.inode,
                    mtime: task.mtime,
                    start_offset: task.start_offset,
                    bytes_read,
                    events,
                    message_events,
                    limit_snapshots,
                    issues,
                    last_model: current_model,
                    last_effort: current_effort,
                    last_event_key: None,
                    skipped: true,
                    parse_duration: file_start.elapsed(),
                };
            }
            Err(err) => {
                issues.push(IngestIssue {
                    file_path: task.file_path.clone(),
                    message: err.to_string(),
                });
                return ParsedFile {
                    file_path: task.file_path,
                    inode: task.inode,
                    mtime: task.mtime,
                    start_offset: task.start_offset,
                    bytes_read,
                    events,
                    message_events,
                    limit_snapshots,
                    issues,
                    last_model: current_model,
                    last_effort: current_effort,
                    last_event_key: None,
                    skipped: true,
                    parse_duration: file_start.elapsed(),
                };
            }
        }
    }

    if let Err(err) = file.seek(SeekFrom::Start(task.start_offset)) {
        issues.push(IngestIssue {
            file_path: task.file_path.clone(),
            message: err.to_string(),
        });
        return ParsedFile {
            file_path: task.file_path,
            inode: task.inode,
            mtime: task.mtime,
            start_offset: task.start_offset,
            bytes_read,
            events,
            message_events,
            limit_snapshots,
            issues,
            last_model: current_model,
            last_effort: current_effort,
            last_event_key: None,
            skipped: true,
            parse_duration: file_start.elapsed(),
        };
    }

    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    let session_id = tracker_core::session_id_from_source(&task.file_path);

    loop {
        match reader.read_line(&mut buf) {
            Ok(0) => break,
            Ok(bytes) => {
                bytes_read = bytes_read.saturating_add(bytes as u64);
                let line = buf.trim_end_matches(&['\n', '\r'][..]);
                let Some(obj) = parse_json_line(line) else {
                    buf.clear();
                    continue;
                };
                if let Some(model) = extract_model(&obj) {
                    current_model = Some(model);
                }
                if let Some(effort) = extract_effort_if_turn_context(&obj) {
                    current_effort = Some(effort);
                }
                if let Some(mut event) = extract_usage_event_from_value(
                    &obj,
                    line,
                    &task.file_path,
                    current_model.as_deref(),
                    &session_id,
                    current_effort.as_deref(),
                ) {
                    let delta = delta_usage(prev_usage.as_ref(), event.usage);
                    if has_pricing
                        && let Some(cost) = compute_cost_for_event(pricing, &event, delta)
                    {
                        event.cost_usd = Some(cost);
                    }
                    prev_usage = Some(event.usage);
                    events.push(event);
                }
                if let Some(event) =
                    extract_message_event_from_value(&obj, line, &task.file_path, &session_id)
                {
                    message_events.push(event);
                }
                let mut snapshots = extract_limit_snapshots_from_value(&obj, line, &task.file_path);
                if !snapshots.is_empty() {
                    limit_snapshots.append(&mut snapshots);
                }
                buf.clear();
            }
            Err(err) => {
                issues.push(IngestIssue {
                    file_path: task.file_path.clone(),
                    message: err.to_string(),
                });
                break;
            }
        }
    }

    let parse_duration = file_start.elapsed();
    if timing_enabled {
        eprintln!(
            "ingest file: {} read={}ms db=0ms events={} bytes={}",
            task.file_path,
            parse_duration.as_millis(),
            events.len(),
            bytes_read
        );
    }

    let last_event_key = events.last().map(|event| event.id.clone());
    ParsedFile {
        file_path: task.file_path,
        inode: task.inode,
        mtime: task.mtime,
        start_offset: task.start_offset,
        bytes_read,
        events,
        message_events,
        limit_snapshots,
        issues,
        last_model: current_model,
        last_effort: current_effort,
        last_event_key,
        skipped: false,
        parse_duration,
    }
}

pub fn ingest_codex_home(db: &mut Db, codex_home: &Path) -> Result<IngestStats> {
    let mut stats = IngestStats::default();
    let pricing = std::sync::Arc::new(db.list_pricing_rules()?);
    let has_pricing = !pricing.is_empty();
    let timing_enabled = env::var("CODEX_TRACKER_INGEST_TIMING").is_ok();
    let ingest_start = Instant::now();
    let mut parse_total = StdDuration::ZERO;
    let mut db_total = StdDuration::ZERO;
    let codex_home_str = codex_home.to_string_lossy().to_string();
    let home = db.get_or_create_home(&codex_home_str, Some("Default"))?;
    db.update_home_last_seen(home.id)?;
    let sessions_dir = codex_home.join("sessions");
    if !sessions_dir.is_dir() {
        return Ok(stats);
    }

    let mut tasks = Vec::new();
    for entry in WalkDir::new(&sessions_dir).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let file_path = err
                    .path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| "<unknown>".to_string());
                stats.issues.push(IngestIssue {
                    file_path: file_path.clone(),
                    message: err.to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        if !entry.file_type().is_file() || !is_log_path(path) {
            continue;
        }
        stats.files_scanned += 1;
        let file_path = path.to_string_lossy().to_string();
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                stats.files_skipped += 1;
                stats.issues.push(IngestIssue {
                    file_path: file_path.clone(),
                    message: err.to_string(),
                });
                continue;
            }
        };
        let file_len = metadata.len();
        let inode = inode_from_metadata(&metadata);
        let mtime = metadata
            .modified()
            .ok()
            .map(|time| DateTime::<Utc>::from(time).to_rfc3339());
        let cursor = db.get_cursor(home.id, &file_path)?;
        let can_resume = matches!(
            cursor.as_ref(),
            Some(cursor) if cursor.byte_offset <= file_len && inode == cursor.inode
        );
        let (start_offset, seed_model, seed_effort) = match cursor.as_ref() {
            Some(cursor) if can_resume => (
                cursor.byte_offset,
                cursor.last_model.clone(),
                cursor.last_effort.clone(),
            ),
            _ => (0, None, None),
        };
        if start_offset >= file_len {
            stats.files_skipped += 1;
            continue;
        }
        let prev_usage = if can_resume {
            db.last_usage_totals_for_source(home.id, &file_path)?
        } else {
            None
        };
        tasks.push(FileTask {
            path: path.to_path_buf(),
            file_path,
            inode,
            mtime,
            start_offset,
            seed_model,
            seed_effort,
            prev_usage,
        });
    }

    let parsed_files = tasks
        .into_par_iter()
        .map(|task| parse_file(task, &pricing, has_pricing, timing_enabled))
        .collect::<Vec<_>>();

    let mut all_events = Vec::new();
    let mut all_message_events = Vec::new();
    let mut all_limit_snapshots = Vec::new();
    let mut cursors = Vec::new();
    for parsed in parsed_files {
        parse_total += parsed.parse_duration;
        stats.bytes_read += parsed.bytes_read;
        stats.issues.extend(parsed.issues);
        if parsed.skipped {
            stats.files_skipped += 1;
            continue;
        }
        all_events.extend(parsed.events);
        all_message_events.extend(parsed.message_events);
        all_limit_snapshots.extend(parsed.limit_snapshots);
        cursors.push(IngestCursor {
            codex_home_id: home.id,
            codex_home: codex_home_str.clone(),
            file_path: parsed.file_path,
            inode: parsed.inode,
            mtime: parsed.mtime,
            byte_offset: parsed.start_offset.saturating_add(parsed.bytes_read),
            last_event_key: parsed.last_event_key,
            updated_at: Utc::now().to_rfc3339(),
            last_model: parsed.last_model,
            last_effort: parsed.last_effort,
        });
    }

    let db_start = Instant::now();
    if !all_events.is_empty() {
        stats.events_inserted = db.insert_usage_events(home.id, &all_events)?;
    }
    if !all_message_events.is_empty() {
        let _ = db.insert_message_events(home.id, &all_message_events)?;
    }
    if !all_limit_snapshots.is_empty() {
        let _ = db.insert_limit_snapshots(home.id, &all_limit_snapshots)?;
    }
    for cursor in cursors {
        db.upsert_cursor(&cursor)?;
    }
    db_total += db_start.elapsed();

    if timing_enabled {
        eprintln!(
            "ingest total: files={} scanned={} skipped={} events={} read={}ms db={}ms total={}ms",
            stats.files_scanned + stats.files_skipped,
            stats.files_scanned,
            stats.files_skipped,
            stats.events_inserted,
            parse_total.as_millis(),
            db_total.as_millis(),
            ingest_start.elapsed().as_millis()
        );
    }
    Ok(stats)
}

fn inode_from_metadata(metadata: &fs::Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(metadata.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}
