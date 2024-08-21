//! CLI implementation.

mod card;
mod config;
mod decode;
mod output;
mod source;

pub use crate::cli::card::DynCard;
use crate::cli::config::Config;
pub use crate::cli::decode::LuaDecoder;
use crate::cli::output::Resize;
use crate::cli::source::SourceType;
use crate::data::source::SourceMap;
use crate::data::Predicate;
use crate::decode::Decoder;
use crate::image::{ImgBackend, OutputMap};
use crate::layer::RenderContext;

use clap::Parser;
use mlua::Lua;
use std::path::PathBuf;

/// Render card images automatically from code defined templates
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Template name, corresponding to a folder in ~/.config/cartomata
    pub template: String,

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

    /// If set, cards without artwork will be rendered with a placeholder
    #[arg(long)]
    pub placeholder: bool,
}

macro_rules! error {
    ($res:expr) => {
        $res.unwrap_or_else(|e| panic!("{e}"))
    };
}

macro_rules! warn {
    ($res:expr) => {
        match $res {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: {e}");
                continue;
            }
        }
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
        let (folder, config) = error!(Config::find(cli.template));
        let (src_map, img_map, font_map, mut out_map) = error!(config.maps(&folder));
        let mut ib = error!(ImgBackend::new());

        let mut layer_ctx = RenderContext {
            backend: &mut ib,
            font_map: &font_map,
            img_map: &img_map,
        };

        let mut source = error!(src_map.source(&(cli.source, cli.input)));
        let filter = if let Some(filter) = &cli.filter {
            Some(error!(Predicate::from_string(filter)))
        } else {
            None
        };
        if let Some(resize) = cli.resize {
            out_map.resize = resize;
        }

        let cards = source.read(filter.as_ref());
        let lua = Lua::new();
        let decoder = error!(LuaDecoder::new(&lua, &folder));
        for card_res in cards.into_iter() {
            let card = warn!(card_res);
            let out_path = out_map.path(&card);
            let stack = warn!(decoder.decode(card));
            let img = warn!(stack.render(&mut layer_ctx));

            let mut path = cli.output.clone();
            path.push(out_path);
            warn!(out_map.write(layer_ctx.backend, &img, path));
        }
    }
}
