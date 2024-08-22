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
use std::marker::PhantomData;
use std::path::PathBuf;

pub trait DataSource<C: Card> {
    fn read(&mut self, filter: Option<&Predicate>) -> Vec<Result<C>>;
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum SourceType {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub struct SourceMap<C: Card> {
    #[cfg(feature = "csv")]
    pub csv: Option<CsvSourceConfig>,
    #[cfg(feature = "sqlite")]
    pub sqlite: Option<SqliteSourceConfig>,
    card_type: PhantomData<C>,
}

impl<C: Card> SourceMap<C> {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "csv")]
            csv: None,
            #[cfg(feature = "sqlite")]
            sqlite: None,
            card_type: PhantomData,
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

    fn infer_source_type(path: &PathBuf) -> Option<SourceType> {
        let ext = path.extension()?.to_str()?;
        match ext {
            #[cfg(feature = "csv")]
            "csv" | "tsv" => Some(SourceType::Csv),
            #[cfg(feature = "sqlite")]
            "db" | "cdb" => Some(SourceType::Sqlite),
            _ => None,
        }
    }

    pub fn select(
        self,
        src_type: Option<SourceType>,
        path: PathBuf,
    ) -> Result<Box<dyn DataSource<C>>> {
        let src_type = src_type
            .or_else(|| Self::infer_source_type(&path))
            .ok_or_else(|| Error::SourceInferError(path.clone()))?;
        match src_type {
            #[cfg(feature = "csv")]
            SourceType::Csv => {
                let config = self.csv.unwrap_or_default();
                CsvSource::open(config, &path).map(|s| Box::new(s) as Box<dyn DataSource<C>>)
            }
            #[cfg(feature = "sqlite")]
            SourceType::Sqlite => {
                let config = self
                    .sqlite
                    .ok_or(Error::MissingSourceConfig("sqlite"))?;
                SqliteSource::open(config, &path).map(|s| Box::new(s) as Box<dyn DataSource<C>>)
            }
        }
    }
}
