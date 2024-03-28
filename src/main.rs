use cartomata::cli::Cli;
use cartomata::data::source::{DataSource, CsvSource, DataSourceType, SqliteSource};
use cartomata::template::Template;
use clap::Parser;

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
    match source_type {
        DataSourceType::Sqlite => {
            let mut source = SqliteSource::open(&template, &cli.input).unwrap_or_else(|e| panic!("{e}"));
            let cards = source.fetch_generic(&cli.ids);
            println!("{cards:?}");
        }
        DataSourceType::Csv => {
            let mut source = CsvSource::open(&template, &cli.input).unwrap_or_else(|e| panic!("{e}"));
            let cards = source.fetch_generic(&cli.ids);
            println!("{cards:?}");
        }
    };
}
