//! Implements abstract layers that can be rendered to images.

mod artwork;
mod asset;
mod label;
mod text;

use crate::error::ImgError;
use crate::image::{ImageMap, ImgBackend};
use crate::text::FontMap;
pub use artwork::ArtworkLayer;
pub use asset::AssetLayer;
use core::fmt::Debug;
pub use label::LabelLayer;
use libvips::VipsImage;
pub use text::TextLayer;

#[derive(Clone)]
pub struct RenderContext<'a> {
    pub backend: &'a ImgBackend,
    pub font_map: &'a FontMap,
    pub img_map: &'a ImageMap,
}
pub trait Layer: Debug {
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage, ImgError>;
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let bg = ctx.img_map.background;
        let (w, h) = ctx.img_map.card_size;

        let mut img = ctx.backend.create(&bg, w, h)?;

        let LayerStack(layers) = self;
        for layer in layers.into_iter() {
            img = layer.render(img, ctx)?;
        }
        Ok(img)
    }
}
