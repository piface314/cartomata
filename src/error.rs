//! Common error types.

use std::path::PathBuf;

/// A shortcut type equivalent to `Result<T, ril::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    MissingSourceConfig(&'static str),
    SourceInferError(PathBuf),
    FailedOpenTemplate(PathBuf, String),
    FailedOpenDataSource(PathBuf, String),
    FailedPrepDataSource(String),
    MissingVariable(&'static str),
    MissingIdField,
    FailedRecordRead(String),
    FailedFieldRead(String),
    FailedOpenImage(String, String),
    FailedOpenDecoder(String, String),
    FailedPrepareDecoder(String),
    Decoding(String),
    ArtworkNotFound(String),
    InvalidCString(String),
    CairoError(String),
    VipsError(String),
    ScanError(String),
    TextInvalidAttr(String),
    TextAttrParseError(String, String),
    FontConfigInitError,
    LoadFontError(String),
    FontUndefined(String),
    ImageConversionError(&'static str, &'static str),
    ImageCacheMiss(String),
    FontCacheMiss(String),
    SyntaxError {
        desc: String,
        expected: Option<String>,
    },
    InvalidOperand(String, String, String),
    ReadLockError(&'static str, String),
    WriteLockError(&'static str, String),
    MutexLockError(&'static str, String),
    SendError(usize, String),
    JoinError(usize),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingSourceConfig(e) => write!(f, "Missing {e} source configuration"),
            Error::SourceInferError(path) => {
                write!(f, "Failed to infer source type for {}", path.display())
            }
            Error::FailedOpenTemplate(path, e) => {
                write!(f, "Failed to open template {}:\n{e}", path.display())
            }
            Error::FailedOpenDataSource(path, e) => {
                write!(f, "Failed to open data source {}:\n{e}", path.display())
            }
            Error::FailedPrepDataSource(e) => write!(f, "Failed to prepare data source:\n{e}"),
            Error::MissingVariable(e) => write!(f, "Missing environment variable: {e}"),
            Error::MissingIdField => write!(f, "Missing id field"),
            Error::FailedRecordRead(e) => write!(f, "Failed to read record {e}"),
            Error::FailedFieldRead(e) => write!(f, "Failed to read field {e}"),
            Error::FailedOpenImage(path, e) => write!(f, "Failed to open image {path}:\n{e}"),
            Error::FailedOpenDecoder(path, e) => write!(f, "Failed to open decoder {path}:\n{e}"),
            Error::FailedPrepareDecoder(e) => write!(f, "Failed to prepare decoder:\n{e}"),
            Error::Decoding(e) => write!(f, "Failed to run decoder:\n{e}"),
            Error::ArtworkNotFound(e) => write!(f, "Artwork image not found for {e}"),
            Error::InvalidCString(e) => write!(f, "invalid string for C: {e}"),
            Error::CairoError(e) => write!(f, "Error from cairo: {e}"),
            Error::VipsError(e) => write!(f, "Error from libvips: {e}"),
            Error::ScanError(e) => write!(f, "invalid input: {e}"),
            Error::TextInvalidAttr(e) => write!(f, "invalid attribute: {e}"),
            Error::TextAttrParseError(k, e) => write!(f, "failed to parse value for {k}: {e}"),
            Error::FontConfigInitError => write!(f, "failed to initialize fontconfig"),
            Error::LoadFontError(e) => write!(f, "failed to load font {e}"),
            Error::FontUndefined(e) => write!(
                f,
                "font undefined: {e} (at least `path` or `family` must be specified)"
            ),
            Error::ImageConversionError(from, to) => {
                write!(f, "Failed to convert image from {from} to {to}")
            }
            Error::ImageCacheMiss(e) => write!(f, "{e} not in image cache"),
            Error::FontCacheMiss(e) => write!(f, "font not found: {e}"),
            Error::SyntaxError {
                desc,
                expected: Some(expected),
            } => write!(f, "syntax error, expected {expected}:\n{desc}"),
            Error::SyntaxError { desc, .. } => write!(f, "syntax error:\n{desc}"),
            Error::InvalidOperand(op, exp, got) => {
                write!(f, "invalid operand for {op}: expected {exp}, got {got}")
            }
            Error::ReadLockError(var, e) => {
                write!(f, "failed to acquire read lock for {var}:\n{e}")
            }
            Error::WriteLockError(var, e) => {
                write!(f, "failed to acquire write lock for {var}:\n{e}")
            }
            Error::MutexLockError(var, e) => write!(f, "failed to acquire lock for {var}:\n{e}"),
            Error::SendError(id, e) => {
                write!(f, "failed to send message from thread {id:02}:\n{e}")
            }
            Error::JoinError(id) => write!(f, "failed to join thread {id:02}"),
        }
    }
}

impl Error {
    pub fn syntax_error_expecting(expected: &str, src: &str, i: usize) -> Self {
        Self::SyntaxError {
            desc: str_excerpt(10, i, src),
            expected: Some(expected.to_string()),
        }
    }

    pub fn syntax_error(src: &str, i: usize) -> Self {
        Self::SyntaxError {
            desc: str_excerpt(10, i, src),
            expected: None,
        }
    }
}

fn str_excerpt(n: usize, index: usize, src: &str) -> String {
    let n_start = n / 2;
    let n_end = n - n_start;
    let mut start = index.saturating_sub(n_start); // i - st = nst
    let mut end = index.saturating_add(n_end).clamp(0, src.len());
    while start > 0 && !src.is_char_boundary(start) {
        start -= 1;
    }
    while end < src.len() && !src.is_char_boundary(end) {
        end += 1;
    }
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < src.len() { "..." } else { "" };
    let padding = " ".repeat(
        prefix.len()
            + src[start..]
                .char_indices()
                .take_while(|(i, _)| *i < index - start)
                .count(),
    );
    format!("{prefix}{}{suffix}\n{padding}^", &src[start..end])
}
