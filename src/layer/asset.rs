//! Represents an image layer loaded from the template assets

use crate::error::Result;
use crate::image::{BlendMode, FitMode, Origin, Stroke};
use crate::layer::{Layer, RenderContext};

use libvips::VipsImage;
#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Serialize, Deserialize, LuaLayer))]
pub struct AssetLayer {
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub w: Option<f64>,
    pub h: Option<f64>,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
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

impl Layer for AssetLayer {
    fn render(&self, img: VipsImage, ctx: &mut RenderContext) -> Result<VipsImage> {
        let path = ctx.img_map.asset_path(&self.path);
        let key = &path.to_string_lossy();
        ctx.backend.cache(key)?;
        let asset = ctx.backend.get_cached(key)?;
        let asset = ctx.backend.scale_to(&asset, self.w, self.h)?;
        let asset = if let Some(stroke) = self.stroke {
            ctx.backend.stroke(&asset, stroke)?
        } else {
            asset
        };
        let (asset, ox, oy) = ctx.backend.rotate(&asset, self.r, self.ox, self.oy)?;
        let (ox, oy) = (Origin::Absolute(ox), Origin::Absolute(oy));
        ctx.backend.overlay(&img, &asset, self.x, self.y, ox, oy, self.blend)
    }
}
