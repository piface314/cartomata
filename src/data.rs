//! Controls how to extract and represent card data.

pub mod source;

pub use source::DataSource;

use serde::de::DeserializeOwned;

pub trait Card: DeserializeOwned {
    fn get_int(&self, field: &str) -> Option<i64>;
    fn get_float(&self, field: &str) -> Option<f64>;
    fn get_bool(&self, field: &str) -> Option<bool>;
    fn get_string(&self, field: &str) -> Option<String>;
}
