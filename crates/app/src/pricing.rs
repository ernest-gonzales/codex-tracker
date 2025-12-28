use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use crate::error::{AppError, Result};
use tracker_core::PricingRuleInput;
use tracker_db::Db;

pub fn apply_pricing_defaults(db_path: &Path, defaults_path: &Path) -> Result<()> {
    let rules = if defaults_path.exists() {
        load_pricing_defaults(defaults_path)?
    } else {
        load_initial_pricing()?
    };
    let mut db = Db::open(db_path)?;
    db.replace_pricing_rules(&rules)?;
    Ok(())
}

pub fn sync_pricing_defaults(db_path: &Path, defaults_path: &Path) -> Result<()> {
    let db = Db::open(db_path)?;
    let rules = db.list_pricing_rules()?;
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

pub fn load_pricing_defaults(path: &Path) -> Result<Vec<PricingRuleInput>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(AppError::from)
}

pub fn load_initial_pricing() -> Result<Vec<PricingRuleInput>> {
    let data = include_str!("../initial-pricing.json");
    serde_json::from_str(data).map_err(AppError::from)
}

pub fn write_pricing_defaults(path: &Path, rules: &[PricingRuleInput]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, rules).map_err(AppError::from)
}
