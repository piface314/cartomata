//! Template definitions.

use crate::data::CardSchema;
use crate::data::source::DataSourceType;
use crate::data::source::csv::CsvSourceConfig;
use crate::data::source::sqlite::SqliteSourceConfig;

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Template {
    pub template: Base,
    pub assets: Option<AssetsConfig>,
    pub artwork: Option<ArtworkConfig>,
    pub fonts: HashMap<String, FontConfig>,
    pub schema: CardSchema,
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
    pub sqlite: SqliteSourceConfig,
    pub csv: CsvSourceConfig,
}
