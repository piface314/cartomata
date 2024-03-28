//! CLI implementation.

use crate::data::source::DataSourceType;

use clap::Parser;
use std::path::PathBuf;

/// Render card images automatically from code defined templates
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Template name, corresponding to a folder in ~/.config/cartomata
    pub template: String,

    /// Data source type
    #[arg(short, long, value_enum)]
    pub source: Option<DataSourceType>,

    /// Input data path
    #[arg(short, long)]
    pub input: String,

    /// Output images path
    #[arg(short, long)]
    pub output: PathBuf,

    /// Optional ids to specify cards to be rendered
    #[arg(long)]
    pub ids: Vec<String>,

    /// If set, cards without artwork will be rendered with a placeholder
    #[arg(long)]
    pub placeholder: bool
}
