//! Represents an image layer loaded from the template assets

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
#[cfg_attr(feature = "cli", derive(LuaLayer))]
#[cfg_attr(feature = "cli", derive(Deserialize))]
pub struct AssetLayer {
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub w: Option<i32>,
    pub h: Option<i32>,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub a: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub ox: Origin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub oy: Origin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub fit: FitMode,
    #[cfg_attr(feature = "cli", serde(default))]
    pub blend: BlendMode,
    pub stroke: Option<Stroke>,
}

impl AssetLayer {
    pub fn new(path: impl Into<String>, x: impl Into<i32>, y: impl Into<i32>) -> Self {
        Self {
            path: path.into(),
            x: x.into(),
            y: y.into(),
            w: Default::default(),
            h: Default::default(),
            r: Default::default(),
            a: 1.0,
            ox: Default::default(),
            oy: Default::default(),
            fit: Default::default(),
            blend: Default::default(),
            stroke: Default::default(),
        }
    }

    #[must_use]
    pub fn with_width(mut self, w: impl Into<i32>) -> Self {
        self.w = Some(w.into());
        self
    }

    #[must_use]
    pub fn with_height(mut self, h: impl Into<i32>) -> Self {
        self.h = Some(h.into());
        self
    }

    #[must_use]
    pub fn with_rotation(mut self, r: impl Into<f64>) -> Self {
        self.r = r.into();
        self
    }

    #[must_use]
    pub fn with_opacity(mut self, a: impl Into<f64>) -> Self {
        self.a = a.into().clamp(0.0, 1.0);
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

impl Layer for AssetLayer {
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let ib = ctx.backend;
        let img_map = ctx.img_map;

        let path = img_map.asset_path(&self.path);
        let asset = ib.open(&path.to_string_lossy())?;
        let asset = ib.scale_to(&asset, self.w, self.h)?;
        let asset = if let Some(stroke) = self.stroke {
            ib.stroke(&asset, stroke)?
        } else {
            asset
        };
        let (asset, ox, oy) = ib.rotate(&asset, self.r, self.ox, self.oy)?;
        let (ox, oy) = (Origin::Absolute(ox), Origin::Absolute(oy));
        let asset = ib.set_opacity(&asset, self.a)?;
        ib.overlay(&img, &asset, self.x, self.y, ox, oy, self.blend)
    }
}
