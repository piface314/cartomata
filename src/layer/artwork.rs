//! Represents an image layer loaded from artwork folder.

use crate::error::{Error, Result};
use crate::layer::Layer;
use crate::template::Template;

use cartomata_derive::LuaLayer;
use mlua::LuaSerdeExt;
use ril::{Image, ResizeAlgorithm, Rgba};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, LuaLayer)]
pub struct ArtworkLayer {
    pub id: String,
    pub x: u32,
    pub y: u32,
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

    fn resized(&self, img: Image<Rgba>) -> (u32, u32, Image<Rgba>) {
        let (w, h) = img.dimensions();
        let (w, h) = (w as f64, h as f64);
        let aspect_ratio = w / h;
        let resize = self.resize_mode();
        let (tw, th) = match resize {
            ResizeMode::Contain | ResizeMode::Cover => {
                if (aspect_ratio < 1.0) ^ (resize == ResizeMode::Contain) {
                    let s = self.w as f64 / w;
                    (self.w, (s * h) as u32)
                } else {
                    let s = self.h as f64 / h;
                    ((s * w) as u32, self.h)
                }
            }
            ResizeMode::Stretch => (self.w, self.h),
        };
        (tw, th, img.resized(tw, th, ResizeAlgorithm::Bicubic))
    }
}

impl Layer for ArtworkLayer {
    fn render(&self, template: &Template, target: &mut Image<Rgba>) -> Result<()> {
        let path = self
            .find_file(template)
            .ok_or_else(|| Error::ArtworkNotFound(self.id.clone()))?;
        let reader = fs::File::open(&path)
            .map_err(|e| Error::FailedOpenImage(self.id.clone(), e.to_string()))?;
        let img = Image::<Rgba>::from_reader_inferred(reader)
            .map_err(|e| Error::FailedOpenImage(self.id.clone(), e.to_string()))?;
        let (tw, th, img) = self.resized(img);
        let (x, y) = ((self.w as i64 - tw as i64) / 2, (self.h as i64 - th as i64) / 2);
        target.paste(self.x as i64 + x, self.y as i64 + y, &img);
        Ok(())
    }
}
