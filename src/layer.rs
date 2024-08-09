//! Implements abstract layers that can be rendered to images.

mod artwork;
mod asset;
mod label;
mod text;

pub use artwork::ArtworkLayer;
pub use asset::AssetLayer;
pub use label::LabelLayer;
pub use text::TextLayer;

use crate::error::Result;
use crate::image::{ImageMap, ImgBackend};
use crate::text::FontMap;

use core::fmt::Debug;
use libvips::VipsImage;

pub struct RenderContext<'a> {
    pub backend: &'a mut ImgBackend,
    pub font_map: &'a FontMap,
    pub img_map: &'a ImageMap,
}

pub trait Layer: Debug {
    fn render(&self, img: VipsImage, ctx: &mut RenderContext) -> Result<VipsImage>;
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, ctx: &mut RenderContext) -> Result<VipsImage> {
        let bg = ctx.img_map.background;
        let (w, h) = ctx.img_map.card_size;

        let mut img = ctx.backend.new_canvas(&bg, w, h)?;

        let LayerStack(layers) = self;
        for layer in layers.into_iter() {
            img = layer.render(img, ctx)?;
        }
        Ok(img)
    }
}
