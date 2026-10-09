//! Implementations for different data sources.
//!
//! Each data source type has to be enabled with its respective feature, e.g. `csv`, `sqlite`.

#[cfg(feature = "csv")]
mod csv;
mod json;
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "yaml")]
mod yaml;

use crate::data::Card;
use crate::data::Predicate;
#[cfg(feature = "csv")]
pub use csv::{CsvSource, CsvSourceConfig};
pub use json::JsonSource;
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteSource, SqliteSourceConfig};
use std::error::Error;
#[cfg(feature = "toml")]
pub use toml::TomlSource;
#[cfg(feature = "yaml")]
pub use yaml::YamlSource;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// A data source, once created, can return an iterator of cards, optionally
/// accepting a predicate to filter which cards should be processed.
pub trait DataSource<C: Card>: Send {
    /// Reads the data source, returning an iterator of cards. By passing a predicate as parameter,
    /// the data source might use it directly over the cards after reading them, or use a more
    /// specific and more efficient way to filter cards while reading them.
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> Result<Box<dyn Iterator<Item = Result<C>> + '_>>;
}
