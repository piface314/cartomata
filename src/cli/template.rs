use std::path::PathBuf;

use crate::cli::card::DynCard;
use crate::cli::config::Config;
use crate::cli::decode::{LuaDecoder, LuaDecoderFactory};
use crate::cli::output::{OutputMap, Resize};
#[cfg(feature = "csv")]
use crate::data::source::{CsvSource, CsvSourceConfig};
#[cfg(feature = "sqlite")]
use crate::data::source::{SqliteSource, SqliteSourceConfig};
use crate::data::{Card, DataSource};
use crate::error::{Error, Result};
use crate::image::{ImageMap, ImgBackend};
use crate::template::Template;
use crate::text::FontMap;

use clap::ValueEnum;
use libvips::VipsImage;
use std::path::Path;

pub struct DynTemplate {
    source_map: SourceMap,
    decoder_factory: LuaDecoderFactory,
    resource_map: ImageMap,
    font_map: FontMap,
    output_map: OutputMap,
}

impl DynTemplate {
    pub fn from_config(config: Config, folder: PathBuf) -> Result<Self> {
        let assets_folder = config.assets_folder(&folder);

        let mut source_map = SourceMap::new();

        #[cfg(feature = "csv")]
        source_map.with_csv(config.source.csv);

        #[cfg(feature = "sqlite")]
        source_map.with_sqlite(config.source.sqlite);

        let decoder_factory = LuaDecoderFactory::new(folder)?;

        let resource_map = ImageMap {
            artwork_folder: config
                .artwork
                .map(|cfg| cfg.path)
                .unwrap_or_else(|| PathBuf::from("artwork")),
            assets_folder,
            background: config.base.background,
            extensions: config.base.ext,
            card_size: (config.base.size.width, config.base.size.height),
            placeholder: config.assets.map(|cfg| cfg.placeholder).unwrap_or_default(),
        };

        let mut font_map = FontMap::new()?;
        font_map.load(config.font)?;

        let mut output_map = OutputMap::new(config.base.identity);
        output_map.set_ext(resource_map.extensions.first().cloned());

        Ok(Self {
            source_map,
            decoder_factory,
            resource_map,
            font_map,
            output_map,
        })
    }

    pub fn configure_output(
        &mut self,
        prefix: Option<PathBuf>,
        resize: Option<Resize>,
        ext: Option<String>,
    ) {
        self.output_map.set_prefix(prefix);
        self.output_map.set_resize(resize);
        self.output_map.set_ext(ext);
    }
}

impl Template<DynCard> for DynTemplate {
    type SourceKey = (Option<SourceType>, PathBuf);
    type Decoder = LuaDecoder;

    fn source(&self, key: Self::SourceKey) -> Result<Box<dyn DataSource<DynCard>>> {
        self.source_map.select(key.0, key.1)
    }

    fn identify(&self, card: &DynCard) -> String {
        self.output_map.identify(card)
    }

    fn decoder(&self) -> Result<Self::Decoder> {
        self.decoder_factory.create()
    }

    fn resources(&self) -> &ImageMap {
        &self.resource_map
    }

    fn fonts(&self) -> &FontMap {
        &self.font_map
    }

    fn output(&self, card: &DynCard, img: &VipsImage, ib: &ImgBackend) -> Result<()> {
        self.output_map.write(card, img, ib)
    }
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum SourceType {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub struct SourceMap {
    #[cfg(feature = "csv")]
    csv: Option<CsvSourceConfig>,
    #[cfg(feature = "sqlite")]
    sqlite: Option<SqliteSourceConfig>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "csv")]
            csv: None,
            #[cfg(feature = "sqlite")]
            sqlite: None,
        }
    }

    #[cfg(feature = "csv")]
    pub fn with_csv(&mut self, cfg: Option<CsvSourceConfig>) {
        self.csv = cfg;
    }

    #[cfg(feature = "sqlite")]
    pub fn with_sqlite(&mut self, cfg: Option<SqliteSourceConfig>) {
        self.sqlite = cfg;
    }

    fn infer_source_type(path: impl AsRef<Path>) -> Option<SourceType> {
        let ext = path.as_ref().extension()?.to_str()?;
        match ext {
            #[cfg(feature = "csv")]
            "csv" | "tsv" => Some(SourceType::Csv),
            #[cfg(feature = "sqlite")]
            "db" | "cdb" => Some(SourceType::Sqlite),
            _ => None,
        }
    }

    pub fn select<C: Card>(
        &self,
        src_type: Option<SourceType>,
        path: impl AsRef<Path>,
    ) -> Result<Box<dyn DataSource<C>>> {
        let path = path.as_ref();
        let src_type = src_type
            .or_else(|| Self::infer_source_type(path))
            .ok_or_else(|| Error::source_inference(path))?;
        match src_type {
            #[cfg(feature = "csv")]
            SourceType::Csv => {
                let config = self.csv.unwrap_or_default();
                let source = CsvSource::open(config, &path)?;
                Ok(Box::new(source) as Box<dyn DataSource<C>>)
            }
            #[cfg(feature = "sqlite")]
            SourceType::Sqlite => {
                let config = self
                    .sqlite
                    .clone()
                    .ok_or_else(|| Error::no_source_config("sqlite"))?;
                let source = SqliteSource::open(config, &path)?;
                Ok(Box::new(source) as Box<dyn DataSource<C>>)
            }
        }
    }
}
