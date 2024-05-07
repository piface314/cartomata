//! Represents an image layer loaded from the template assets

use crate::error::{Error, Result};
use crate::layer::Layer;
use crate::template::Template;

use cartomata_derive::LuaLayer;
use mlua::LuaSerdeExt;
use ril::{Image, Rgba, OverlayMode, ResizeAlgorithm};
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, LuaLayer)]
pub struct AssetLayer {
    pub path: String,
    pub x: u32,
    pub y: u32,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

impl AssetLayer {
    fn resized(&self, img: Image<Rgba>) -> Image<Rgba> {
        let ref_size = img.dimensions();
        match (self.w, self.h) {
            (Some(w), Some(h)) => img.resized(w, h, ResizeAlgorithm::Bicubic),
            (Some(w), None) => {
                let s = w as f64 / ref_size.0 as f64;
                img.resized(w, (ref_size.1 as f64 * s) as u32, ResizeAlgorithm::Bicubic)
            }
            (None, Some(h)) => {
                let s = h as f64 / ref_size.1 as f64;
                img.resized((ref_size.0 as f64 * s) as u32, h, ResizeAlgorithm::Bicubic)
            }
            (None, None) => img,
        }
    }
}

impl Layer for AssetLayer {
    fn render(&self, template: &Template, target: &mut Image<Rgba>) -> Result<()> {
        let mut path = template.assets_folder()?;
        path.push(&self.path);
        let reader = fs::File::open(path)
            .map_err(|e| Error::FailedOpenImage(self.path.clone(), e.to_string()))?;
        let img = Image::<Rgba>::from_reader_inferred(reader)
            .map_err(|e| Error::FailedOpenImage(self.path.clone(), e.to_string()))?
            .with_overlay_mode(OverlayMode::Merge);
        target.paste(self.x as i64, self.y as i64, &self.resized(img));
        Ok(())
    }
}
