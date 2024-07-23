//! Implementations to decode card data into layers.

#[cfg(feature = "cli")]
pub mod dynamic;

use crate::data::Card;
use crate::error::Result;
use crate::layer::LayerStack;

pub trait Decoder<C: Card> {
    fn decode(&self, card: C) -> Result<LayerStack>;
}
