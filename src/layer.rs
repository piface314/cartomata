//! Implements abstract layers that can be rendered to images.

pub mod artwork;
pub mod asset;

pub use artwork::ArtworkLayer;
pub use asset::AssetLayer;

use crate::error::Result;
use crate::template::{HexRgba, Template};

use core::fmt::Debug;
use ril::{Image, Rgba, OverlayMode};

pub trait Layer: Debug {
    fn render(&self, template: &Template, target: &mut Image<Rgba>) -> Result<()>;
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, template: &Template) -> Result<Image<Rgba>> {
        let HexRgba(color) = &template.base.background;
        let size = &template.base.size;
        let mut img = Image::new(size.width as u32, size.height as u32, *color)
            .with_overlay_mode(OverlayMode::Merge);
        let LayerStack(layers) = self;
        for layer in layers.into_iter() {
            layer.render(template, &mut img)?;
        }
        Ok(img)
    }
}
