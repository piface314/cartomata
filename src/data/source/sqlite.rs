//! Contains implementation for SQLite as card data source.

use crate::data::predicate::SetValue;
use crate::data::{Card, DataSource, Predicate, Value};
use crate::error::{Error, Result};

use itertools::Itertools;
use rusqlite::types::{ToSqlOutput, Value as SqlValue, ValueRef as SqlValueRef};
use rusqlite::{params_from_iter, Connection, Statement};
use serde::Deserialize;
use serde_rusqlite::{from_rows, DeserRows};
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SqliteSourceConfig {
    pub query: String,
    pub with_predicate: Option<String>,
}

pub struct SqliteSource {
    query: String,
    with_predicate: Option<String>,
    connection: Connection,
}

impl SqliteSource {
    pub fn open(config: SqliteSourceConfig, path: impl AsRef<Path>) -> Result<SqliteSource> {
        let path = path.as_ref();
        let connection = Connection::open(path)
            .map_err(|e| Error::FailedOpenDataSource(path.to_path_buf(), e.to_string()))?;
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
                    .map_err(|e| Error::FailedPrepDataSource(e.to_string()))
                    .map(|stmt| (stmt, vars))?
            }
            None => self
                .connection
                .prepare(&self.query)
                .map_err(|e| Error::FailedPrepDataSource(e.to_string()))
                .map(|stmt| (stmt, Vec::new()))?,
        };

        let mut stmt = AliasBox::new(stmt);
        let rows = from_rows::<C>(
            stmt.query(params_from_iter(vars.iter()))
                .map_err(|e| Error::FailedPrepDataSource(e.to_string()))?,
        );
        let rows = unsafe { std::mem::transmute(rows) };
        Ok(Box::new(SqliteIterator { rows, _stmt: stmt }))
    }
}


struct SqliteIterator<'c, C: Card> {
    // actually has lifetime of `stmt``
    rows: DeserRows<'static, C>,
    // SAFETY: we must never move out of this box as long as `rows` is alive
    _stmt: AliasBox<Statement<'c>>,
}

struct AliasBox<T> {
    ptr: *const T,
}

impl<T> AliasBox<T> {
    pub fn new(value: T) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(value)),
        }
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr as *mut T
    }
}

impl<T> std::ops::Deref for AliasBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}
impl<T> std::ops::DerefMut for AliasBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.as_ptr() }
    }
}

impl<T> Drop for AliasBox<T> {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.ptr as *mut T));
        }
    }
}

impl<'c, C: Card> Iterator for SqliteIterator<'c, C> {
    type Item = Result<C>;
    fn next(&mut self) -> Option<Self::Item> {
        self.rows
            .next()
            .map(|r| r.map_err(|e| Error::FailedRecordRead(e.to_string())))
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
