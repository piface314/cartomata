//! Represents an image layer loaded from artwork folder.

use crate::error::ImgError;
use crate::image::{BlendMode, FitMode, Origin, Stroke};
use crate::layer::{Layer, RenderContext};

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::Deserialize;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize))]
#[cfg_attr(feature = "cli", derive(LuaLayer))]
pub struct ArtworkLayer {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: f64,
    pub h: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
    #[cfg_attr(feature = "cli", serde(default = "default_origin"))]
    pub ox: Origin,
    #[cfg_attr(feature = "cli", serde(default = "default_origin"))]
    pub oy: Origin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub fit: FitMode,
    #[cfg_attr(feature = "cli", serde(default))]
    pub blend: BlendMode,
    pub stroke: Option<Stroke>,
}

#[cfg(feature = "cli")]
fn default_origin() -> Origin {
    Origin::Relative(0.5)
}

impl ArtworkLayer {
    pub fn new(
        id: impl Into<String>,
        x: impl Into<i32>,
        y: impl Into<i32>,
        w: impl Into<f64>,
        h: impl Into<f64>,
    ) -> Self {
        Self {
            id: id.into(),
            x: x.into(),
            y: y.into(),
            w: w.into(),
            h: h.into(),
            r: Default::default(),
            ox: Origin::Relative(0.5),
            oy: Origin::Relative(0.5),
            fit: Default::default(),
            blend: Default::default(),
            stroke: Default::default(),
        }
    }

    #[must_use]
    pub fn with_rotation(mut self, r: impl Into<f64>) -> Self {
        self.r = r.into();
        self
    }

    #[must_use]
    pub fn with_origin(mut self, ox: Origin, oy: Origin) -> Self {
        self.ox = ox;
        self.oy = oy;
        self
    }

    #[must_use]
    pub fn with_fit_mode(mut self, fit: FitMode) -> Self {
        self.fit = fit;
        self
    }

    #[must_use]
    pub fn with_blend_mode(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }

    #[must_use]
    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

impl Layer for ArtworkLayer {
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let img_map = ctx.img_map;
        let ib = ctx.backend;
        let path = img_map
            .artwork_path(&self.id)
            .ok_or_else(|| ImgError::no_artwork(&self.id))?;
        let artwork = ib.open(path.to_string_lossy())?;
        let artwork = ib.scale_to_fit(&artwork, self.w, self.h, self.fit)?;
        let artwork = if let Some(stroke) = self.stroke {
            ib.stroke_checked(artwork, stroke)?
        } else {
            artwork
        };
        let (artwork, dx, dy) = ib.rotate_checked(artwork, self.r, self.ox, self.oy)?;
        let ox = Origin::Absolute(-self.ox.apply(self.w));
        let oy = Origin::Absolute(-self.oy.apply(self.h));
        ib.overlay(
            &img,
            &artwork,
            (self.x as f64 - dx).round() as i32,
            (self.y as f64 - dy).round() as i32,
            ox,
            oy,
            self.blend,
        )
    }
}
