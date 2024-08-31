//! CLI implementation.
mod card;
mod config;
mod decode;
mod output;

pub use crate::cli::card::DynCard;
use crate::cli::config::Config;
pub use crate::cli::decode::LuaDecoderFactory;
use crate::cli::output::Resize;
use crate::data::{DataSource, Predicate, SourceMap, SourceType};
use crate::error::Result;
use crate::pipeline::Pipeline;

use clap::Parser;
use output::DynOutputMap;
use std::num::NonZero;
use std::path::PathBuf;

/// Render card images automatically from code defined templates
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[cfg(target_os = "linux")]
    /// Template name, corresponding to a folder in ~/.config/cartomata,
    /// or the current folder if omitted.
    pub template: Option<String>,

    #[cfg(target_os = "windows")]
    /// Template name, corresponding to a folder in %APPDATA%/cartomata,
    /// or the current folder if omitted.
    pub template: Option<String>,

    /// Data source type
    #[arg(short, long, value_enum)]
    pub source: Option<SourceType>,

    /// Input data path
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output images path
    #[arg(short, long)]
    pub output: PathBuf,

    /// Optionally filters input data
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Optionally resizes output
    #[arg(long)]
    pub resize: Option<Resize>,

    /// Number of worker threads
    #[arg(short, long, default_value_t = NonZero::new(4).unwrap())]
    pub workers: NonZero<usize>,
}

macro_rules! error {
    ($res:expr) => {
        $res.unwrap_or_else(|e| panic!("{e}"))
    };
}

impl Cli {
    pub fn run() {
        std::panic::set_hook(Box::new(|panic_info| {
            if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                eprintln!("{s}");
            } else {
                eprintln!("{panic_info}");
            }
        }));

        let cli = Self::parse();
        let (folder, config) = error!(cli.find_config());
        let (src_map, img_map, font_map, mut out_map) = error!(config.maps(&folder));

        let source = error!(cli.select_source(src_map));
        let decoder_factory = error!(LuaDecoderFactory::new(folder));
        cli.configure_output(&mut out_map);

        let filter = cli
            .filter
            .as_ref()
            .map(|f| error!(Predicate::from_string(f)));

        let pipeline = error!(Pipeline::new(
            cli.workers,
            source,
            decoder_factory,
            img_map,
            font_map,
            out_map
        ));

        error!(pipeline.run(filter));
    }

    fn find_config(&self) -> Result<(PathBuf, Config)> {
        Config::find(self.template.as_ref().map(|s| s.as_str()))
    }

    fn select_source(
        &self,
        src_map: SourceMap,
    ) -> Result<Box<(dyn DataSource<DynCard> + 'static)>> {
        src_map.select::<DynCard>(self.source, &self.input)
    }

    fn configure_output(&self, out_map: &mut DynOutputMap) {
        if let Some(resize) = self.resize {
            out_map.resize = resize;
        }
        out_map.prefix = Some(self.output.clone());
    }
}
