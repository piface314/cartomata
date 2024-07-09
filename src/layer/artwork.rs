//! Represents an image layer loaded from artwork folder.

use crate::error::{Error, Result};
use crate::layer::Layer;
use crate::template::Template;

use cairo::{Context, ImageSurface};
use cartomata_derive::LuaLayer;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, LuaLayer)]
pub struct ArtworkLayer {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub w: u32,
    pub h: u32,
    pub ox: Option<f64>,
    pub oy: Option<f64>,
    pub resize: Option<ResizeMode>,
}

#[derive(Debug, Copy, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResizeMode {
    Contain,
    Cover,
    Stretch,
}

impl ArtworkLayer {
    fn find_file(&self, template: &Template) -> Option<PathBuf> {
        let mut path = template.artwork_folder();
        path.push(&self.id);
        template
            .base
            .ext
            .iter()
            .filter_map(move |ext| {
                path.set_extension(ext);
                path.exists().then(|| path.clone())
            })
            .next()
    }

    fn resize_mode(&self) -> ResizeMode {
        *self.resize.as_ref().unwrap_or(&ResizeMode::Cover)
    }

    fn target_size(&self, img: &ImageSurface) -> (f64, f64, f64, f64) {
        let (w, h) = (img.width() as f64, img.height() as f64);
        let aspect_ratio = w / h;
        let resize = self.resize_mode();
        match resize {
            ResizeMode::Contain | ResizeMode::Cover => {
                if (aspect_ratio < 1.0) ^ (resize == ResizeMode::Contain) {
                    let s = self.w as f64 / w;
                    (self.w as f64, s * h, s, s)
                } else {
                    let s = self.h as f64 / h;
                    (s * w, self.h as f64, s, s)
                }
            }
            ResizeMode::Stretch => (self.w as f64, self.h as f64, 1.0, 1.0),
        }
    }
}

impl Layer for ArtworkLayer {
    fn render(&self, template: &Template, cr: &Context) -> Result<()> {
        let path = self
            .find_file(template)
            .ok_or_else(|| Error::ArtworkNotFound(self.id.clone()))?;
        let mut reader = fs::File::open(&path)
            .map_err(|e| Error::FailedOpenImage(self.id.clone(), e.to_string()))?;
        let img = ImageSurface::create_from_png(&mut reader)
            .map_err(|e| Error::FailedOpenImage(self.id.clone(), e.to_string()))?;
        let (tw, th, sx, sy) = self.target_size(&img);
        let (x, y) = (
            (self.w as f64 - tw) * self.ox.unwrap_or(0.5),
            (self.h as f64 - th) * self.oy.unwrap_or(0.5),
        );
        cr.save().map_err(|e| Error::CairoError(e.to_string()))?;
        cr.translate(self.x as f64 + x, self.y as f64 + y);
        cr.scale(sx, sy);
        cr.set_source_surface(&img, 0.0, 0.0)
            .map_err(|e| Error::CairoError(e.to_string()))?;
        cr.paint().map_err(|e| Error::CairoError(e.to_string()))?;
        cr.restore().map_err(|e| Error::CairoError(e.to_string()))?;
        Ok(())
    }
}
