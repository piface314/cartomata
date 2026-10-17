//! Implements abstract layers that can be rendered to images.

mod artwork;
mod asset;
mod label;
mod text;

use crate::error::ImgError;
use crate::image::{BlendMode, Blur, ImageMap, ImgBackend, Stroke};
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

pub struct LayerOut {
    pub img: VipsImage,
    pub x: i32,
    pub y: i32,
    pub blend: BlendMode,
}

impl LayerOut {
    pub fn new(img: VipsImage, x: i32, y: i32) -> Self {
        Self { img, x, y, blend: BlendMode::Over }
    }

    pub fn with_blend_mode(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }
}

pub struct LayerFx {
    desc: String,
    f: Box<dyn Fn(&RenderContext, LayerOut) -> Result<LayerOut, ImgError>>,
}

impl LayerFx {
    pub fn new(
        desc: impl Into<String>,
        f: impl Fn(&RenderContext, LayerOut) -> Result<LayerOut, ImgError> + 'static,
    ) -> Self {
        Self { desc: desc.into(), f: Box::new(f) }
    }

    pub fn apply(&self, ctx: &RenderContext, params: LayerOut) -> Result<LayerOut, ImgError> {
        (self.f)(ctx, params)
    }
}

impl Debug for LayerFx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LayerFx({})", self.desc)
    }
}

pub trait Layer: Debug {
    fn effects(&self) -> &Vec<LayerFx>;
    fn effects_mut(&mut self) -> &mut Vec<LayerFx>;
    fn render(&mut self, ctx: &RenderContext) -> Result<LayerOut, ImgError>;
    fn render_on(&mut self, base: VipsImage, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let mut params = self.render(ctx)?;
        for fx in self.effects().iter() {
            params = fx.apply(ctx, params)?;
        }
        let LayerOut { img, x, y, blend } = params;
        ctx.backend.overlay(&base, &img, x, y, blend)
    }

    fn stroke(mut self, stroke: Stroke) -> Self
    where
        Self: Sized,
    {
        self.effects_mut().push(stroke.into());
        self
    }

    fn blend(mut self, blend: BlendMode) -> Self
    where
        Self: Sized,
    {
        self.effects_mut().push(blend.into());
        self
    }

    fn blur(mut self, sigma: f64) -> Self
    where
        Self: Sized,
    {
        self.effects_mut().push(Blur::from(sigma).into());
        self
    }
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let bg = ctx.img_map.background;
        let (w, h) = ctx.img_map.card_size;

        let mut img = ctx.backend.create(&bg, w, h)?;

        let LayerStack(layers) = self;
        for mut layer in layers.into_iter() {
            img = layer.render_on(img, ctx)?;
        }
        Ok(img)
    }

    pub fn push(&mut self, layer: impl Layer + 'a) {
        self.0.push(Box::new(layer));
    }
}
