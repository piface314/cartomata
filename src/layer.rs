//! Implements abstract layers that can be rendered to images.

pub mod artwork;
pub mod asset;
pub mod text;

pub use artwork::ArtworkLayer;
pub use asset::AssetLayer;
pub use text::TextLayer;

use crate::error::Result;
use crate::image::ImgBackend;
use crate::template::Template;

use cairo::{Context, ImageSurface};
use core::fmt::Debug;

pub trait Layer: Debug {
    fn render(&self, cr: &Context, ib: &ImgBackend, template: &Template) -> Result<()>;
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, template: &Template, ib: &ImgBackend) -> Result<ImageSurface> {
        let bg = template.base.background;
        let size = &template.base.size;

        let (img, cr) = ib.new_canvas(&bg, size.width, size.height)?;

        let LayerStack(layers) = self;
        for layer in layers.into_iter() {
            layer.render(&cr, ib, template)?;
        }
        Ok(img)
    }
}
