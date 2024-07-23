//! Represents an image layer loaded from artwork folder.

use crate::error::{Error, Result};
use crate::image::{FitMode, ImgBackend, Stroke};
use crate::layer::Layer;
use crate::template::Template;

use cairo::Context;
#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize, Serialize, LuaLayer))]
pub struct ArtworkLayer {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub r: Option<f64>,
    pub ox: Option<f64>,
    pub oy: Option<f64>,
    pub stroke: Option<Stroke>,
    pub fit: Option<FitMode>,
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

    fn fit_mode(&self) -> FitMode {
        self.fit.as_ref().copied().unwrap_or(FitMode::Cover)
    }
}

impl Layer for ArtworkLayer {
    fn render(&self, cr: &Context, ib: &ImgBackend, template: &Template) -> Result<()> {
        let path = self
            .find_file(template)
            .ok_or_else(|| Error::ArtworkNotFound(self.id.clone()))?;
        let mut img = ib.load_image(path)?;
        let (tw, th, sx, sy) = ib.target_size_to_fit(&img, self.w, self.h, self.fit_mode());
        let (ox, oy) = (self.ox.unwrap_or(0.5), self.oy.unwrap_or(0.5));
        let (dx, dy) = ((self.w - tw) * ox, (self.h - th) * oy);
        ib.paint(
            cr,
            &mut img,
            self.x + dx,
            self.y + dy,
            sx,
            sy,
            self.w * 0.5,
            self.h * 0.5,
            self.r.unwrap_or(0.0),
            self.stroke,
        )?;
        Ok(())
    }
}
