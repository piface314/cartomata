//! Represents an image layer loaded from the template assets

use crate::error::ImgError;
use crate::image::{FitMode, Origin};
use crate::layer::{Layer, LayerFx, LayerOut, RenderContext};

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::Deserialize;

#[derive(Debug)]
#[cfg_attr(feature = "cli", derive(Deserialize))]
#[cfg_attr(feature = "cli", derive(LuaLayer))]
#[cfg_attr(feature = "cli", cartomata_use_crate)]
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
    #[cfg_attr(feature = "cli", serde(skip))]
    effects: Vec<LayerFx>,
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
            effects: Default::default(),
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
}

impl Layer for AssetLayer {
    fn effects(&self) -> &Vec<LayerFx> {
        &self.effects
    }

    fn effects_mut(&mut self) -> &mut Vec<LayerFx> {
        &mut self.effects
    }

    fn render(&mut self, ctx: &RenderContext) -> Result<LayerOut, ImgError> {
        let ib = ctx.backend;
        let img_map = ctx.img_map;

        let path = img_map.asset_path(&self.path);
        let img = ib.open(&path.to_string_lossy())?;
        let img = ib.scale_to_checked(img, self.w, self.h)?;
        let (img, ox, oy) = ib.rotate_checked(img, self.r, self.ox, self.oy)?;
        let img = ib.set_opacity(&img, self.a)?;
        Ok(LayerOut::new(
            img,
            (self.x as f64 - ox).round() as i32,
            (self.y as f64 - oy).round() as i32,
        ))
    }
}
