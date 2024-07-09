//! Implements abstract layers that can be rendered to images.

pub mod artwork;
pub mod asset;
pub mod text;

pub use artwork::ArtworkLayer;
pub use asset::AssetLayer;
pub use text::TextLayer;

use crate::error::{Error, Result};
use crate::template::Template;

use cairo::{Context, Format, ImageSurface};
use core::fmt::Debug;

pub trait Layer: Debug {
    fn render(&self, template: &Template, target: &Context) -> Result<()>;
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, template: &Template) -> Result<ImageSurface> {
        let (r, g, b, a) = template.base.background.rgba();
        let size = &template.base.size;

        let img = ImageSurface::create(Format::ARgb32, size.width as i32, size.height as i32)
            .map_err(|e| Error::CairoError(e.to_string()))?;
        let cr = Context::new(&img).map_err(|e| Error::CairoError(e.to_string()))?;
        cr.set_source_rgba(r, g, b, a);
        cr.paint().map_err(|e| Error::CairoError(e.to_string()))?;

        let LayerStack(layers) = self;
        for layer in layers.into_iter() {
            layer.render(template, &cr)?;
        }
        Ok(img)
    }
}
