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
use crate::error::{Error, Result};

#[cfg(feature = "cli")]
use clap::ValueEnum;
use std::path::Path;

pub trait DataSource<C: Card> {
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> Result<Box<dyn Iterator<Item = Result<C>> + '_>>;
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum SourceType {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub struct SourceMap {
    #[cfg(feature = "csv")]
    csv: Option<CsvSourceConfig>,
    #[cfg(feature = "sqlite")]
    sqlite: Option<SqliteSourceConfig>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "csv")]
            csv: None,
            #[cfg(feature = "sqlite")]
            sqlite: None,
        }
    }

    #[cfg(feature = "csv")]
    pub fn with_csv(&mut self, cfg: Option<CsvSourceConfig>) {
        self.csv = cfg;
    }

    #[cfg(feature = "sqlite")]
    pub fn with_sqlite(&mut self, cfg: Option<SqliteSourceConfig>) {
        self.sqlite = cfg;
    }

    fn infer_source_type(path: impl AsRef<Path>) -> Option<SourceType> {
        let ext = path.as_ref().extension()?.to_str()?;
        match ext {
            #[cfg(feature = "csv")]
            "csv" | "tsv" => Some(SourceType::Csv),
            #[cfg(feature = "sqlite")]
            "db" | "cdb" => Some(SourceType::Sqlite),
            _ => None,
        }
    }

    pub fn select<C: Card>(
        self,
        src_type: Option<SourceType>,
        path: impl AsRef<Path>,
    ) -> Result<Box<dyn DataSource<C>>> {
        let path = path.as_ref();
        let src_type = src_type
            .or_else(|| Self::infer_source_type(path))
            .ok_or_else(|| Error::SourceInferError(path.to_path_buf()))?;
        match src_type {
            #[cfg(feature = "csv")]
            SourceType::Csv => {
                let config = self.csv.unwrap_or_default();
                let source = CsvSource::open(config, &path)?;
                Ok(Box::new(source) as Box<dyn DataSource<C>>)
            }
            #[cfg(feature = "sqlite")]
            SourceType::Sqlite => {
                let config = self.sqlite.ok_or(Error::MissingSourceConfig("sqlite"))?;
                let source = SqliteSource::open(config, &path)?;
                Ok(Box::new(source) as Box<dyn DataSource<C>>)
            }
        }
    }
}
