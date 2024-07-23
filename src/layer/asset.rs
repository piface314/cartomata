//! Represents an image layer loaded from the template assets

use crate::error::Result;
use crate::image::{ImgBackend, Stroke};
use crate::layer::Layer;
use crate::template::Template;

use cairo::Context;
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
    pub x: f64,
    pub y: f64,
    pub w: Option<f64>,
    pub h: Option<f64>,
    pub r: Option<f64>,
    pub ox: Option<f64>,
    pub oy: Option<f64>,
    pub stroke: Option<Stroke>,
}

impl Layer for AssetLayer {
    fn render(&self, cr: &Context, ib: &ImgBackend, template: &Template) -> Result<()> {
        let mut path = template.assets_folder()?;
        path.push(&self.path);
        let mut img = ib.load_image(path)?;
        let (tw, th, sx, sy) = ib.target_size(&img, self.w, self.h);
        ib.paint(
            cr,
            &mut img,
            self.x,
            self.y,
            sx,
            sy,
            tw * self.ox.unwrap_or(0.0),
            th * self.oy.unwrap_or(0.0),
            self.r.unwrap_or(0.0),
            self.stroke,
        )?;
        Ok(())
    }
}
