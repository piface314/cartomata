//! Contains implementation for CSV as card data source.

use crate::data::{Card, DataSource, Predicate};
use crate::error::{Error, Result};

use itertools::Itertools;
use serde::Deserialize;
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

impl CsvSource {
    pub fn open(config: CsvSourceConfig, path: &impl AsRef<Path>) -> Result<CsvSource> {
        let path = path.as_ref();
        let reader = csv::ReaderBuilder::new()
            .delimiter(config.delimiter as u8)
            .has_headers(config.header)
            .from_path(path)
            .map_err(|e| Error::source_open(path, e))?;
        Ok(Self { reader })
    }
}

impl<C: Card> DataSource<C> for CsvSource {
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> Result<Box<dyn Iterator<Item = Result<C>> + '_>> {
        let iterator = self
            .reader
            .deserialize::<C>()
            .map(|r| r.map_err(Error::record_read));

        match filter {
            Some(filter) => Ok(Box::new(iterator.filter_ok(move |card| filter.eval(card)))),
            None => Ok(Box::new(iterator)),
        }
    }
}
