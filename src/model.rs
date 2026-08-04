//! The domain model — plain types, no handles, no format opinions.

use serde::{Deserialize, Serialize};

/// One reported run, as a client submits it. `detail` carries the
/// kind-specific numbers (for behave: scenarios, failed) so a new
/// stat kind needs no schema change.
#[derive(Deserialize)]
pub struct NewRun {
    pub kind: String,
    pub host: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// One day of one kind, aggregated for the trends table.
#[derive(Serialize)]
pub struct Daily {
    pub day: String,
    pub kind: String,
    pub runs: i64,
    pub failed: i64,
    pub avg_ms: i64,
}

/// One stored run, as the recent-runs table shows it.
#[derive(Serialize)]
pub struct RunRow {
    pub received: String,
    pub kind: String,
    pub host: String,
    pub duration_ms: i64,
    pub detail: serde_json::Value,
}

/// Everything the run page needs, fetched in one trip through the
/// mapper.
#[derive(Serialize)]
pub struct Report {
    pub daily: Vec<Daily>,
    pub recent: Vec<RunRow>,
}
