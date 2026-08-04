//! All configuration in one place. Nothing else reads the
//! environment.

use std::env;

pub struct Config {
    pub bind_addr: String,
    pub db_path: String,
}

impl Config {
    /// Read configuration from the environment. Call once at
    /// startup.
    ///
    /// Returns a fully populated `Config`: any variable that is
    /// unset falls back to a default that works for local
    /// development. Loading cannot fail, so there is no `Result`.
    pub fn load() -> Self {
        // `unwrap_or_else` takes a closure, so the default String is
        // only built when the variable is missing; `unwrap_or` would
        // build it eagerly on every call. Lazy for computed defaults,
        // eager only for values that cost nothing.
        Self {
            bind_addr: env::var("TALLY_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            db_path: env::var("TALLY_DB").unwrap_or_else(|_| "tally.sqlite".into()),
        }
    }
}
