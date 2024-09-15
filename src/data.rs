//! Controls how to extract and represent card data.

mod predicate;
pub mod source;
mod value;

pub use crate::data::predicate::Predicate;
pub use crate::data::source::DataSource;
pub use crate::data::value::Value;

use serde::de::DeserializeOwned;

pub trait Card: DeserializeOwned + 'static {
    fn get(&self, field: &str) -> Value;
}
