//! Represents an image layer loaded from the template assets

use crate::error::Result;
use crate::image::{BlendMode, FitMode, ImgBackend, Origin, Stroke};
use crate::layer::Layer;
use crate::template::Template;

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
    pub ox: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub oy: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub origin: Origin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub fit: FitMode,
    #[cfg_attr(feature = "cli", serde(default))]
    pub blend: BlendMode,
    pub stroke: Option<Stroke>,
}

impl Layer for AssetLayer {
    fn render(&self, img: VipsImage, ib: &ImgBackend, template: &Template) -> Result<VipsImage> {
        let mut path = template.assets_folder()?;
        path.push(&self.path);
        let asset = ib.load_image(path)?;
        let asset = ib.scale_to(&asset, self.w, self.h)?;
        let asset = if let Some(stroke) = self.stroke {
            ib.stroke(&asset, stroke)?
        } else {
            asset
        };
        let (asset, ox, oy) = ib.rotate(&asset, self.r, self.ox, self.oy, self.origin)?;
        ib.overlay(&img, &asset, self.x, self.y, ox, oy, Origin::Absolute, self.blend)
    }
}
