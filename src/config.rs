//! All configuration in one place. Nothing else reads the
//! environment.

use std::env;

pub struct Config {
    pub bind_addr: String,
    pub db_path: String,
}

impl Config {
    /// Read configuration from the environment, with defaults that
    /// work for local development. Call once at startup.
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
