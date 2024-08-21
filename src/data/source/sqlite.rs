//! Contains implementation for SQLite as card data source.

use crate::data::predicate::SetValue;
use crate::data::{Card, DataSource, Predicate, Value};
use crate::error::{Error, Result};

use itertools::Itertools;
use rusqlite::types::{ToSqlOutput, Value as SqlValue, ValueRef as SqlValueRef};
use rusqlite::{params_from_iter, Connection};
use serde::Deserialize;
use serde_rusqlite::from_rows;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SqliteSourceConfig {
    pub query: String,
    pub with_predicate: Option<String>,
}

pub struct SqliteSource<'a> {
    query: &'a str,
    with_predicate: Option<&'a str>,
    connection: Connection,
}

impl<'a> SqliteSource<'a> {
    pub fn open(
        config: &'a SqliteSourceConfig,
        path: &impl AsRef<Path>,
    ) -> Result<SqliteSource<'a>> {
        let path = path.as_ref();
        let connection = Connection::open(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_path_buf(), e.to_string()))?;
        Ok(Self {
            query: config.query.as_str(),
            with_predicate: config.with_predicate.as_ref().map(String::as_str),
            connection,
        })
    }
}

impl<'a, C: Card> DataSource<'a, C> for SqliteSource<'a> {
    fn read(&mut self, filter: Option<&Predicate>) -> Vec<Result<C>> {
        let stmt_result = match filter {
            Some(filter) => match filter.where_clause() {
                Ok((clause, vars)) => {
                    let query = self
                        .with_predicate
                        .map(|q| q.replacen("WHERE ?", &clause, 1))
                        .unwrap_or_else(|| {
                            let mut query = self.query.to_string();
                            query.push(' ');
                            query.push_str(&clause);
                            query
                        });
                    self.connection
                        .prepare(&query)
                        .map_err(|e| Error::FailedPrepDataSource(e.to_string()))
                        .map(|stmt| (stmt, vars))
                }
                Err(e) => Err(e),
            },
            None => self
                .connection
                .prepare(self.query)
                .map_err(|e| Error::FailedPrepDataSource(e.to_string()))
                .map(|stmt| (stmt, Vec::new())),
        };

        if let Err(e) = stmt_result {
            return vec![Err(e)];
        }

        let (mut stmt, vars) = stmt_result.unwrap();
        let query_result = stmt
            .query(params_from_iter(vars.iter()))
            .map_err(|e| Error::FailedPrepDataSource(e.to_string()));

        if let Err(e) = query_result {
            return vec![Err(e)];
        }

        from_rows::<C>(query_result.unwrap())
            .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())))
            .collect()
    }
}

impl Value {
    fn to_sql<'a>(&'a self) -> ToSqlOutput<'a> {
        match self {
            Value::Bool(v) => ToSqlOutput::Owned(SqlValue::Integer(*v as i64)),
            Value::Int(v) => ToSqlOutput::Owned(SqlValue::Integer(*v)),
            Value::Float(v) => ToSqlOutput::Owned(SqlValue::Real(*v)),
            Value::Str(v) => ToSqlOutput::Borrowed(SqlValueRef::Text(v.as_bytes())),
            Value::Nil => ToSqlOutput::Owned(SqlValue::Null),
        }
    }
}

macro_rules! seq_write {
    ($f:ident; $str:literal) => {
        write!($f, $str)?
    };
    ($f:ident; ($str:literal, $($v:expr),*)) => {
        write!($f, $str, $($v),*)?
    };
    ($_:ident; $fn:expr) => {
        $fn?
    };
    ($f:ident; $($v:expr);*) => {{
        $(seq_write!($f; $v);)*
    }};
}

impl Predicate {
    pub fn where_clause(&self) -> Result<(String, Vec<ToSqlOutput>)> {
        let mut buf = String::from("WHERE ");
        let mut vars = Vec::new();
        self.sql_r(&mut buf, &mut vars)
            .map_err(|e| Error::FailedPrepDataSource(e.to_string()))?;
        Ok((buf, vars))
    }

    fn sql_r<'a>(&'a self, buf: &mut String, vars: &mut Vec<ToSqlOutput<'a>>) -> std::fmt::Result {
        match self {
            Self::And(a, b) => {
                seq_write!(buf; "("; a.sql_r(buf, vars); " AND "; b.sql_r(buf, vars); ")")
            }
            Self::Or(a, b) => {
                seq_write!(buf; "("; a.sql_r(buf, vars); " OR "; b.sql_r(buf, vars); ")")
            }
            Self::Not(a) => seq_write!(buf; "NOT "; a.sql_r(buf, vars)),
            Self::Eq(col, v) => {
                write!(buf, "{} = ?", esc_col(col))?;
                vars.push(v.to_sql());
            }
            Self::Neq(col, v) => {
                write!(buf, "{} != ?", esc_col(col))?;
                vars.push(v.to_sql());
            }
            Self::In(col, SetValue::IntSet(vs)) => {
                write!(buf, "{} IN ({})", esc_col(col), repeat_vars(vs.len()))?;
                vars.extend(vs.iter().map(|v| ToSqlOutput::Owned(SqlValue::Integer(*v))));
            }
            Self::In(col, SetValue::StrSet(vs)) => {
                write!(buf, "{} IN ({})", esc_col(col), repeat_vars(vs.len()))?;
                vars.extend(vs.iter().map(|v| ToSqlOutput::Borrowed(SqlValueRef::Text(v.as_bytes()))));
            }
            Self::Like(col, v) => {
                write!(buf, "{} LIKE ?", esc_col(col))?;
                vars.push(ToSqlOutput::Owned(SqlValue::Text(format!("%{v}%"))));
            }
            Self::Lt(col, v) => {
                write!(buf, "{} < ?", esc_col(col))?;
                vars.push(v.to_sql());
            }
            Self::Le(col, v) => {
                write!(buf, "{} <= ?", esc_col(col))?;
                vars.push(v.to_sql());
            }
            Self::Gt(col, v) => {
                write!(buf, "{} > ?", esc_col(col))?;
                vars.push(v.to_sql());
            }
            Self::Ge(col, v) => {
                write!(buf, "{} >= ?", esc_col(col))?;
                vars.push(v.to_sql());
            }
        };
        Ok(())
    }
}

fn esc_col(s: impl AsRef<str>) -> String {
    format!("`{}`", s.as_ref().replace("`", "``"))
}

fn repeat_vars(n: usize) -> String {
    (0..n).map(|_| "?").join(", ")
}
