//! Implementation for SQLite as card data source.

use crate::abox::AliasBox;
use crate::data::predicate::ValueSet;
use crate::data::{Card, DataSource, Predicate, Value};
use crate::error::{Error, Result};

use itertools::Itertools;
use rusqlite::types::{ToSqlOutput, Value as SqlValue, ValueRef as SqlValueRef};
use rusqlite::{params_from_iter, Connection, Statement};
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
    pub fn open(config: SqliteSourceConfig, path: impl AsRef<Path>) -> Result<SqliteSource> {
        let path = path.as_ref();
        let connection = Connection::open(path).map_err(|e| Error::source_open(path, e))?;
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
    ) -> Result<Box<dyn Iterator<Item = Result<C>> + '_>> {
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
                self.connection
                    .prepare(&query)
                    .map_err(Error::source_prep)
                    .map(|stmt| (stmt, vars))?
            }
            None => self
                .connection
                .prepare(&self.query)
                .map_err(Error::source_prep)
                .map(|stmt| (stmt, Vec::new()))?,
        };

        let mut stmt = AliasBox::new(stmt);
        let rows = from_rows::<C>(
            stmt.query(params_from_iter(vars.iter()))
                .map_err(Error::source_prep)?,
        );
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
    type Item = Result<C>;
    fn next(&mut self) -> Option<Self::Item> {
        self.rows.next().map(|r| r.map_err(Error::record_read))
    }
}

impl Value {
    /// Converts the value into a SQL compatible representation.
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
    /// Formats a predicate into a SQLite `WHERE` clause.
    pub fn where_clause(&self) -> Result<(String, Vec<ToSqlOutput>)> {
        let mut buf = String::from("WHERE ");
        let mut vars = Vec::new();
        self.sql_r(&mut buf, &mut vars)
            .map_err(Error::source_prep)?;
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
            Self::In(col, ValueSet::Int(vs)) => {
                write!(buf, "{} IN ({})", esc_col(col), repeat_vars(vs.len()))?;
                vars.extend(vs.iter().map(|v| ToSqlOutput::Owned(SqlValue::Integer(*v))));
            }
            Self::In(col, ValueSet::Str(vs)) => {
                write!(buf, "{} IN ({})", esc_col(col), repeat_vars(vs.len()))?;
                vars.extend(
                    vs.iter()
                        .map(|v| ToSqlOutput::Borrowed(SqlValueRef::Text(v.as_bytes()))),
                );
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
