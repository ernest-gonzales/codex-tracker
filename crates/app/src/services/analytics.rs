use chrono::{Duration, SecondsFormat, Utc};

use crate::error::Result;
use crate::services::{open_db, require_active_home, SharedConfig};
use tracker_core::{
    ActiveSession, ContextPressureStats, ContextStatus, ModelBreakdown, ModelCostBreakdown,
    ModelEffortCostBreakdown, ModelEffortTokenBreakdown, ModelTokenBreakdown, TimeRange,
    TimeSeriesPoint, UsageEvent, UsageSummary,
};
use tracker_db::{Bucket, Db, Metric};

#[derive(Clone)]
pub struct AnalyticsService {
    config: SharedConfig,
}

impl AnalyticsService {
    pub(super) fn new(config: SharedConfig) -> Self {
        Self { config }
    }

    fn db(&self) -> Result<Db> {
        open_db(&self.config)
    }

    pub fn summary(&self, range: &TimeRange) -> Result<UsageSummary> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.summary(range, home.id)?)
    }

    pub fn context_latest(&self) -> Result<Option<ContextStatus>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.latest_context(home.id)?)
    }

    pub fn context_sessions(&self, active_minutes: Option<u32>) -> Result<Vec<ActiveSession>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        let minutes = match active_minutes {
            Some(value) => value,
            None => db.get_context_active_minutes()?,
        };
        let since = (Utc::now() - Duration::minutes(minutes as i64))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(db.active_sessions(home.id, &since)?)
    }

    pub fn context_stats(&self, range: &TimeRange) -> Result<ContextPressureStats> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.context_pressure_stats(range, home.id)?)
    }

    pub fn timeseries(
        &self,
        range: &TimeRange,
        bucket: Bucket,
        metric: Metric,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.timeseries(range, bucket, metric, home.id)?)
    }

    pub fn breakdown(&self, range: &TimeRange) -> Result<Vec<ModelBreakdown>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.breakdown_by_model(range, home.id)?)
    }

    pub fn breakdown_tokens(&self, range: &TimeRange) -> Result<Vec<ModelTokenBreakdown>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.breakdown_by_model_tokens(range, home.id)?)
    }

    pub fn breakdown_costs(&self, range: &TimeRange) -> Result<Vec<ModelCostBreakdown>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.breakdown_by_model_costs(range, home.id)?)
    }

    pub fn breakdown_effort_tokens(
        &self,
        range: &TimeRange,
    ) -> Result<Vec<ModelEffortTokenBreakdown>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.breakdown_by_model_effort_tokens(range, home.id)?)
    }

    pub fn breakdown_effort_costs(
        &self,
        range: &TimeRange,
    ) -> Result<Vec<ModelEffortCostBreakdown>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.breakdown_by_model_effort_costs(range, home.id)?)
    }

    pub fn events(
        &self,
        range: &TimeRange,
        model: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<UsageEvent>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.list_usage_events(range, model, limit, offset, home.id)?)
    }
}
