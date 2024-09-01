//! Represents an image layer loaded from artwork folder.

use crate::error::Result;
use crate::image::{BlendMode, FitMode, Origin, Stroke};
use crate::layer::{Layer, RenderContext};

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
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

fn default_origin() -> Origin {
    Origin::Relative(0.5)
}

impl Layer for ArtworkLayer {
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage> {
        let img_map = ctx.img_map;
        let ib = ctx.backend;
        let path = img_map.artwork_path(&self.id)?;
        let artwork = ib.open(path.to_string_lossy())?;
        let artwork = ib.scale_to_fit(&artwork, self.w, self.h, self.fit)?;
        let artwork = if let Some(stroke) = self.stroke {
            ib.stroke(&artwork, stroke)?
        } else {
            artwork
        };
        let (artwork, dx, dy) = ib.rotate(&artwork, self.r, self.ox, self.oy)?;
        let ox = Origin::Absolute(-self.ox.apply(self.w));
        let oy = Origin::Absolute(-self.oy.apply(self.h));
        ib.overlay(
            &img,
            &artwork,
            self.x - dx as i32,
            self.y - dy as i32,
            ox,
            oy,
            self.blend,
        )
    }
}
