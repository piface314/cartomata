//! Implements abstract layers that can be rendered to images.

mod artwork;
mod asset;
mod label;
mod text;

use crate::error::ImgError;
use crate::image::{BlendMode, Blur, ImageMap, ImgBackend, Stroke, Unit};
use crate::text::FontMap;
pub use artwork::ArtworkLayer;
pub use asset::AssetLayer;
use core::fmt::Debug;
pub use label::{LabelLayer, Overflow};
use libvips::VipsImage;
pub use text::TextLayer;

#[derive(Clone)]
pub struct RenderContext<'g, 'l> {
    pub backend: &'g ImgBackend,
    pub font_map: &'g FontMap,
    pub img_map: &'g ImageMap,
    pub out: Option<&'l CardOut>,
}

impl<'g, 'l> RenderContext<'g, 'l> {
    pub fn with_out(&self, out: &'l CardOut) -> Self {
        Self {
            backend: self.backend,
            font_map: self.font_map,
            img_map: self.img_map,
            out: Some(out),
        }
    }
}

impl<'g, 'l> AsRef<[LayerOut]> for RenderContext<'g, 'l> {
    #[inline]
    fn as_ref(&self) -> &[LayerOut] {
        self.out.as_ref().map(|out| out.as_ref()).unwrap_or(&[])
    }
}

pub struct CardOut {
    pub layers: Vec<LayerOut>,
    pub img: VipsImage,
}

impl AsRef<[LayerOut]> for CardOut {
    #[inline]
    fn as_ref(&self) -> &[LayerOut] {
        &self.layers
    }
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

    fn stroke(mut self, stroke: Stroke) -> Self
    where
        Self: Sized,
    {
        self.effects_mut().push(stroke.into());
        self
    }

    fn stroke_opt(self, stroke: Option<Stroke>) -> Self
    where
        Self: Sized,
    {
        if let Some(stroke) = stroke {
            self.stroke(stroke)
        } else {
            self
        }
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

    fn scale(mut self, s: f64, ox: Unit, oy: Unit) -> Self
    where
        Self: Sized,
    {
        self.effects_mut().push(LayerFx::new(
            format!("Scale {{ s: {s}, ox: {ox:?}, oy: {oy:?} }}"),
            move |ctx: &RenderContext, out: LayerOut| -> Result<LayerOut, ImgError> {
                let ib = ctx.backend;
                let LayerOut { img, x, y, blend } = out;
                let (ow, oh) = (img.get_width() as f64, img.get_height() as f64);
                let img = ib.scale_checked(img, s, s)?;
                let (tw, th) = (img.get_width() as f64, img.get_height() as f64);
                let x = x + ox.apply(ow - tw).round() as i32;
                let y = y + oy.apply(oh - th).round() as i32;
                Ok(LayerOut { img, x, y, blend })
            },
        ));
        self
    }
}

#[derive(Debug)]
pub struct LayerStack<'a>(pub Vec<Box<dyn Layer + 'a>>);

impl<'a> LayerStack<'a> {
    pub fn render(self, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let bg = ctx.img_map.background;
        let (w, h) = ctx.img_map.card_size;

        let img = ctx.backend.create(&bg, w, h)?;

        let LayerStack(layers) = self;
        let mut out = CardOut { img, layers: Vec::with_capacity(layers.len()) };
        for mut layer in layers.into_iter() {
            let ctx = ctx.with_out(&out);
            let mut params = layer.render(&ctx)?;
            for fx in layer.effects().iter() {
                params = fx.apply(&ctx, params)?;
            }
            let LayerOut { img, x, y, blend } = &params;
            out.img = ctx.backend.overlay(&out.img, img, *x, *y, *blend)?;
            out.layers.push(params);
        }
        Ok(out.img)
    }

    pub fn push(&mut self, layer: impl Layer + 'a) {
        self.0.push(Box::new(layer));
    }
}
