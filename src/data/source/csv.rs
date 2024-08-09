//! Contains implementation for CSV as card data source.

use crate::data::source::DataSource;
use crate::data::Card;
use crate::error::{Error, Result};

use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Deserialize, Copy, Clone)]
pub struct CsvSourceConfig {
    #[serde(default = "default_delimiter")]
    pub delimiter: char,
    #[serde(default = "default_header")]
    pub header: bool,
}

fn default_delimiter() -> char {
    ','
}

fn default_header() -> bool {
    true
}

impl Default for CsvSourceConfig {
    fn default() -> Self {
        CsvSourceConfig {
            delimiter: default_delimiter(),
            header: default_header(),
        }
    }
}

pub struct CsvSource {
    reader: csv::Reader<std::fs::File>,
}

impl<'a> CsvSource {
    pub fn open(config: &CsvSourceConfig, path: &impl AsRef<Path>) -> Result<CsvSource> {
        let path = path.as_ref();
        let reader = csv::ReaderBuilder::new()
            .delimiter(config.delimiter as u8)
            .has_headers(config.header)
            .from_path(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_path_buf(), e.to_string()))?;
        Ok(Self { reader })
    }
}

impl<'a, C: Card> DataSource<'a, C> for CsvSource {
    fn read(&mut self, ids: &Vec<String>) -> Vec<Result<C>> {
        let iterator = self
            .reader
            .deserialize::<C>()
            .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())));

        if ids.is_empty() {
            iterator.collect()
        } else {
            let ids: HashSet<&str> = ids.iter().map(|id| id.as_str()).collect();
            iterator
                .filter_ok(|card| {
                    card.get_string("id")
                        .map(|id| ids.contains(id.as_str()))
                        .unwrap_or(false)
                })
                .collect()
        }
    }
}
