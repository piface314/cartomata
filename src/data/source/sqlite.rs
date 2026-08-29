//! Implementation for SQLite as card data source.

use crate::abox::AliasBox;
use crate::data::predicate::ValueSet;
use crate::data::source::Result as SrcResult;
use crate::data::{Card, DataSource, Predicate, PredicatePath, PredicatePathPart, Value};
use itertools::Itertools;
use thiserror::Error;
use rusqlite::types::{ToSqlOutput, Value as SqlValue, ValueRef as SqlValueRef};
use rusqlite::{params_from_iter, Connection, Error as SqliteError, Statement};
use serde::Deserialize;
use serde_rusqlite::{from_rows, DeserRows};
use std::fmt::Write;
use std::path::Path;

/// Configurations for reading a SQLite file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SqliteSourceConfig {
    /// The SELECT query to be executed without a predicate.
    pub query: String,
    /// The SELECT query to be executed with a predicate.
    /// The predicate is inserted in place of the first occurrence of the string `WHERE ?`.
    /// If `None`, the default query is used instead, and the predicate is appended at the end
    /// of the string.
    pub with_predicate: Option<String>,
}

/// A reader for a SQLite file as a card data source.
///
/// # Example
/// ```
/// use cartomata::data::source::{DataSource, SqliteSource, SqliteSourceConfig};
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
/// let path = "examples/sample.db".to_string();
/// let config = SqliteSourceConfig { query: "SELECT * FROM card".into(), with_predicate: None };
/// let mut sqlite_source = SqliteSource::open(config, &path).unwrap();
/// let cards: Vec<Result<MyCard>> = sqlite_source.read(None).unwrap().collect();
/// assert_eq!(cards[0], Ok(MyCard { id: 271, name: "E".to_string(), power: 2.71 }));
///
/// let config = SqliteSourceConfig { query: "SELECT * FROM card".into(), with_predicate: None };
/// let mut sqlite_source = SqliteSource::open(config, &path).unwrap();
/// let p = Predicate::from_string("power >= 3.0").unwrap();
/// let cards: Vec<Result<MyCard>> = sqlite_source.read(Some(p)).unwrap().collect();
/// assert_eq!(cards[0], Ok(MyCard { id: 314, name: "Pi".to_string(), power: 3.14 }));
/// ```
pub struct SqliteSource {
    query: String,
    with_predicate: Option<String>,
    connection: Connection,
}

impl SqliteSource {
    pub fn open(
        config: SqliteSourceConfig,
        path: impl AsRef<Path>,
    ) -> Result<SqliteSource, SqliteError> {
        let path = path.as_ref();
        let connection = Connection::open(path)?;
        Ok(Self {
            query: config.query,
            with_predicate: config.with_predicate,
            connection,
        })
    }
}

impl<'s, C: Card> DataSource<C> for SqliteSource {
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> SrcResult<Box<dyn Iterator<Item = SrcResult<C>> + '_>> {
        let (stmt, vars) = match &filter {
            Some(filter) => {
                let (clause, vars) = filter.where_clause()?;
                let query = self
                    .with_predicate
                    .as_ref()
                    .map(|q| q.replacen("WHERE ?", &clause, 1))
                    .unwrap_or_else(|| {
                        let mut query = self.query.to_string();
                        query.push(' ');
                        query.push_str(&clause);
                        query
                    });
                self.connection.prepare(&query).map(|stmt| (stmt, vars))?
            }
            None => self
                .connection
                .prepare(&self.query)
                .map(|stmt| (stmt, Vec::new()))?,
        };

        let mut stmt = AliasBox::new(stmt);
        let rows = from_rows::<C>(stmt.query(params_from_iter(vars.iter()))?);
        let rows = unsafe { std::mem::transmute(rows) };
        Ok(Box::new(SqliteIterator { rows, _stmt: stmt }))
    }
}

struct SqliteIterator<'c, C: Card> {
    // actually has lifetime of `_stmt``
    rows: DeserRows<'static, C>,
    // SAFETY: we must never move out of this box as long as `rows` is alive
    _stmt: AliasBox<Statement<'c>>,
}

impl<'c, C: Card> Iterator for SqliteIterator<'c, C> {
    type Item = SrcResult<C>;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.rows.next()?;
        Some(result.map_err(|e| e.into()))
    }
}

