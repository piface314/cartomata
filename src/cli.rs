//! CLI implementation.
mod card;
mod config;
mod decode;
mod output;

pub use crate::cli::card::DynCard;
use crate::cli::config::Config;
pub use crate::cli::decode::LuaDecoder;
use crate::cli::output::Resize;
use crate::data::{Predicate, SourceType};
use crate::pipeline::Pipeline;

use clap::Parser;
use mlua::Lua;
use std::path::PathBuf;

/// Render card images automatically from code defined templates
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Template name, corresponding to a folder in ~/.config/cartomata,
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
        let (folder, config) = error!(Config::find(cli.template.as_ref().map(|s| s.as_str())));
        let (src_map, img_map, font_map, mut out_map) = error!(config.maps(&folder));

        let source = error!(src_map.select::<DynCard>(cli.source, cli.input));

        let lua = Lua::new();
        let decoder = error!(LuaDecoder::new(&lua, &folder));

        if let Some(resize) = cli.resize {
            out_map.resize = resize;
        }
        out_map.prefix = Some(cli.output);

        let mut pipeline = error!(Pipeline::new(
            None, source, decoder, img_map, font_map, out_map
        ));

        let filter = cli
            .filter
            .as_ref()
            .map(|f| error!(Predicate::from_string(f)));

        error!(pipeline.run(filter));
    }
}
