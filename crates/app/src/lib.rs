use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Duration, Local, SecondsFormat, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tracker_core::{PricingRuleInput, TimeRange};
use tracker_db::Db;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub pricing_defaults_path: PathBuf,
}

impl AppState {
    pub fn new(db_path: PathBuf, pricing_defaults_path: PathBuf) -> Self {
        Self {
            db_path,
            pricing_defaults_path,
        }
    }

    pub fn is_fresh_db(&self) -> bool {
        !self.db_path.exists()
    }

    pub fn setup_db(&self) -> Result<(), tracker_db::DbError> {
        setup_db(&self.db_path)
    }

    pub fn initialize(&self) -> Result<(), String> {
        let is_fresh_db = self.is_fresh_db();
        self.setup_db()
            .map_err(|err| format!("initialize db: {}", err))?;
        if is_fresh_db {
            self.apply_pricing_defaults()?;
        }
        self.sync_pricing_defaults()?;
        self.refresh_data()?;
        Ok(())
    }

    pub fn open_db(&self) -> Result<Db, tracker_db::DbError> {
        Db::open(&self.db_path)
    }

    pub fn apply_pricing_defaults(&self) -> Result<(), String> {
        apply_pricing_defaults(&self.db_path, &self.pricing_defaults_path)
    }

    pub fn sync_pricing_defaults(&self) -> Result<(), String> {
        sync_pricing_defaults(&self.db_path, &self.pricing_defaults_path)
    }

    pub fn refresh_data(&self) -> Result<(), String> {
        refresh_data(&self.db_path)
    }

    pub fn write_pricing_defaults(&self, rules: &[PricingRuleInput]) -> Result<(), String> {
        write_pricing_defaults(&self.pricing_defaults_path, rules)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RangeParams {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

pub fn resolve_range(params: &RangeParams) -> Result<TimeRange, String> {
    if let (Some(start), Some(end)) = (params.start.clone(), params.end.clone()) {
        let start = normalize_rfc3339_to_utc(&start)?;
        let end = normalize_rfc3339_to_utc(&end)?;
        return Ok(TimeRange { start, end });
    }
    if let Some(start) = params.start.clone() {
        let start = normalize_rfc3339_to_utc(&start)?;
        let end = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        return Ok(TimeRange { start, end });
    }
    let now_local = Local::now();
    let (start_local, end_local) = match params.range.as_deref().unwrap_or("last7days") {
        "today" => {
            let start = Local
                .with_ymd_and_hms(
                    now_local.year(),
                    now_local.month(),
                    now_local.day(),
                    0,
                    0,
                    0,
                )
                .single()
                .ok_or_else(|| "invalid local date".to_string())?;
            (start, now_local)
        }
        "last7days" => {
            let start = now_local - Duration::days(7);
            (start, now_local)
        }
        "last14days" => {
            let start = now_local - Duration::days(14);
            (start, now_local)
        }
        "thismonth" => {
            let start = Local
                .with_ymd_and_hms(now_local.year(), now_local.month(), 1, 0, 0, 0)
                .single()
                .ok_or_else(|| "invalid local date".to_string())?;
            (start, now_local)
        }
        "alltime" => {
            let start = Local
                .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| "invalid local date".to_string())?;
            (start, now_local)
        }
        value => {
            return Err(format!("unsupported range {}", value));
        }
    };
    let start = start_local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let end = end_local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    Ok(TimeRange { start, end })
}

pub fn normalize_rfc3339_to_utc(value: &str) -> Result<String, String> {
    let parsed =
        DateTime::parse_from_rfc3339(value).map_err(|err| format!("invalid datetime: {}", err))?;
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn setup_db(path: &Path) -> Result<(), tracker_db::DbError> {
    let mut db = Db::open(path)?;
    db.migrate()?;
    Ok(())
}

pub fn apply_pricing_defaults(db_path: &Path, defaults_path: &Path) -> Result<(), String> {
    let rules = if defaults_path.exists() {
        load_pricing_defaults(defaults_path)?
    } else {
        load_initial_pricing()?
    };
    let mut db = Db::open(db_path).map_err(|err| format!("open db: {}", err))?;
    db.replace_pricing_rules(&rules)
        .map_err(|err| format!("replace pricing: {}", err))?;
    Ok(())
}

pub fn sync_pricing_defaults(db_path: &Path, defaults_path: &Path) -> Result<(), String> {
    let db = Db::open(db_path).map_err(|err| format!("open db: {}", err))?;
    let rules = db
        .list_pricing_rules()
        .map_err(|err| format!("list pricing: {}", err))?;
    if rules.is_empty() && !defaults_path.exists() {
        return Ok(());
    }
    let inputs = rules
        .into_iter()
        .map(|rule| PricingRuleInput {
            model_pattern: rule.model_pattern,
            input_per_1m: rule.input_per_1m,
            cached_input_per_1m: rule.cached_input_per_1m,
            output_per_1m: rule.output_per_1m,
            effective_from: rule.effective_from,
            effective_to: rule.effective_to,
        })
        .collect::<Vec<_>>();
    write_pricing_defaults(defaults_path, &inputs)
}

pub fn load_pricing_defaults(path: &Path) -> Result<Vec<PricingRuleInput>, String> {
    let file = fs::File::open(path).map_err(|err| format!("open defaults: {}", err))?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|err| format!("parse defaults: {}", err))
}

pub fn load_initial_pricing() -> Result<Vec<PricingRuleInput>, String> {
    let data = include_str!("../initial-pricing.json");
    serde_json::from_str(data).map_err(|err| format!("parse initial pricing: {}", err))
}

pub fn write_pricing_defaults(path: &Path, rules: &[PricingRuleInput]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create defaults dir: {}", err))?;
    }
    let file = fs::File::create(path).map_err(|err| format!("create defaults: {}", err))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, rules).map_err(|err| format!("write defaults: {}", err))
}

pub fn refresh_data(path: &Path) -> Result<(), String> {
    let mut db = Db::open(path).map_err(|err| format!("open db: {}", err))?;
    let home = db
        .ensure_active_home()
        .map_err(|err| format!("ensure active home: {}", err))?;
    ingest::ingest_codex_home(&mut db, Path::new(&home.path))
        .map_err(|err| format!("ingest: {}", err))?;
    Ok(())
}
