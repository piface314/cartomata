//! Represents an image layer loaded from the template assets

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
pub struct AssetLayer {
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub w: Option<i32>,
    pub h: Option<i32>,
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
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage> {
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
        ib.overlay(&img, &asset, self.x, self.y, ox, oy, self.blend)
    }
}
