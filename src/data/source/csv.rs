//! Contains implementation for CSV as card data source.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CsvSourceConfig {
    pub delimiter: Option<char>,
    pub columns: Option<Vec<String>>
}

