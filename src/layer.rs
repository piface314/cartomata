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
use crate::image::ImgBackend;
use crate::template::Template;
use crate::text::FontMap;

use libvips::VipsImage;
use core::fmt::Debug;

pub struct LayerContext<'a> {
    pub backend: &'a mut ImgBackend,
    pub font_map: &'a FontMap<'a>,
    pub template: &'a Template,
}

pub trait Layer: Debug {
    fn render(&self, img: VipsImage, ctx: &mut LayerContext) -> Result<VipsImage>;
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, ctx: &mut LayerContext) -> Result<VipsImage> {
        let bg = ctx.template.base.background;
        let size = &ctx.template.base.size;

        let mut img = ctx.backend.new_canvas(&bg, size.width, size.height)?;

        let LayerStack(layers) = self;
        for layer in layers.into_iter() {
            img = layer.render(img, ctx)?;
        }
        Ok(img)
    }
}
