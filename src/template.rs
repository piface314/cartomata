//! Template definitions.

use crate::data::source::csv::CsvSourceConfig;
use crate::data::source::sqlite::SqliteSourceConfig;
use crate::data::source::DataSourceType;
use crate::data::Schema;
use crate::error::{Error, Result};

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Template {
    pub template: Base,
    pub assets: Option<AssetsConfig>,
    pub artwork: Option<ArtworkConfig>,
    pub fonts: HashMap<String, FontConfig>,
    pub schema: Schema,
    pub source: DataSourceConfig,
}

#[derive(Debug, Deserialize)]
pub struct Base {
    pub name: String,
    pub size: CardSize,
}

#[derive(Debug, Deserialize)]
pub struct CardSize {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Deserialize)]
pub struct AssetsConfig {
    pub path: Option<PathBuf>,
    pub cover: Option<PathBuf>,
    pub placeholder: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct ArtworkConfig {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct FontConfig {
    pub path: PathBuf,
    pub size: f32,
}

#[derive(Debug, Deserialize)]
pub struct DataSourceConfig {
    pub default: Option<DataSourceType>,
    pub sqlite: Option<SqliteSourceConfig>,
    pub csv: Option<CsvSourceConfig>,
}

impl Template {
    pub fn find(name: impl AsRef<str>) -> Result<Self> {
        let mut path = Self::folder()?;
        path.push(name.as_ref());
        path.push("template.toml");
        Self::open(&path)
    }

    pub fn open(path: &impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| Error::FailedOpenTemplate(path.display().to_string(), e.to_string()))?;
        let template: Template = toml::from_str(&content)
            .map_err(|e| Error::FailedOpenTemplate(path.display().to_string(), e.to_string()))?;
        Ok(template)
    }

    #[cfg(target_os = "windows")]
    fn folder() -> Result<PathBuf> {
        let home = std::env::var("APPDATA").map_err(|_| Error::MissingVariable("APPDATA"))?;
        let mut home = PathBuf::from(home);
        home.push("cartomata");
        Ok(home)
    }

    #[cfg(target_os = "linux")]
    fn folder() -> Result<PathBuf> {
        let home = std::env::var("HOME").map_err(|_| Error::MissingVariable("HOME"))?;
        let mut home = PathBuf::from(home);
        home.push(".config");
        home.push("cartomata");
        Ok(home)
    }
}
