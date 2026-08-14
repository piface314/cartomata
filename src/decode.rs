//! Implementations to decode card data into layers.

use crate::data::Card;
use crate::layer::LayerStack;
use std::error::Error;

pub trait Decoder<C: Card> {
    fn decode(&self, card: &C) -> Result<LayerStack<'_>, impl Error>;
}
