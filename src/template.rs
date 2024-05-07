//! Template definitions.

use crate::data::source::csv::CsvSourceConfig;
use crate::data::source::sqlite::SqliteSourceConfig;
use crate::data::source::DataSourceType;
use crate::data::Schema;
use crate::error::{Error, Result};

use regex::Regex;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Template {
    #[serde(rename = "template")]
    pub base: Base,
    pub schema: Schema,
    pub assets: Option<AssetsConfig>,
    pub artwork: Option<ArtworkConfig>,
    pub fonts: HashMap<String, FontConfig>,
    pub source: DataSourceConfig,
}

#[derive(Debug, Deserialize)]
pub struct Base {
    pub name: String,
    pub size: CardSize,
    pub background: HexRgba,
    #[serde(default = "default_extensions")]
    pub ext: Vec<String>
}

fn default_extensions() -> Vec<String> {
    vec![String::from("png"), String::from("jpg"), String::from("jpeg")]
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

#[derive(Debug)]
pub struct HexRgba(pub ril::pixel::Rgba);

struct HexRgbaVisitor;

impl<'de> Visitor<'de> for HexRgbaVisitor {
    type Value = HexRgba;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the form #RRGGBBAA or #RRGGBB")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        let re =
            Regex::new(r"^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})?$")
                .unwrap();

        let captures = re.captures(v).ok_or(E::custom(format!(
            "string not in form #RRGGBBAA or #RRGGBB: {v}"
        )))?;
        let mut values = captures
            .iter()
            .skip(1)
            .map(|c| c.map(|v| u8::from_str_radix(v.as_str(), 16).unwrap()));
        let r = values.next().unwrap().unwrap_or(0);
        let g = values.next().unwrap().unwrap_or(0);
        let b = values.next().unwrap().unwrap_or(0);
        let a = values.next().unwrap().unwrap_or(255);
        Ok(HexRgba(ril::pixel::Rgba { r, g, b, a }))
    }
}

impl<'de> Deserialize<'de> for HexRgba {
    fn deserialize<D>(deserializer: D) -> std::result::Result<HexRgba, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(HexRgbaVisitor)
    }
}
