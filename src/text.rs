//! Implements text markup parsing and related utility functions.

pub mod attr;
mod font;
mod markup;
mod parser;

pub use font::{FontMap, FontPath};
pub use markup::Markup;
pub use parser::{escape, unescape};
