//! Configuration for dynamic templates.

#[cfg(feature = "csv")]
use crate::data::source::CsvSourceConfig;
#[cfg(feature = "sqlite")]
use crate::data::source::SqliteSourceConfig;
use crate::error::{Error, Result};
use crate::image::Color;
use crate::text::FontPath;

use serde::{
    de::{self, Deserializer, Visitor},
    Deserialize,
};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(rename = "template")]
    pub base: Base,
    pub assets: Option<AssetsConfig>,
    pub artwork: Option<ArtworkConfig>,
    pub font: HashMap<String, FontPath>,
    pub source: DataSourceConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Base {
    pub name: String,
    pub size: CardSize,
    #[serde(default)]
    pub background: Color,
    #[serde(default = "default_extensions")]
    pub ext: Vec<String>,
    #[serde(default = "default_identity")]
    pub identity: String,
}

fn default_extensions() -> Vec<String> {
    vec![
        String::from("png"),
        String::from("jpg"),
        String::from("jpeg"),
    ]
}

fn default_identity() -> String {
    String::from("{id}")
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CardSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetsConfig {
    pub path: Option<PathBuf>,
    pub placeholder: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtworkConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataSourceConfig {
    pub sqlite: Option<SqliteSourceConfig>,
    pub csv: Option<CsvSourceConfig>,
}

impl Config {
    pub fn find(name: Option<&impl AsRef<str>>) -> Result<(PathBuf, Self)> {
        let path = match name {
            Some(name) => {
                let mut path = Self::config_folder()?;
                path.push(name.as_ref());
                path.push("template.toml");
                path
            }
            None => PathBuf::from("./template.toml"),
        };
        Self::open(&path)
    }

    pub fn open(path: &impl AsRef<Path>) -> Result<(PathBuf, Self)> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| Error::config_open(path, e))?;
        let raw: Self = toml::from_str(&content)
            .map_err(|e| Error::config_deser(path, e))?;
        let folder = path
            .parent()
            .expect("toml file is inside some folder")
            .to_path_buf();
        let fonts = raw
            .font
            .into_iter()
            .map(|(k, v)| (k, Self::prefix_font_path(&folder, v)))
            .collect();
        Ok((
            folder,
            Self {
                base: raw.base,
                assets: raw.assets,
                artwork: raw.artwork,
                font: fonts,
                source: raw.source,
            },
        ))
    }

    #[cfg(target_os = "windows")]
    fn config_folder() -> Result<PathBuf> {
        let home = std::env::var("APPDATA").map_err(|_| Error::no_env_variable("APPDATA"))?;
        let mut home = PathBuf::from(home);
        home.push("cartomata");
        Ok(home)
    }

    #[cfg(target_os = "linux")]
    fn config_folder() -> Result<PathBuf> {
        let home = std::env::var("HOME").map_err(|_| Error::no_env_variable("HOME"))?;
        let mut home = PathBuf::from(home);
        home.push(".cartomata");
        Ok(home)
    }

    fn prefix_font_path(folder: &PathBuf, fp: FontPath) -> FontPath {
        match fp {
            FontPath::Desc { .. } => fp,
            FontPath::Path(path) => {
                let mut new_path = folder.clone();
                new_path.push(&path);
                FontPath::Path(new_path)
            }
        }
    }

    pub fn assets_folder(&self, folder: &PathBuf) -> PathBuf {
        let mut path = folder.clone();
        match self.assets.as_ref().and_then(|a| a.path.as_ref()) {
            Some(p) => path.push(p),
            None => path.push("assets"),
        }
        path
    }
}

struct FontPathVisitor;

impl<'de> Visitor<'de> for FontPathVisitor {
    type Value = FontPath;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with either `path` or `name` set")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut name: Option<String> = None;
        let mut style: Option<String> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "path" => {
                    let path = map.next_value::<PathBuf>()?;
                    return Ok(FontPath::Path(path));
                }
                "name" => {
                    name = Some(map.next_value::<String>()?);
                }
                "style" => {
                    style = Some(map.next_value::<String>()?);
                }
                _ => {
                    return Err(de::Error::unknown_field(
                        key.as_str(),
                        &["path", "name", "style"],
                    ))
                }
            }
        }
        if let Some(name) = name {
            Ok(FontPath::Desc { name, style })
        } else {
            Err(de::Error::missing_field("name"))
        }
    }
}

impl<'de> Deserialize<'de> for FontPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<FontPath, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FontPathVisitor)
    }
}
