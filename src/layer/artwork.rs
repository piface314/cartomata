//! Represents an image layer loaded from artwork folder.

use crate::error::ImgError;
use crate::image::{FitMode, Unit};
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
pub struct ArtworkLayer {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: f64,
    pub h: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
    #[cfg_attr(feature = "cli", serde(default = "default_unit"))]
    pub ox: Unit,
    #[cfg_attr(feature = "cli", serde(default = "default_unit"))]
    pub oy: Unit,
    #[cfg_attr(feature = "cli", serde(default))]
    pub fit: FitMode,
    #[cfg_attr(feature = "cli", serde(skip))]
    effects: Vec<LayerFx>,
}

#[cfg(feature = "cli")]
fn default_unit() -> Unit {
    Unit::Pct(0.5)
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
            ox: Unit::Pct(0.5),
            oy: Unit::Pct(0.5),
            fit: Default::default(),
            effects: Default::default(),
        }
    }

    #[must_use]
    pub fn with_rotation(mut self, r: impl Into<f64>) -> Self {
        self.r = r.into();
        self
    }

    #[must_use]
    pub fn with_origin(mut self, ox: Unit, oy: Unit) -> Self {
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

impl Layer for ArtworkLayer {
    fn effects(&self) -> &Vec<LayerFx> {
        &self.effects
    }

    fn effects_mut(&mut self) -> &mut Vec<LayerFx> {
        &mut self.effects
    }

    fn render(&mut self, ctx: &RenderContext) -> Result<LayerOut, ImgError> {
        let img_map = ctx.img_map;
        let ib = ctx.backend;
        let path = img_map
            .artwork_path(&self.id)
            .ok_or_else(|| ImgError::no_artwork(&self.id))?;
        let artwork = ib.open(path.to_string_lossy())?;
        let artwork = ib.scale_to_fit(&artwork, self.w, self.h, self.fit)?;
        let (img, dx, dy) = ib.rotate_checked(artwork, self.r, self.ox, self.oy)?;
        let (ox, oy) = (self.ox.apply(self.w), self.oy.apply(self.h));
        Ok(LayerOut::new(
            img,
            (self.x as f64 - dx + ox).round() as i32,
            (self.y as f64 - dy + oy).round() as i32,
        ))
    }
}
