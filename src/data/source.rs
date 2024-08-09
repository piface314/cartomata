//! Contains implementations for different data sources.

#[cfg(feature = "csv")]
mod csv;
#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "csv")]
pub use csv::{CsvSource, CsvSourceConfig};
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteSource, SqliteSourceConfig};

use crate::data::Card;
use crate::error::Result;

pub trait DataSource<'a, C: Card> {
    fn read(&mut self, ids: &Vec<String>) -> Vec<Result<C>>;
}

pub trait SourceMap {
    type C: Card;
    type K;

    fn source<'s>(&'s self, key: &Self::K) -> Result<Box<dyn DataSource<'s, Self::C> + 's>>;
}
