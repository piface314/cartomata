//! Implements text markup parsing and related utility functions.

pub mod attr;
mod font;
mod markup;
mod parser;

pub use font::FontManager;
pub use markup::Markup;
pub use parser::{TextParser, escape, unescape};
