//! Implementations for different data sources.
//!
//! Each data source type has to be enabled with its respective feature, e.g. `csv`, `sqlite`.

#[cfg(feature = "csv")]
mod csv;
#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "csv")]
pub use crate::data::source::csv::{CsvSource, CsvSourceConfig};
#[cfg(feature = "sqlite")]
pub use crate::data::source::sqlite::{SqliteSource, SqliteSourceConfig};
use crate::data::Card;
use crate::data::Predicate;
use crate::error::Result;

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
