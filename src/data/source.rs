//! Contains implementations for different data sources.

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

pub trait DataSource<C: Card>: Send {
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> Result<Box<dyn Iterator<Item = Result<C>> + '_>>;
}
