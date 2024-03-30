//! Contains implementation for CSV as card data source.

use crate::data::card::Card;
use crate::data::source::DataSource;
use crate::error::{Error, Result};
use crate::template::Template;

use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct CsvSourceConfig {
    pub delimiter: Option<char>,
    pub header: Option<bool>,
}

pub struct CsvSource {
    reader: csv::Reader<std::fs::File>,
}

impl<'a> CsvSource {
    pub fn open(template: &'a Template, path: &impl AsRef<str>) -> Result<CsvSource> {
        let config = template
            .source
            .csv
            .as_ref()
            .ok_or_else(|| Error::MissingSourceConfig("csv"))?;
        let path = path.as_ref();
        let reader = csv::ReaderBuilder::new()
            .delimiter(config.delimiter.map(|c| c as u8).unwrap_or(b','))
            .has_headers(config.header.unwrap_or(true))
            .from_path(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_string(), e.to_string()))?;
        Ok(Self { reader })
    }
}

impl<'a, C: Card> DataSource<'a, C> for CsvSource {
    fn fetch(&mut self, ids: &Vec<String>) -> Vec<Result<C>> {
        let iterator = self
            .reader
            .deserialize::<C>()
            .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())));

        if ids.is_empty() {
            iterator.collect()
        } else {
            let ids: HashSet<&str> = ids.iter().map(|id| id.as_str()).collect();
            iterator
                .filter_ok(|card| card.id().map_or(false, |id| ids.contains(id.as_str())))
                .collect()
        }
    }
}
