//! Contains implementation for SQLite as card data source.

use std::collections::HashMap;

use crate::data::source::DataSource;
use crate::data::{DynCard, Schema, Type, Value};
use crate::error::{Error, Result};
use crate::template::Template;

use itertools::Itertools;
use rusqlite::{params_from_iter, Connection};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SqliteSourceConfig {
    pub query: String,
}

pub struct SqliteSource<'a> {
    query: &'a str,
    connection: Connection,
    schema: &'a Schema,
}

impl<'a> SqliteSource<'a> {
    pub fn open(template: &'a Template, path: &impl AsRef<str>) -> Result<SqliteSource<'a>> {
        let config = template
            .source
            .sqlite
            .as_ref()
            .ok_or_else(|| Error::MissingSourceConfig("sqlite"))?;
        let schema = &template.schema;
        let path = path.as_ref();
        let connection = Connection::open(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_string(), e.to_string()))?;
        Ok(Self {
            query: &config.query,
            connection,
            schema,
        })
    }
}

impl<'a> DataSource<'a> for SqliteSource<'a> {
    fn fetch_dynamic(&mut self, ids: &Vec<String>) -> Vec<Result<DynCard<'a>>> {
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

        let rows_result = stmt
            .query_map(params_from_iter(ids), |row| {
                let mut card: DynCard = HashMap::new();
                for (field, ftype) in self.schema.iter() {
                    let field = field.as_str();
                    let v = match ftype {
                        Type::Int => row.get::<_, i64>(field).map(|v| Value::Int(v)),
                        Type::Float => row.get::<_, f64>(field).map(|v| Value::Float(v)),
                        Type::String => row.get::<_, String>(field).map(|s| Value::String(s)),
                        Type::Bool => row.get::<_, bool>(field).map(|v| Value::Bool(v)),
                    };
                    card.insert(field, v.unwrap_or(Value::Nil));
                }
                Ok(card)
            })
            .map_err(|e| Error::FailedPrepDataSource(e.to_string()));

        match rows_result {
            Err(e) => vec![Err(e)],
            Ok(rows) => rows
                .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())))
                .collect(),
        }
    }
}
