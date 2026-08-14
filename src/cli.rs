//! CLI implementation.
mod card;
mod config;
mod decode;
mod error;
mod output;
mod template;

pub use crate::cli::card::DynCard;
use crate::cli::config::Config;
use crate::cli::error::CliError;
use crate::cli::output::Resize;
use crate::cli::template::{DynTemplate, SourceType};
use crate::data::Predicate;
use crate::error::RuntimeError;
use crate::logs;
use crate::pipeline::Chain;
use crate::pipeline::Visitor;
use crate::pipeline::{LogVisitor, ParallelismOptions, Pipeline};
use clap::Parser;
use std::num::NonZero;
use std::path::PathBuf;
use std::thread::JoinHandle;

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

    /// Maximum number of cards to be read at a time
    #[arg(long)]
    pub batch: Option<NonZero<usize>>,

    #[cfg(feature = "diff")]
    /// If set, processes all cards, not only the ones that changed.
    #[arg(short, long)]
    pub all: bool,
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

        Self::run_internal().unwrap_or_else(|e| {
            panic!(
                "{}[ERROR]{} {e}",
                logs::ERR_COLOR.fg_str(),
                termion::style::Reset
            )
        })
    }

    pub fn run_internal() -> std::result::Result<(), CliError> {
        let cli = Self::parse();
        let (folder, config) = Config::find(cli.template.as_ref())?;

        let mut template = DynTemplate::from_config(config, folder)?;
        template.configure_output(cli.output.clone(), cli.resize, cli.ext);

        let filter = if let Some(f) = cli.filter.as_ref() {
            Some(Predicate::from_string(f)?)
        } else {
            None
        };

        let source_key = (cli.source, cli.input);
        let v_handle = if cli.workers.get() > 1 {
            let opt = ParallelismOptions::new(cli.workers).with_batch_size(cli.batch);
            let (visitor, handle) =
                Self::create_visitor(&source_key, &cli.output, opt.n_workers())?;
            let pipeline = Pipeline::new(template, visitor);
            pipeline.run_parallel(source_key, filter, opt)?.join()?;
            handle
        } else {
            let (visitor, handle) = Self::create_visitor(&source_key, &cli.output, 0)?;
            let pipeline = Pipeline::new(template, visitor);
            pipeline.run(source_key, filter);
            handle
        };
        v_handle
            .join()
            .map_err(|_| RuntimeError::thread_join(0))??;
        Ok(())
    }

    #[cfg(not(feature = "diff"))]
    fn create_visitor(
        _source_key: &(Option<SourceType>, PathBuf),
        _output: &PathBuf,
        n_workers: usize,
    ) -> std::io::Result<(
        impl Visitor<DynCard, DynTemplate> + Clone,
        JoinHandle<std::io::Result<()>>,
    )> {
        Ok(LogVisitor::new(n_workers))
    }

    #[cfg(feature = "diff")]
    fn create_visitor(
        source_key: &(Option<SourceType>, PathBuf),
        output: &Option<PathBuf>,
        n_workers: usize,
    ) -> std::io::Result<(
        impl Visitor<DynCard, DynTemplate> + Clone,
        JoinHandle<std::io::Result<()>>,
    )> {
        use crate::diff::DiffVisitor;
        use md5::{Digest, Md5};

        let mut output = output.clone().unwrap_or_else(|| PathBuf::from("."));

        let diff_name = {
            let mut hasher = Md5::new();
            let source_key = (source_key.0.clone(), source_key.1.canonicalize()?);
            let output = output.canonicalize()?;
            hasher.update(format!("{source_key:?}:{output:?}").as_bytes());
            let digest = hasher.finalize();
            let mut hex_digest = String::new();
            for b in digest.into_iter() {
                hex_digest.push_str(&format!("{b:x}"));
            }
            hex_digest
        };
        output.push(".diff");
        std::fs::create_dir_all(&output)?;
        output.push(diff_name);
        let (log_visitor, handle) = LogVisitor::new(n_workers);
        let diff_visitor = DiffVisitor::new(Some(log_visitor.tx()), output);
        let visitor = log_visitor.chain(diff_visitor);
        Ok((visitor, handle))
    }
}
