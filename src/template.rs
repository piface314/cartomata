use std::error::Error;

use crate::data::{Card, DataSource};
use crate::decode::Decoder;
use crate::image::{ImageMap, ImgBackend};
use crate::text::FontMap;

use libvips::VipsImage;

pub trait Template<C: Card> {
    type SourceKey;
    type Decoder: Decoder<C>;

    fn name(&self) -> Option<&str> {
        None
    }

    fn source(&self, key: Self::SourceKey) -> Result<Box<dyn DataSource<C>>, impl Error>;
    fn identify(&self, card: &C) -> String;
    fn decoder(&self) -> Result<Self::Decoder, impl Error>;
    fn resources(&self) -> &ImageMap;
    fn fonts(&self) -> &FontMap;
    fn output(&self, card: &C, img: &VipsImage, ib: &ImgBackend) -> Result<(), impl Error>;
}
