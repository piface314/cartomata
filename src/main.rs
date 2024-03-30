use cartomata::cli::Cli;
use cartomata::data::{Card, DynCard};
use cartomata::template::Template;
use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Deserialize, Card)]
pub struct SampleCard {
    pub id: u64,
    pub name: String,
    pub level: i64,
    pub element: i64,
    pub r#type: i64,
    pub power: i64,
    pub size: i64,
    pub rarity: i64,
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("{s}");
        } else {
            eprintln!("{panic_info}");
        }
    }));
    let cli = Cli::parse();
    let template = Template::find(cli.template).unwrap_or_else(|e| panic!("{e}"));
    let source_type = cli
        .source
        .or(template.source.default)
        .expect("Choose a data source");
    let mut source = source_type
        .open::<DynCard>(&template, &cli.input)
        .unwrap_or_else(|e| panic!("{e}"));
    let cards = source.fetch(&cli.ids);
    println!("{cards:?}");
}
