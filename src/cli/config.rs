//! Configuration for dynamic templates.

use crate::cli::output::DynOutputMap;
use crate::cli::source::DynSourceMap;
#[cfg(feature = "csv")]
use crate::data::source::CsvSourceConfig;
#[cfg(feature = "sqlite")]
use crate::data::source::SqliteSourceConfig;
use crate::error::{Error, Result};
use crate::image::{Color, ImageMap};
use crate::text::{FontMap, FontPath};

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
    base: Base,
    assets: Option<AssetsConfig>,
    artwork: Option<ArtworkConfig>,
    fonts: HashMap<String, FontPath>,
    source: DataSourceConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Base {
    pub size: CardSize,
    #[serde(default)]
    pub background: Color,
    #[serde(default = "default_extensions")]
    pub ext: Vec<String>,
    #[serde(default = "default_out_pattern")]
    pub out_pattern: String,
}

fn default_extensions() -> Vec<String> {
    vec![
        String::from("png"),
        String::from("jpg"),
        String::from("jpeg"),
    ]
}

fn default_out_pattern() -> String {
    "{id}.png".to_string()
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct CardSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetsConfig {
    pub path: Option<PathBuf>,
    pub placeholder: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
struct ArtworkConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataSourceConfig {
    pub sqlite: Option<SqliteSourceConfig>,
    pub csv: Option<CsvSourceConfig>,
}

impl Config {
    pub fn find(name: impl AsRef<str>) -> Result<(PathBuf, Self)> {
        let name = name.as_ref();
        let mut path = Self::config_folder()?;
        path.push(name);
        path.push("template.toml");
        Self::open(&path)
    }

    pub fn open(path: &impl AsRef<Path>) -> Result<(PathBuf, Self)> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| Error::FailedOpenTemplate(path.to_path_buf(), e.to_string()))?;
        let raw: Self = toml::from_str(&content)
            .map_err(|e| Error::FailedOpenTemplate(path.to_path_buf(), e.to_string()))?;
        let folder = path
            .parent()
            .expect("toml file is inside some folder")
            .to_path_buf();
        let fonts = raw
            .fonts
            .into_iter()
            .map(|(k, v)| (k, Self::prefix_font_path(&folder, v)))
            .collect();
        Ok((
            folder,
            Self {
                base: raw.base,
                assets: raw.assets,
                artwork: raw.artwork,
                fonts,
                source: raw.source,
            },
        ))
    }

    pub fn maps(self, folder: &PathBuf) -> Result<(DynSourceMap, ImageMap, FontMap, DynOutputMap)> {
        let assets_folder = self.assets_folder(folder);

        let src_map = DynSourceMap {
            csv: self.source.csv,
            sqlite: self.source.sqlite,
        };

        let img_map = ImageMap {
            artwork_folder: self
                .artwork
                .map(|cfg| cfg.path)
                .unwrap_or_else(|| PathBuf::from("artwork")),
            assets_folder,
            background: self.base.background,
            extensions: self.base.ext,
            card_size: (self.base.size.width, self.base.size.height),
            placeholder: self.assets.map(|cfg| cfg.placeholder).unwrap_or_default(),
        };

        let mut font_map = FontMap::new()?;
        font_map.load(self.fonts)?;

        let out_map = DynOutputMap {
            width: None,
            height: None,
            pattern: self.base.out_pattern,
        };
        Ok((src_map, img_map, font_map, out_map))
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

    fn assets_folder(&self, folder: &PathBuf) -> PathBuf {
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
                _ => return Err(de::Error::unknown_field(key.as_str(), &["path", "name", "style"])),
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
