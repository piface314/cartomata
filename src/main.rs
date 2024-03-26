use cartomata::cli::Cli;
use cartomata::error::{Error, Result};
use cartomata::template::Template;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn template_folder() -> Result<PathBuf> {
    let home = std::env::var("APPDATA").map_err(|_| Error::MissingVariable("APPDATA"))?;
    let mut home = PathBuf::from(home);
    home.push("cartomata");
    Ok(home)
}

#[cfg(target_os = "linux")]
fn template_folder() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| Error::MissingVariable("HOME"))?;
    let mut home = PathBuf::from(home);
    home.push(".config");
    home.push("cartomata");
    Ok(home)
}

fn main() {
    let cli = Cli::parse();
    let mut template_path = template_folder().unwrap_or_else(|e| panic!("{e}"));
    template_path.push(&cli.template);
    let mut template_toml_path = template_path.clone();
    template_toml_path.push("template.toml");
    let template = fs::read_to_string(&template_toml_path).unwrap_or_else(|e| {
        panic!(
            "Failed to open {}: {e}",
            template_toml_path.to_str().unwrap_or("")
        )
    });
    let template: Template =
        toml::from_str(&template).unwrap_or_else(|e| panic!("Invalid template file: {e}"));
    println!("{:?}", template);
}
