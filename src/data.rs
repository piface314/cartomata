//! Representation, extraction and filtering of card data.

mod predicate;
pub mod source;
mod value;

pub use crate::data::predicate::Predicate;
pub use crate::data::source::DataSource;
pub use crate::data::value::Value;

#[cfg(feature = "derive")]
pub use cartomata_derive::Card;
use serde::de::DeserializeOwned;

/// Represents a single card, to mark data types to be used as input to be processed.
///
/// This trait can be derived if the `derive` feature is enabled.
///
/// # Example
/// ```
/// use cartomata::data::{Card, Value};
/// use serde::Deserialize;
///
/// #[derive(Card, Deserialize)]
/// struct MyCard {
///     id: i64,
///     name: String,
///     power: f64,
/// }
///
/// let sample = MyCard {id: 123, name: "Sample".to_string(), power: 3.14};
/// assert_eq!(sample.get("power"), Value::Float(3.14))
/// ```
pub trait Card: DeserializeOwned + 'static {
    /// Generic access to card data fields regardless of its implementation.
    fn get(&self, field: &str) -> Value;
}
