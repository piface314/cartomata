//! Implementation for CSV as card data source.

use crate::data::{Card, DataSource, Predicate};
use crate::error::{Error, Result};

use itertools::Itertools;
use serde::Deserialize;
use std::path::Path;

/// Configurations for reading a CSV file.
#[derive(Debug, Deserialize, Copy, Clone)]
pub struct CsvSourceConfig {
    /// Character that delimits the end of each field in a row. Defaults to `,`.
    #[serde(default = "default_delimiter")]
    pub delimiter: char,
    /// Whether the input file contains a header. Defaults to `true`.
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
        CsvSourceConfig { delimiter: default_delimiter(), header: default_header() }
    }
}

/// A reader for a CSV file as a card data source.
///
/// # Example
/// ```
/// use cartomata::data::source::{DataSource, CsvSource, CsvSourceConfig};
/// use cartomata::data::{Card, Predicate};
/// use cartomata::Result;
/// use serde::Deserialize;
///
/// #[derive(Debug, Card, Deserialize, PartialEq)]
/// struct MyCard {
///     id: i64,
///     name: String,
///     power: f64,
/// }
///
/// let path = "examples/sample.csv".to_string();
/// let mut csv_source = CsvSource::open(CsvSourceConfig::default(), &path).unwrap();
/// let cards: Vec<Result<MyCard>> = csv_source.read(None).unwrap().collect();
/// assert_eq!(cards[0], Ok(MyCard { id: 314, name: "Pi".to_string(), power: 3.14 }));
///
/// let mut csv_source = CsvSource::open(CsvSourceConfig::default(), &path).unwrap();
/// let p = Predicate::from_string("power < 3.0").unwrap();
/// let cards: Vec<Result<MyCard>> = csv_source.read(Some(p)).unwrap().collect();
/// assert_eq!(cards[0], Ok(MyCard { id: 271, name: "E".to_string(), power: 2.71 }));
/// ```
pub struct CsvSource {
    reader: csv::Reader<std::fs::File>,
}

impl CsvSource {
    /// Opens a CSV file according to the configurations, to be used a card data source.
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
