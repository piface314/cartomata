//! Contains implementations for different data sources.

pub mod csv;
pub mod sqlite;

use clap::ValueEnum;
use serde::Deserialize;

// pub trait DataSource {
//     fn fetch_generic
// }


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataSourceType {
    /// CSV source
    Csv,
    /// SQLite source
    Sqlite,
}
