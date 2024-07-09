//! Represents an image layer loaded from the template assets

use crate::error::{Error, Result};
use crate::layer::Layer;
use crate::template::Template;

use cairo::{Context, ImageSurface};
use cartomata_derive::LuaLayer;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, LuaLayer)]
pub struct AssetLayer {
    pub path: String,
    pub x: i64,
    pub y: i64,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

impl AssetLayer {
    fn target_scale(&self, img: &ImageSurface) -> (f64, f64) {
        let (w, h) = (img.width() as f64, img.height() as f64);
        match (self.w, self.h) {
            (Some(rw), Some(rh)) => {
                let (rw, rh) = (rw as f64, rh as f64);
                (rw / w, rh / h)
            }
            (Some(rw), None) => {
                let s = rw as f64 / w;
                (s, s)
            }
            (None, Some(rh)) => {
                let s = rh as f64 / h;
                (s, s)
            }
            (None, None) => (1.0, 1.0),
        }
    }
}

impl Layer for AssetLayer {
    fn render(&self, template: &Template, cr: &Context) -> Result<()> {
        let mut path = template.assets_folder()?;
        path.push(&self.path);
        let mut reader = fs::File::open(&path)
            .map_err(|e| Error::FailedOpenImage(self.path.clone(), e.to_string()))?;
        let img = ImageSurface::create_from_png(&mut reader)
            .map_err(|e| Error::FailedOpenImage(self.path.clone(), e.to_string()))?;
        let (sx, sy) = self.target_scale(&img);
        cr.save().map_err(|e| Error::CairoError(e.to_string()))?;
        cr.translate(self.x as f64, self.y as f64);
        cr.scale(sx, sy);
        cr.set_source_surface(&img, 0.0, 0.0)
            .map_err(|e| Error::CairoError(e.to_string()))?;
        cr.paint().map_err(|e| Error::CairoError(e.to_string()))?;
        cr.restore().map_err(|e| Error::CairoError(e.to_string()))?;
        Ok(())
    }
}
