//! Common error types.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TextError {
    #[error("font {key:?} not found")]
    FontMissing { key: String },
    #[error("invalid {tag} attribute {attr:?}")]
    InvalidAttr { tag: &'static str, attr: String },
    #[error("failed to parse {val:?} as value for {tag} attribute {attr:?}: {reason}")]
    InvalidAttrVal {
        tag: &'static str,
        attr: &'static str,
        val: String,
        reason: String,
    },
}

impl TextError {
    pub fn font_missing(key: impl AsRef<str>) -> Self {
        Self::FontMissing { key: key.as_ref().to_string() }
    }

    pub fn invalid_attr(tag: &'static str, attr: impl AsRef<str>) -> Self {
        Self::InvalidAttr { tag, attr: attr.as_ref().to_string() }
    }

    pub fn invalid_attr_val(
        tag: &'static str,
        attr: &'static str,
        val: impl AsRef<str>,
        reason: String,
    ) -> Self {
        Self::InvalidAttrVal { tag, attr, val: val.as_ref().to_string(), reason }
    }
}

#[derive(Debug, Clone, Copy, Error)]
#[error("failed to initialize font map")]
pub struct FontMapInitError;

#[derive(Debug, Error)]
pub enum FontMapError {
    #[error("failed to load font {key:?} from {}", path.display())]
    FileLoad { key: String, path: PathBuf },
    #[error("failed to load font {key:?}, {param} {value:?} contains invalid characters")]
    Load { key: String, param: &'static str, value: String },
    #[error("font {key:?} has no name")]
    Unnamed { key: String },
}

impl FontMapError {
    pub fn file_load(key: impl AsRef<str>, path: impl AsRef<Path>) -> Self {
        Self::FileLoad {
            key: key.as_ref().to_string(),
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load(key: impl AsRef<str>, param: &'static str, value: impl AsRef<str>) -> Self {
        Self::Load {
            key: key.as_ref().to_string(),
            param,
            value: value.as_ref().to_string(),
        }
    }

    pub fn unnamed(key: impl AsRef<str>) -> Self {
        Self::Unnamed { key: key.as_ref().to_string() }
    }
}

#[derive(Debug, Clone, Error)]
pub enum MarkupParseError {
    #[error("invalid input {slice:?}")]
    ScanError { slice: String },
    #[error("syntax error{}:\n{desc}", expected.as_ref().map(|e| format!(", expected {e}")).unwrap_or_default())]
    SyntaxError { desc: String, expected: Option<String> },
    #[error("{0}")]
    TextError(#[source] TextError),
}

impl From<TextError> for MarkupParseError {
    fn from(value: TextError) -> Self {
        Self::TextError(value)
    }
}

impl MarkupParseError {
    pub fn scan(slice: impl AsRef<str>) -> Self {
        Self::ScanError { slice: slice.as_ref().to_string() }
    }

    pub fn syntax_error_expecting(expected: &str, src: &str, i: usize) -> Self {
        Self::SyntaxError {
            desc: str_excerpt(10, i, src),
            expected: Some(expected.to_string()),
        }
    }

    pub fn syntax_error(src: &str, i: usize) -> Self {
        Self::SyntaxError { desc: str_excerpt(10, i, src), expected: None }
    }
}

#[derive(Debug, Error)]
pub enum ImgError {
    #[error("from vips: {source}{}", details.as_ref().map(|e| format!("\n{e}")).unwrap_or_default())]
    Vips {
        #[source]
        source: libvips::error::Error,
        details: Option<String>,
    },
    #[error("from cairo: {0}")]
    Cairo(#[source] cairo::Error),
    #[error("failed to convert image from {from} to {to}: {reason}")]
    Conversion { from: &'static str, to: &'static str, reason: String },
    #[error("text error: {0}")]
    Text(#[source] TextError),
    #[error("text parse error: {0}")]
    TextParse(#[source] MarkupParseError),
    #[error("artwork image not found for {key:?}")]
    NoArtwork { key: String },
}

impl ImgError {
    pub fn cairo_to_vips(reason: impl std::error::Error) -> Self {
        Self::Conversion { from: "cairo", to: "vips", reason: reason.to_string() }
    }

    pub fn no_artwork(key: impl AsRef<str>) -> Self {
        Self::NoArtwork { key: key.as_ref().to_string() }
    }
}

impl From<cairo::Error> for ImgError {
    fn from(source: cairo::Error) -> Self {
        Self::Cairo(source)
    }
}

impl From<TextError> for ImgError {
    fn from(source: TextError) -> Self {
        Self::Text(source)
    }
}

impl From<MarkupParseError> for ImgError {
    fn from(source: MarkupParseError) -> Self {
        Self::TextParse(source)
    }
}

#[derive(Debug, Clone, Error)]
pub enum PredicateParseError {
    #[error("invalid input {slice:?}")]
    ScanError { slice: String },
    #[error("syntax error{}:\n{desc}", expected.as_ref().map(|e| format!(", expected {e}")).unwrap_or_default())]
    SyntaxError { desc: String, expected: Option<String> },
    #[error("invalid operand for `{operator}`: expected {expected}, got {got}")]
    BadOperand { operator: String, expected: &'static str, got: String },
}

impl PredicateParseError {
    pub fn scan(slice: impl AsRef<str>) -> Self {
        Self::ScanError { slice: slice.as_ref().to_string() }
    }

    pub fn syntax_error_expecting(expected: &str, src: &str, i: usize) -> Self {
        Self::SyntaxError {
            desc: str_excerpt(10, i, src),
            expected: Some(expected.to_string()),
        }
    }

    pub fn syntax_error(src: &str, i: usize) -> Self {
        Self::SyntaxError { desc: str_excerpt(10, i, src), expected: None }
    }

    pub fn bad_operand(
        operator: impl std::fmt::Display,
        expected: &'static str,
        got: impl std::fmt::Display,
    ) -> Self {
        Self::BadOperand {
            operator: operator.to_string(),
            expected,
            got: got.to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to acquire read lock for `{variable}`: {reason}")]
    ReadLock { variable: &'static str, reason: String },
    #[error("failed to acquire write lock for {variable}: {reason}")]
    WriteLock { variable: &'static str, reason: String },
    #[error("failed to acquire lock for {variable}: {reason}")]
    MutexLock { variable: &'static str, reason: String },
    #[error("failed to send message to thread: {reason}")]
    ThreadSend { reason: String },
    #[error("failed to join thread {worker:02}")]
    ThreadJoin { worker: usize },
    #[error("i/o error: {0}")]
    Io(#[source] std::io::Error),
    #[error("{0}")]
    Img(#[source] ImgError),
    #[error("failed to open source: {reason}")]
    CantOpenSource { reason: String },
    #[error("failed to read from source: {reason}")]
    CantRead { reason: String },
    #[error("failed to init decoder: {reason}")]
    CantInitDecoder { reason: String },
    #[error("failed to decode card: {reason}")]
    CantDecode { reason: String },
    #[error("failed to write card: {reason}")]
    CantWrite { reason: String },
}

impl RuntimeError {
    pub fn read_lock(variable: &'static str, reason: impl std::error::Error) -> Self {
        Self::ReadLock { variable, reason: reason.to_string() }
    }

    pub fn write_lock(variable: &'static str, reason: impl std::error::Error) -> Self {
        Self::WriteLock { variable, reason: reason.to_string() }
    }

    pub fn mutex_lock(variable: &'static str, reason: impl std::error::Error) -> Self {
        Self::MutexLock { variable, reason: reason.to_string() }
    }

    pub fn thread_send(reason: impl std::error::Error) -> Self {
        Self::ThreadSend { reason: reason.to_string() }
    }

    pub fn thread_join(worker: usize) -> Self {
        Self::ThreadJoin { worker }
    }

    pub fn io_error(reason: std::io::Error) -> Self {
        Self::Io(reason)
    }

    pub fn cant_open_source(reason: impl std::error::Error) -> Self {
        Self::CantOpenSource { reason: reason.to_string() }
    }

    pub fn cant_read(reason: impl AsRef<dyn std::error::Error>) -> Self {
        Self::CantRead { reason: reason.as_ref().to_string() }
    }

    pub fn cant_init_decoder(reason: impl std::error::Error) -> Self {
        Self::CantInitDecoder { reason: reason.to_string() }
    }

    pub fn cant_decode(reason: impl std::error::Error) -> Self {
        Self::CantDecode { reason: reason.to_string() }
    }

    pub fn cant_write(reason: impl std::error::Error) -> Self {
        Self::CantWrite { reason: reason.to_string() }
    }
}

impl From<std::io::Error> for RuntimeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ImgError> for RuntimeError {
    fn from(e: ImgError) -> Self {
        Self::Img(e)
    }
}

pub(crate) fn str_excerpt(n: usize, index: usize, src: &str) -> String {
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
    let excerpt = format!("{prefix}{}{suffix}\n{padding}^", &src[start..end]);
    excerpt
}
