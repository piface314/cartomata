//! Common error types.

/// A shortcut type equivalent to `Result<T, ril::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    MissingSourceConfig(&'static str),
    FailedOpenTemplate(String, String),
    FailedOpenDataSource(String, String),
    FailedPrepDataSource(String),
    MissingVariable(&'static str),
    FieldNotInSchema(String),
    MissingIdField,
    FailedRecordRead(String),
    FailedFieldRead(String),
    FailedOpenImage(String, String),
    FailedOpenDecoder(String, String),
    FailedPrepareDecoder(String),
    Decoding(String),
    ArtworkNotFound(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingSourceConfig(e) => write!(f, "Missing {e} source configuration"),
            Error::FailedOpenTemplate(path, e) => write!(f, "Failed to open template {path}:\n{e}"),
            Error::FailedOpenDataSource(path, e) => write!(f, "Failed to open data source {path}:\n{e}"),
            Error::FailedPrepDataSource(e) => write!(f, "Failed to prepare data source:\n{e}"),
            Error::MissingVariable(e) => write!(f, "Missing environment variable: {e}"),
            Error::FieldNotInSchema(e) => write!(f, "Field {e} not in schema"),
            Error::MissingIdField => write!(f, "Missing id field"),
            Error::FailedRecordRead(e) => write!(f, "Failed to read record {e}"),
            Error::FailedFieldRead(e) => write!(f, "Failed to read field {e}"),
            Error::FailedOpenImage(path, e) => write!(f, "Failed to open image {path}:\n{e}"),
            Error::FailedOpenDecoder(path, e) => write!(f, "Failed to open decoder {path}:\n{e}"),
            Error::FailedPrepareDecoder(e) => write!(f, "Failed to prepare decoder:\n{e}"),
            Error::Decoding(e) => write!(f, "Failed to run decoder:\n{e}"),
            Error::ArtworkNotFound(e) => write!(f, "Artwork image not found for {e}"),
        }   
    }
}
