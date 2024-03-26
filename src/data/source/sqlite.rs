//! Contains implementation for SQLite as card data source.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SqliteSourceConfig {
    pub query: String,
}

