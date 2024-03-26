//! Common error types.

/// A shortcut type equivalent to `Result<T, ril::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    SQLiteError(sqlite::Error),
    CsvError(csv::Error),
    CsvSourceError(&'static str),
    UnknownCardFieldType(String),
    MissingVariable(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SQLiteError(e) => write!(f, "SQLite error: {e}"),
            Error::CsvError(e) => write!(f, "CSV error: {e}"),
            Error::CsvSourceError(e) => write!(f, "CSV error: {e}"),
            Error::UnknownCardFieldType(e) => write!(f, "Unknown card field type: {e}"),
            Error::MissingVariable(e) => write!(f, "Missing environment variable: {e}"),
        }   
    }
}
