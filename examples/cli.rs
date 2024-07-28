use cartomata::cli::Cli;
use cartomata::data::{Card, DynCard};
use cartomata::decode::dynamic::DynamicDecoder;
use cartomata::decode::Decoder;
use cartomata::image::ImgBackend;
use cartomata::template::Template;
use cartomata::text::FontManager;
use clap::Parser;
use mlua::Lua;
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
    let vips = libvips::VipsApp::new("libvips", false).expect("Cannot initialize libvips");
    let cli = Cli::parse();
    let template = Template::find(cli.template).unwrap_or_else(|e| panic!("{e}"));

    let fc = fontconfig::Fontconfig::new().unwrap();
    let mut fm = FontManager::new(&fc);
    fm.load_from_template(&template)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut ib = ImgBackend::new(vips, fm);

    let source_type = cli
        .source
        .or(template.source.default)
        .expect("Choose a data source");
    let mut source = source_type
        .open::<DynCard>(&template, &cli.input)
        .unwrap_or_else(|e| panic!("{e}"));
    let cards = source.fetch(&cli.ids);
    let lua = Lua::new();
    let decoder = DynamicDecoder::new(&lua, &template).unwrap_or_else(|e| panic!("{e}"));
    for card_res in cards.into_iter() {
        let card = match card_res {
            Ok(card) => card,
            Err(e) => {
                eprintln!("Warning: {e}");
                continue;
            }
        };
        let id = card.id();
        let stack = match decoder.decode(card) {
            Ok(stack) => stack,
            Err(e) => {
                eprintln!("Warning: {e}");
                continue;
            }
        };
        let image_res = stack.render(&template, &mut ib);
        match image_res {
            Ok(image) => {
                let mut path = cli.output.clone();
                path.push(format!("{id}.png"));
                image
                    .image_write_to_file(&path.to_string_lossy())
                    .unwrap_or_else(|e| eprintln!("Warning: {e}"));
            }
            Err(e) => eprintln!("Warning: {e}"),
        }
    }
}