fn value_to_sql<'v>(value: &'v Value) -> ToSqlOutput<'v> {
    match value {
        Value::Bool(v) => ToSqlOutput::Owned(SqlValue::Integer(*v as i64)),
        Value::Number(x) => {
            if let Some(v) = x.as_i64() {
                ToSqlOutput::Owned(SqlValue::Integer(v))
            } else if let Some(v) = x.as_f64() {
                ToSqlOutput::Owned(SqlValue::Real(v))
            } else {
                ToSqlOutput::Owned(SqlValue::Null)
            }
        }
        Value::String(v) => ToSqlOutput::Borrowed(SqlValueRef::Text(v.as_bytes())),
        _ => ToSqlOutput::Owned(SqlValue::Null),
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

#[derive(Debug, Error)]
pub enum SqlitePredicateError {
    #[error("{0}")]
    Fmt(#[source] std::fmt::Error),
    #[error("only simple column names are allowed in predicates for SQLite, but got {0}")]
    ColOnly(String),
}

impl From<std::fmt::Error> for SqlitePredicateError {
    fn from(value: std::fmt::Error) -> Self {
        Self::Fmt(value)
    }
}

impl Predicate {
    /// Formats a predicate into a SQLite `WHERE` clause.
    fn where_clause(&'_ self) -> Result<(String, Vec<ToSqlOutput<'_>>), SqlitePredicateError> {
        let mut buf = String::from("WHERE ");
        let mut vars = Vec::new();
        self.sql_r(&mut buf, &mut vars)?;
        Ok((buf, vars))
    }

    fn sql_r<'a>(&'a self, buf: &mut String, vars: &mut Vec<ToSqlOutput<'a>>) -> Result<(), SqlitePredicateError> {
        match self {
            Self::And(a, b) => {
                seq_write!(buf; "("; a.sql_r(buf, vars); " AND "; b.sql_r(buf, vars); ")")
            }
            Self::Or(a, b) => {
                seq_write!(buf; "("; a.sql_r(buf, vars); " OR "; b.sql_r(buf, vars); ")")
            }
            Self::Not(a) => seq_write!(buf; "NOT "; a.sql_r(buf, vars)),
            Self::Eq(col, v) => {
                if let Value::Null = v {
                    write!(buf, "{} IS NULL", esc_col(col)?)?;
                } else {
                    write!(buf, "{} = ?", esc_col(col)?)?;
                    vars.push(value_to_sql(v));
                }
            }
            Self::Neq(col, v) => {
                write!(buf, "{} != ?", esc_col(col)?)?;
                vars.push(value_to_sql(v));
            }
            Self::In(col, ValueSet::Int(vs)) => {
                write!(buf, "{} IN ({})", esc_col(col)?, repeat_vars(vs.len()))?;
                vars.extend(vs.iter().map(|v| ToSqlOutput::Owned(SqlValue::Integer(*v))));
            }
            Self::In(col, ValueSet::Str(vs)) => {
                write!(buf, "{} IN ({})", esc_col(col)?, repeat_vars(vs.len()))?;
                vars.extend(
                    vs.iter()
                        .map(|v| ToSqlOutput::Borrowed(SqlValueRef::Text(v.as_bytes()))),
                );
            }
            Self::Like(col, v) => {
                write!(buf, "{} LIKE ?", esc_col(col)?)?;
                vars.push(ToSqlOutput::Owned(SqlValue::Text(format!("%{v}%"))));
            }
            Self::Lt(col, v) => {
                write!(buf, "{} < ?", esc_col(col)?)?;
                vars.push(value_to_sql(v));
            }
            Self::Le(col, v) => {
                write!(buf, "{} <= ?", esc_col(col)?)?;
                vars.push(value_to_sql(v));
            }
            Self::Gt(col, v) => {
                write!(buf, "{} > ?", esc_col(col)?)?;
                vars.push(value_to_sql(v));
            }
            Self::Ge(col, v) => {
                write!(buf, "{} >= ?", esc_col(col)?)?;
                vars.push(value_to_sql(v));
            }
        };
        Ok(())
    }
}

fn esc_col(p: &PredicatePath) -> Result<String, SqlitePredicateError> {
    if let [PredicatePathPart::Key(col)] = p.0.as_slice() {
        Ok(format!("`{}`", col.replace("`", "``")))
    } else {
        Err(SqlitePredicateError::ColOnly(p.to_string()))
    }
}

fn repeat_vars(n: usize) -> String {
    (0..n).map(|_| "?").join(", ")
}
