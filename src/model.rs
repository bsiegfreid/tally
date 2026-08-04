//! The domain model — plain types, no handles, no format opinions.

use serde::{Deserialize, Serialize};

/// One reported run, as a client submits it. `detail` carries the
/// kind-specific numbers (for behave: scenarios, failed) so a new
/// stat kind needs no schema change.
#[derive(Deserialize)]
pub struct NewRun {
    pub kind: String,
    pub host: String,
    /// `u32` on purpose: ~49 days of milliseconds is more than any
    /// run needs, and the conversion into `SQLite`'s `i64` is
    /// lossless. Prefer tightening a type over guarding a cast —
    /// `i64::from(u32)` cannot fail, so nothing needs checking.
    pub duration_ms: u32,
    /// `#[serde(default)]` makes the field optional on the wire; an
    /// absent `detail` arrives as `Value::Null` instead of a 400.
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
