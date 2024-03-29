//! Contains implementation for SQLite as card data source.

use std::collections::HashMap;

use crate::data::source::DataSource;
use crate::data::{DynCard, Schema, Type, Value};
use crate::error::{Error, Result};
use crate::template::Template;

use itertools::Itertools;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SqliteSourceConfig {
    pub query: String,
}

pub struct SqliteSource<'a> {
    query: &'a str,
    id_type: Type,
    connection: sqlite::Connection,
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
        let connection = sqlite::open(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_string(), e.to_string()))?;
        let id_type = *schema.get("id").ok_or_else(|| Error::MissingIdField)?;
        Ok(Self {
            query: &config.query,
            id_type,
            connection,
            schema,
        })
    }

    fn prepare(&self, ids: &Vec<String>) -> Result<sqlite::Statement> {
        let n = ids.len();
        if n == 0 {
            self.connection
                .prepare(self.query)
                .map_err(|e| Error::FailedPrepDataSource(e.to_string()))
        } else {
            let to_value: fn(&String) -> Result<sqlite::Value> = match self.id_type {
                Type::Int => |id| {
                    Ok(sqlite::Value::Integer(
                        id.parse::<i64>()
                            .map_err(|e| Error::FailedPrepDataSource(e.to_string()))?,
                    ))
                },
                Type::Float => |id| {
                    Ok(sqlite::Value::Float(
                        id.parse::<f64>()
                            .map_err(|e| Error::FailedPrepDataSource(e.to_string()))?,
                    ))
                },
                Type::String => |id| Ok(sqlite::Value::String(id.clone())),
            };

            let params = std::iter::repeat("?").take(n).join(", ");
            let query = format!("{} WHERE id IN ({})", self.query, params);
            let mut statement = self
                .connection
                .prepare(query)
                .map_err(|e| Error::FailedPrepDataSource(e.to_string()))?;
            for (i, id) in ids.iter().enumerate() {
                statement
                    .bind((i + 1, to_value(id)?))
                    .map_err(|e| Error::FailedPrepDataSource(e.to_string()))?;
            }
            Ok(statement)
        }
    }
}

impl<'a> DataSource<'a> for SqliteSource<'a> {
    fn fetch_generic(&mut self, ids: &Vec<String>) -> Vec<Result<DynCard<'a>>> {
        let res = self.prepare(ids);
        match res {
            Ok(stmt) => stmt
                .into_iter()
                .map(|r| {
                    let row = r.map_err(|e| Error::FailedRecordRead(e.to_string()))?;
                    let mut card = HashMap::new();
                    for (field, ftype) in self.schema.iter() {
                        let field = field.as_str();
                        let v = match ftype {
                            Type::Int => row.try_read::<i64, _>(field).map(|v| Value::Int(v)),
                            Type::Float => row.try_read::<f64, _>(field).map(|v| Value::Float(v)),
                            Type::String => row
                                .try_read::<&str, _>(field)
                                .map(|s| Value::String(s.to_string())),
                        };
                        card.insert(field, v.unwrap_or(Value::Nil));
                    }
                    Ok(card)
                })
                .collect(),
            Err(e) => vec![Err(e)],
        }
    }
}
