//! Contains implementation for SQLite as card data source.

use crate::data::source::DataSource;
use crate::data::card::Card;
use crate::error::{Error, Result};
use crate::template::Template;

use itertools::Itertools;
use rusqlite::{params_from_iter, Connection};
use serde::Deserialize;
use serde_rusqlite::from_rows;

#[derive(Debug, Deserialize)]
pub struct SqliteSourceConfig {
    pub query: String,
}

pub struct SqliteSource<'a> {
    query: &'a str,
    connection: Connection,
}

impl<'a> SqliteSource<'a> {
    pub fn open(template: &'a Template, path: &impl AsRef<str>) -> Result<SqliteSource<'a>> {
        let config = template
            .source
            .sqlite
            .as_ref()
            .ok_or_else(|| Error::MissingSourceConfig("sqlite"))?;
        let path = path.as_ref();
        let connection = Connection::open(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_string(), e.to_string()))?;
        Ok(Self {
            query: &config.query,
            connection
        })
    }
}

impl<'a, C: Card> DataSource<'a, C> for SqliteSource<'a> {
    fn fetch(&mut self, ids: &Vec<String>) -> Vec<Result<C>> {
        let n = ids.len();
        let stmt_result = if n == 0 {
            self.connection.prepare(self.query)
        } else {
            let params = (1..=n).map(|i| format!("?{i}")).join(", ");
            let query = format!("{} WHERE id IN ({})", self.query, params);
            self.connection.prepare(query.as_str())
        }
        .map_err(|e| Error::FailedPrepDataSource(e.to_string()));

        if let Err(e) = stmt_result {
            return vec![Err(e)];
        }

        let mut stmt = stmt_result.unwrap();
        let query_result = stmt
            .query(params_from_iter(ids))
            .map_err(|e| Error::FailedPrepDataSource(e.to_string()));

        if let Err(e) = query_result {
            return vec![Err(e)];
        }

        from_rows::<C>(query_result.unwrap())
            .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())))
            .collect()
    }
}
