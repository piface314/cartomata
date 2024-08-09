//! Represents an image layer loaded from artwork folder.

use crate::error::Result;
use crate::image::{BlendMode, FitMode, Origin, Stroke};
use crate::layer::{Layer, RenderContext};

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize, Serialize, LuaLayer))]
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
    fn render(&self, img: VipsImage, ctx: &mut RenderContext) -> Result<VipsImage> {
        let path = ctx.img_map.artwork_path(&self.id)?;
        let artwork = ctx.backend.open(path.to_string_lossy())?;
        let artwork = ctx
            .backend
            .scale_to_fit(&artwork, self.w, self.h, self.fit)?;
        let artwork = if let Some(stroke) = self.stroke {
            ctx.backend.stroke(&artwork, stroke)?
        } else {
            artwork
        };
        let (artwork, dx, dy) = ctx.backend.rotate(&artwork, self.r, self.ox, self.oy)?;
        let ox = Origin::Absolute(-self.ox.apply(self.w));
        let oy = Origin::Absolute(-self.oy.apply(self.h));
        ctx.backend.overlay(
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
