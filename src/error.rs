//! Common error types.

use std::path::{Path, PathBuf};

/// A shortcut type equivalent to `Result<T, cartomata::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    NoSourceConfig {
        key: &'static str,
    },
    SourceInference {
        path: PathBuf,
    },
    NoEnvVariable {
        variable: &'static str,
    },
    ConfigOpen {
        path: PathBuf,
        reason: String,
    },
    ConfigDeser {
        path: PathBuf,
        reason: String,
    },
    SourceOpen {
        path: PathBuf,
        reason: String,
    },
    SourcePrep {
        reason: String,
    },
    RecordRead {
        reason: String,
    },
    DecoderOpen {
        path: PathBuf,
        reason: String,
    },
    DecoderPrep {
        reason: String,
    },
    Decode {
        reason: String,
    },
    NoArtwork {
        key: String,
    },
    ExternalError {
        source: &'static str,
        reason: String,
    },
    ScanError {
        slice: String,
    },
    TextInvalidAttr {
        tag: &'static str,
        attr: String,
    },
    TextInvalidAttrVal {
        tag: &'static str,
        attr: &'static str,
        val: String,
        reason: String,
    },
    FontMapInit,
    FontFileLoad {
        key: String,
        path: PathBuf,
    },
    FontLoad {
        key: String,
        param: &'static str,
        value: String,
    },
    FontUnnamed {
        key: String,
    },
    FontMissing {
        key: String,
    },
    ImageConversion {
        from: &'static str,
        to: &'static str,
        reason: String,
    },
    SyntaxError {
        desc: String,
        expected: Option<String>,
    },
    PredicateOperand {
        operator: String,
        expected: &'static str,
        got: String,
    },
    ReadLock {
        variable: &'static str,
        reason: String,
    },
    WriteLock {
        variable: &'static str,
        reason: String,
    },
    MutexLock {
        variable: &'static str,
        reason: String,
    },
    ThreadSend {
        reason: String,
    },
    ThreadJoin {
        worker: usize,
    },
    IoError {
        reason: std::io::Error,
    },
    Unknown,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoSourceConfig { key } => write!(f, "missing source configuration `{key}`"),
            Error::SourceInference { path } => {
                write!(f, "failed to infer source type for `{}`", path.display())
            }
            Error::NoEnvVariable { variable } => {
                write!(f, "missing environment variable `{variable}`")
            }
            Error::ConfigOpen { path, reason: cause } => {
                write!(
                    f,
                    "failed to open template configuration `{}`: {cause}",
                    path.display()
                )
            }
            Error::ConfigDeser { path, reason: cause } => {
                write!(
                    f,
                    "failed to load template configuration {}: {cause}",
                    path.display()
                )
            }
            Error::SourceOpen { path, reason } => {
                write!(f, "failed to open data source {}: {reason}", path.display())
            }
            Error::SourcePrep { reason } => write!(f, "failed to prepare data source: {reason}"),
            Error::RecordRead { reason } => write!(f, "failed to read record: {reason}"),
            Error::DecoderOpen { path, reason } => {
                write!(
                    f,
                    "Failed to open decoder at `{}`:\n{reason}",
                    path.display()
                )
            }
            Error::DecoderPrep { reason } => write!(f, "failed to prepare decoder: {reason}"),
            Error::Decode { reason } => write!(f, "failed to run decoder:\n{reason}"),
            Error::NoArtwork { key } => write!(f, "artwork image not found for `{key}`"),
            Error::ExternalError { source, reason } => write!(f, "from {source}: {reason}"),
            Error::ScanError { slice } => write!(f, "invalid input {slice:?}"),
            Error::TextInvalidAttr { tag, attr } => {
                write!(f, "invalid {tag} attribute `{attr}`")
            }
            Error::TextInvalidAttrVal { tag, attr, val, reason } => write!(
                f,
                "failed to parse {val:?} as value for {tag} attribute `{attr}`: {reason}"
            ),
            Error::FontMapInit => write!(f, "failed to initialize font map"),
            Error::FontFileLoad { key, path } => {
                write!(f, "failed to load font `{key}` from {}", path.display())
            }
            Error::FontLoad { key, param, value } => write!(
                f,
                "failed to load font `{key}`, {param} {value:?} contains invalid characters"
            ),
            Error::FontUnnamed { key } => write!(f, "font `{key}` has no name"),
            Error::FontMissing { key } => write!(f, "font `{key}` not found"),
            Error::ImageConversion { from, to, reason } => {
                write!(f, "failed to convert image from {from} to {to}: {reason}")
            }
            Error::SyntaxError { desc, expected: Some(expected) } => {
                write!(f, "syntax error, expected {expected}:\n{desc}")
            }
            Error::SyntaxError { desc, expected: None } => write!(f, "syntax error:\n{desc}"),
            Error::PredicateOperand { operator, expected, got } => {
                write!(
                    f,
                    "invalid operand for `{operator}`: expected {expected}, got {got}"
                )
            }
            Error::ReadLock { variable, reason } => {
                write!(f, "failed to acquire read lock for `{variable}`: {reason}")
            }
            Error::WriteLock { variable, reason } => {
                write!(f, "failed to acquire write lock for {variable}: {reason}")
            }
            Error::MutexLock { variable, reason } => {
                write!(f, "failed to acquire lock for {variable}: {reason}")
            }
            Error::ThreadSend { reason } => {
                write!(f, "failed to send message to thread: {reason}")
            }
            Error::ThreadJoin { worker } => write!(f, "failed to join thread {worker:02}"),
            Error::IoError { reason } => write!(f, "i/o error: {reason}"),
            _ => write!(f, "unexpected error"),
        }
    }
}

