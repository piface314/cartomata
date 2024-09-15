//! CLI implementation.
mod card;
mod config;
mod decode;
mod output;
mod template;

pub use crate::cli::card::DynCard;
use crate::cli::config::Config;
use crate::cli::output::Resize;
use crate::cli::template::{DynTemplate, SourceType};
use crate::data::Predicate;
use crate::pipeline::Pipeline;

use clap::Parser;
use std::num::NonZero;
use std::path::PathBuf;

/// Render card images automatically from code defined templates.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[cfg(target_os = "linux")]
    /// Template name, corresponding to a folder in ~/.cartomata,
    /// or the current folder if omitted.
    pub template: Option<String>,

    #[cfg(target_os = "windows")]
    /// Template name, corresponding to a folder in %APPDATA%/cartomata,
    /// or the current folder if omitted.
    pub template: Option<String>,

    /// Data source type.
    #[arg(short, long, value_enum)]
    pub source: Option<SourceType>,

    /// Input data path
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output images path, defaults to the current directory.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Optionally filters input data
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Optionally resizes output
    #[arg(long)]
    pub resize: Option<Resize>,

    /// Output image extension, defaults to the first extension
    /// listed in template configuration.
    #[arg(long)]
    pub ext: Option<String>,

    /// Number of worker threads
    #[arg(short, long, default_value_t = NonZero::new(4).unwrap())]
    pub workers: NonZero<usize>,
}

macro_rules! unwrap {
    ($res:expr) => {
        $res.unwrap_or_else(|e| {
            panic!(
                "{}[ERROR]{} {e}",
                termion::color::LightRed.fg_str(),
                termion::style::Reset
            )
        })
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
        let (folder, config) = unwrap!(Config::find(cli.template.as_ref()));

        let mut template = unwrap!(DynTemplate::from_config(config, folder));
        template.configure_output(cli.output, cli.resize, cli.ext);

        let filter = cli
            .filter
            .as_ref()
            .map(|f| unwrap!(Predicate::from_string(f)));

        let pipeline = Pipeline::new(template);
        let source_key = (cli.source, cli.input);
        if cli.workers.get() > 1 {
            unwrap!(pipeline.run_parallel_with_logs(cli.workers, source_key, filter));
        } else {
            unwrap!(pipeline.run_with_logs(source_key, filter));
        }
    }
}
