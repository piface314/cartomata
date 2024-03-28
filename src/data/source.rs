//! Contains implementations for different data sources.

pub mod csv;
pub mod sqlite;

pub use csv::{CsvSource, CsvSourceConfig};
pub use sqlite::{SqliteSource, SqliteSourceConfig};

use crate::data::GCard;
use crate::error::Result;
use crate::template::Template;

use clap::ValueEnum;
use serde::Deserialize;


pub trait DataSource<'a> {
    fn open(template: &'a Template, path: &impl AsRef<str>) -> Result<impl DataSource<'a>>;
    fn fetch_generic(&mut self, ids: &Vec<String>) -> Vec<Result<GCard<'a>>>;
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataSourceType {
    /// CSV source
    Csv,
    /// SQLite source
    Sqlite,
}