impl Error {
    pub fn no_source_config(key: &'static str) -> Self {
        Self::NoSourceConfig { key }
    }

    pub fn source_inference(path: impl AsRef<Path>) -> Self {
        Self::SourceInference { path: path.as_ref().to_path_buf() }
    }

    pub fn no_env_variable(variable: &'static str) -> Self {
        Self::NoEnvVariable { variable }
    }

    pub fn config_open(path: impl AsRef<Path>, reason: impl std::error::Error) -> Self {
        Self::ConfigOpen {
            path: path.as_ref().to_path_buf(),
            reason: reason.to_string(),
        }
    }

    pub fn config_deser(path: impl AsRef<Path>, reason: impl std::error::Error) -> Self {
        Self::ConfigDeser {
            path: path.as_ref().to_path_buf(),
            reason: reason.to_string(),
        }
    }

    pub fn source_open(path: impl AsRef<Path>, reason: impl std::error::Error) -> Self {
        Self::SourceOpen {
            path: path.as_ref().to_path_buf(),
            reason: reason.to_string(),
        }
    }

    pub fn source_prep(reason: impl std::error::Error) -> Self {
        Self::SourcePrep { reason: reason.to_string() }
    }

    pub fn record_read(reason: impl std::error::Error) -> Self {
        Self::RecordRead { reason: reason.to_string() }
    }

    pub fn decoder_open(path: impl AsRef<Path>, reason: impl std::error::Error) -> Self {
        Self::DecoderOpen {
            path: path.as_ref().to_path_buf(),
            reason: reason.to_string(),
        }
    }

    pub fn decoder_prep(reason: impl std::error::Error) -> Self {
        Self::DecoderPrep { reason: reason.to_string() }
    }

    pub fn decode(reason: impl std::error::Error) -> Self {
        Self::Decode { reason: reason.to_string() }
    }

    pub fn no_artwork(key: impl AsRef<str>) -> Self {
        Self::NoArtwork { key: key.as_ref().to_string() }
    }

    pub fn vips(reason: libvips::error::Error, extra: Option<&str>) -> Self {
        Self::ExternalError {
            source: "libvips",
            reason: match extra {
                Some(e) => format!("{reason}\n{e}"),
                None => reason.to_string(),
            },
        }
    }

    pub fn cairo(reason: cairo::Error) -> Self {
        Self::ExternalError { source: "cairo", reason: reason.to_string() }
    }

    pub fn scan(slice: impl AsRef<str>) -> Self {
        Self::ScanError { slice: slice.as_ref().to_string() }
    }

    pub fn text_invalid_attr(tag: &'static str, attr: impl AsRef<str>) -> Self {
        Self::TextInvalidAttr { tag, attr: attr.as_ref().to_string() }
    }

    pub fn text_invalid_attr_val(
        tag: &'static str,
        attr: &'static str,
        val: impl AsRef<str>,
        reason: String,
    ) -> Self {
        Self::TextInvalidAttrVal { tag, attr, val: val.as_ref().to_string(), reason }
    }

    pub fn font_file_load(key: impl AsRef<str>, path: impl AsRef<Path>) -> Self {
        Self::FontFileLoad {
            key: key.as_ref().to_string(),
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn font_load(key: impl AsRef<str>, param: &'static str, value: impl AsRef<str>) -> Self {
        Self::FontLoad {
            key: key.as_ref().to_string(),
            param,
            value: value.as_ref().to_string(),
        }
    }

    pub fn font_unnamed(key: impl AsRef<str>) -> Self {
        Self::FontUnnamed { key: key.as_ref().to_string() }
    }

    pub fn font_missing(key: impl AsRef<str>) -> Self {
        Self::FontMissing { key: key.as_ref().to_string() }
    }

    pub fn cairo_to_vips(reason: impl std::error::Error) -> Self {
        Self::ImageConversion { from: "cairo", to: "vips", reason: reason.to_string() }
    }

    pub fn predicate_operand(
        operator: impl std::fmt::Display,
        expected: &'static str,
        got: impl std::fmt::Display,
    ) -> Self {
        Self::PredicateOperand {
            operator: operator.to_string(),
            expected,
            got: got.to_string(),
        }
    }

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

    pub fn syntax_error_expecting(expected: &str, src: &str, i: usize) -> Self {
        Self::SyntaxError {
            desc: str_excerpt(10, i, src),
            expected: Some(expected.to_string()),
        }
    }

    pub fn syntax_error(src: &str, i: usize) -> Self {
        Self::SyntaxError { desc: str_excerpt(10, i, src), expected: None }
    }

    pub fn io_error(reason: std::io::Error) -> Self {
        Self::IoError { reason }
    }

    pub fn unknown() -> Self {
        Self::Unknown
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
