//! Contains implementations for different data sources.

pub mod csv;
pub mod sqlite;

pub use csv::{CsvSource, CsvSourceConfig};
pub use sqlite::{SqliteSource, SqliteSourceConfig};

use crate::data::card::Card;
use crate::error::Result;
use crate::template::Template;

use clap::ValueEnum;
use serde::Deserialize;

pub trait DataSource<'a, C: Card> {
    fn fetch(&mut self, ids: &Vec<String>) -> Vec<Result<C>>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataSourceType {
    /// CSV source
    Csv,
    /// SQLite source
    Sqlite,
}

impl DataSourceType {
    pub fn open<'a, C: Card>(
        &self,
        template: &'a Template,
        path: &impl AsRef<str>,
    ) -> Result<Box<dyn DataSource<'a, C> + 'a>> {
        match self {
            DataSourceType::Csv => {
                CsvSource::open(template, path).map(|s| Box::new(s) as Box<dyn DataSource<C>>)
            }
            DataSourceType::Sqlite => {
                SqliteSource::open(template, path).map(|s| Box::new(s) as Box<dyn DataSource<C>>)
            }
        }
    }
}
