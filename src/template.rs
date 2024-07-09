//! Template definitions.

use crate::color::Color;
use crate::data::source::csv::CsvSourceConfig;
use crate::data::source::sqlite::SqliteSourceConfig;
use crate::data::source::DataSourceType;
use crate::error::{Error, Result};

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Template {
    #[serde(rename = "template")]
    pub base: Base,
    pub assets: Option<AssetsConfig>,
    pub artwork: Option<ArtworkConfig>,
    pub fonts: HashMap<String, FontConfig>,
    pub source: DataSourceConfig,
}

#[derive(Debug, Deserialize)]
pub struct Base {
    pub name: String,
    pub size: CardSize,
    #[serde(default = "Color::default")]
    pub background: Color,
    #[serde(default = "default_extensions")]
    pub ext: Vec<String>,
}

fn default_extensions() -> Vec<String> {
    vec![
        String::from("png"),
        String::from("jpg"),
        String::from("jpeg"),
    ]
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
    pub path: Option<PathBuf>,
    pub family: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DataSourceConfig {
    pub default: Option<DataSourceType>,
    pub sqlite: Option<SqliteSourceConfig>,
    pub csv: Option<CsvSourceConfig>,
}

impl Template {
    pub fn find(name: impl AsRef<str>) -> Result<Self> {
        let mut path = Self::config_folder()?;
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
    fn config_folder() -> Result<PathBuf> {
        let home = std::env::var("APPDATA").map_err(|_| Error::MissingVariable("APPDATA"))?;
        let mut home = PathBuf::from(home);
        home.push("cartomata");
        Ok(home)
    }

    #[cfg(target_os = "linux")]
    fn config_folder() -> Result<PathBuf> {
        let home = std::env::var("HOME").map_err(|_| Error::MissingVariable("HOME"))?;
        let mut home = PathBuf::from(home);
        home.push(".config");
        home.push("cartomata");
        Ok(home)
    }

    pub fn folder(&self) -> Result<PathBuf> {
        let mut path = Self::config_folder()?;
        path.push(&self.base.name);
        Ok(path)
    }

    pub fn assets_folder(&self) -> Result<PathBuf> {
        let mut path = self.folder()?;
        match self.assets.as_ref().and_then(|a| a.path.as_ref()) {
            Some(p) => path.push(p),
            None => path.push("assets"),
        }
        Ok(path)
    }

    pub fn artwork_folder(&self) -> PathBuf {
        self.artwork
            .as_ref()
            .map(|cfg| cfg.path.clone())
            .unwrap_or_else(|| PathBuf::from("artwork"))
    }
}
