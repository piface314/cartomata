use crate::cli::card::DynCard;
#[cfg(feature = "csv")]
use crate::data::source::{CsvSource, CsvSourceConfig};
use crate::data::source::{DataSource, SourceMap};
#[cfg(feature = "sqlite")]
use crate::data::source::{SqliteSource, SqliteSourceConfig};
use crate::error::{Error, Result};

use clap::ValueEnum;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub struct DynSourceMap {
    pub sqlite: Option<SqliteSourceConfig>,
    pub csv: Option<CsvSourceConfig>,
}

impl DynSourceMap {
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
}

impl SourceMap for DynSourceMap {
    type C = DynCard;
    type K = (Option<SourceType>, PathBuf);

    fn source<'s>(&'s self, key: &Self::K) -> Result<Box<dyn DataSource<'s, Self::C> + 's>> {
        let (src_type, path) = key;
        let src_type = src_type
            .or_else(|| Self::infer_source_type(path))
            .ok_or_else(|| Error::SourceInferError(path.clone()))?;
        match src_type {
            #[cfg(feature = "csv")]
            SourceType::Csv => {
                let config = self.csv.unwrap_or_default();
                CsvSource::open(&config, path).map(|s| Box::new(s) as Box<dyn DataSource<Self::C>>)
            }
            #[cfg(feature = "sqlite")]
            SourceType::Sqlite => {
                let config = self
                    .sqlite
                    .as_ref()
                    .ok_or(Error::MissingSourceConfig("sqlite"))?;
                SqliteSource::open(&config, path)
                    .map(|s| Box::new(s) as Box<dyn DataSource<Self::C>>)
            }
        }
    }
}
