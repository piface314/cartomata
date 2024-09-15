//! Implementations to decode card data into layers.


use crate::data::Card;
use crate::error::Result;
use crate::layer::LayerStack;

pub trait Decoder<C: Card> {
    fn decode(&self, card: &C) -> Result<LayerStack<'_>>;
}
