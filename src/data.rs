//! Controls how to extract and represent card data.

pub mod card;
pub mod source;

pub use card::{Card, DynCard};
pub use source::{DataSource, DataSourceType};
