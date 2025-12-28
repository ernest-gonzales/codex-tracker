use crate::error::Result;
use crate::pricing;
use crate::services::{open_db, require_active_home, SharedConfig};
use tracker_core::PricingRuleInput;
use tracker_db::Db;

#[derive(Clone)]
pub struct PricingService {
    config: SharedConfig,
}

impl PricingService {
    pub(super) fn new(config: SharedConfig) -> Self {
        Self { config }
    }

    fn db(&self) -> Result<Db> {
        open_db(&self.config)
    }

    pub fn list_rules(&self) -> Result<Vec<tracker_core::PricingRule>> {
        let db = self.db()?;
        Ok(db.list_pricing_rules()?)
    }

    pub fn replace_rules(&self, rules: &[PricingRuleInput]) -> Result<usize> {
        let mut db = self.db()?;
        let updated = db.replace_pricing_rules(rules)?;
        if let Err(err) = pricing::write_pricing_defaults(&self.config.pricing_defaults_path, rules)
        {
            eprintln!("failed to update pricing defaults: {}", err);
        }
        Ok(updated)
    }

    pub fn recompute_costs(&self) -> Result<usize> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.update_event_costs(home.id)?)
    }
}
