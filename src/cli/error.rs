//! Error types for cartomata CLI.

use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::error::{FontMapError, FontMapInitError, PredicateParseError, RuntimeError};

#[derive(Debug, Error)]
pub enum DynTemplateError {
    #[error("{0}")]
    DecoderOpen(#[source] DecoderOpenError),
    #[error("{0}")]
    FontMapInit(#[source] FontMapInitError),
    #[error("{0}")]
    FontMap(#[source] FontMapError),
}

impl From<DecoderOpenError> for DynTemplateError {
    fn from(value: DecoderOpenError) -> Self {
        Self::DecoderOpen(value)
    }
}

impl From<FontMapInitError> for DynTemplateError {
    fn from(value: FontMapInitError) -> Self {
        Self::FontMapInit(value)
    }
}

impl From<FontMapError> for DynTemplateError {
    fn from(value: FontMapError) -> Self {
        Self::FontMap(value)
    }
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("missing source configuration {key:?}")]
    MissingConfig { key: &'static str },
    #[error("failed to infer source type for {:?}", path.display())]
    CantInfer { path: PathBuf },
    #[cfg(feature = "csv")]
    #[error("failed to open csv data source {:?}: {reason}", path.display())]
    CantOpenCsv {
        #[source]
        reason: csv::Error,
        path: PathBuf,
    },
    #[cfg(feature = "sqlite")]
    #[error("failed to open sqlite data source {:?}: {reason}", path.display())]
    CantOpenSqlite {
        #[source]
        reason: rusqlite::Error,
        path: PathBuf,
    },
}

impl SourceError {
    pub fn missing_config(key: &'static str) -> Self {
        Self::MissingConfig { key }
    }

    pub fn cant_infer(path: impl AsRef<Path>) -> Self {
        Self::CantInfer { path: path.as_ref().to_path_buf() }
    }

    #[cfg(feature = "csv")]
    pub fn cant_open_csv(path: impl AsRef<Path>, reason: csv::Error) -> Self {
        Self::CantOpenCsv { reason, path: path.as_ref().to_path_buf() }
    }

    #[cfg(feature = "sqlite")]
    pub fn cant_open_sqlite(path: impl AsRef<Path>, reason: rusqlite::Error) -> Self {
        Self::CantOpenSqlite { reason, path: path.as_ref().to_path_buf() }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to open template configuration {:?}: {reason}", path.display())]
    Open {
        path: PathBuf,
        #[source]
        reason: std::io::Error,
    },
    #[error("failed to load template configuration {:?}: {reason}", path.display())]
    Deser {
        path: PathBuf,
        #[source]
        reason: toml::de::Error,
    },
    #[error("failed to define config folder: {reason}")]
    BadFolder {
        #[source]
        reason: VarError,
    },
}

impl ConfigError {
    pub fn open(path: impl AsRef<Path>, reason: std::io::Error) -> Self {
        Self::Open { path: path.as_ref().to_path_buf(), reason }
    }

    pub fn deser(path: impl AsRef<Path>, reason: toml::de::Error) -> Self {
        Self::Deser { path: path.as_ref().to_path_buf(), reason }
    }
}

impl From<VarError> for ConfigError {
    fn from(value: VarError) -> Self {
        Self::BadFolder { reason: value }
    }
}

#[derive(Debug, Error)]
#[error("failed to get env variable {variable:?}: {reason}")]
pub struct VarError {
    pub variable: String,
    #[source]
    pub reason: std::env::VarError,
}

impl VarError {
    pub fn new(variable: impl Into<String>, reason: std::env::VarError) -> Self {
        Self { variable: variable.into(), reason }
    }
}

#[derive(Debug, Error)]
#[error("failed to open decoder at {:?}:\n{reason}", path.display())]
pub struct DecoderOpenError {
    path: PathBuf,
    #[source]
    reason: std::io::Error,
}

impl DecoderOpenError {
    pub fn new(path: impl AsRef<Path>, reason: std::io::Error) -> Self {
        Self { path: path.as_ref().to_path_buf(), reason }
    }
}

#[derive(Debug, Error)]
#[error("failed to spawn decoder: {reason}")]
pub struct DecoderCreateError {
    #[source]
    reason: mlua::Error,
}

impl From<mlua::Error> for DecoderCreateError {
    fn from(reason: mlua::Error) -> Self {
        Self { reason }
    }
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Config(#[source] ConfigError),
    #[error("failed to configure template: {0}")]
    Template(#[source] DynTemplateError),
    #[error("failed to parse predicate: {0}")]
    Predicate(#[source] PredicateParseError),
    #[error("{0}")]
    Io(#[source] std::io::Error),
    #[error("{0}")]
    Runtime(#[source] RuntimeError),
}

impl From<ConfigError> for CliError {
    fn from(e: ConfigError) -> Self {
        Self::Config(e)
    }
}

impl From<DynTemplateError> for CliError {
    fn from(e: DynTemplateError) -> Self {
        Self::Template(e)
    }
}

impl From<PredicateParseError> for CliError {
    fn from(e: PredicateParseError) -> Self {
        Self::Predicate(e)
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<RuntimeError> for CliError {
    fn from(e: RuntimeError) -> Self {
        Self::Runtime(e)
    }
}
