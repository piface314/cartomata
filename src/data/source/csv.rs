//! Contains implementation for CSV as card data source.

use crate::data::source::DataSource;
use crate::data::{DynCard, Type, Value};
use crate::error::{Error, Result};
use crate::template::Template;

use csv::StringRecord;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
pub struct CsvSourceConfig {
    pub delimiter: Option<char>,
    pub columns: Option<Vec<String>>,
}

pub struct CsvSource<'a> {
    id_column: (usize, Type),
    columns: HashMap<&'a str, (usize, Type)>,
    reader: csv::Reader<std::fs::File>,
}

impl<'a> CsvSource<'a> {
    pub fn open(template: &'a Template, path: &impl AsRef<str>) -> Result<CsvSource<'a>> {
        let config = template
            .source
            .csv
            .as_ref()
            .ok_or_else(|| Error::MissingSourceConfig("csv"))?;
        let schema = &template.schema;
        let path = path.as_ref();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(config.delimiter.map(|c| c as u8).unwrap_or(b','))
            .has_headers(config.columns.is_none())
            .from_path(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_string(), e.to_string()))?;
        let mut columns = HashMap::new();
        let icols: Vec<(usize, &str)> = match &config.columns {
            Some(cols) => cols.iter().map(|s| s.as_str()).enumerate().collect(),
            None => {
                let header = reader
                    .headers()
                    .map_err(|e| Error::FailedRecordRead(e.to_string()))?;
                header.iter().enumerate().collect()
            }
        };
        for (i, col) in icols.into_iter() {
            let (field, ftype) = schema
                .get_key_value(col)
                .ok_or_else(|| Error::FieldNotInSchema(col.to_string()))?;
            columns.insert(field.as_str(), (i, *ftype));
        }
        let id_column = *columns.get("id").ok_or_else(|| Error::MissingIdField)?;
        Ok(Self {
            id_column,
            columns,
            reader,
        })
    }
}

impl<'a> DataSource<'a> for CsvSource<'a> {
    fn fetch_generic(&mut self, ids: &Vec<String>) -> Vec<Result<DynCard<'a>>> {
        let iterator = self
            .reader
            .records()
            .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())));

        let read_card = |record: StringRecord| -> Result<DynCard> {
            let mut card = HashMap::new();
            for (field, (i, ftype)) in &self.columns {
                let v = record
                    .get(*i)
                    .ok_or_else(|| Error::FailedFieldRead(field.to_string()))?;
                card.insert(*field, Value::parse(*ftype, v));
            }
            Ok(card)
        };

        if ids.is_empty() {
            iterator.map(|r| read_card(r?)).collect()
        } else {
            let ids: HashSet<&str> = ids.iter().map(|id| id.as_str()).collect();
            iterator
                .map(|r| {
                    let record = r?;
                    let id = record
                        .get(self.id_column.0)
                        .ok_or_else(|| Error::MissingIdField)?;
                    if !ids.contains(id) {
                        return Ok(None);
                    }
                    read_card(record).map(|c| Some(c))
                })
                .filter_map_ok(|c| c)
                .collect()
        }
    }
}
