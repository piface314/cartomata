//! Represents an image layer loaded from artwork folder.

use crate::error::{Error, Result};
use crate::image::{BlendMode, FitMode, ImgBackend, Origin, Stroke};
use crate::layer::Layer;
use crate::template::Template;

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub ox: f64,
    #[cfg_attr(feature = "cli", serde(default = "default_origin"))]
    pub oy: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub origin: Origin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub fit: FitMode,
    #[cfg_attr(feature = "cli", serde(default))]
    pub blend: BlendMode,
    pub stroke: Option<Stroke>,
}

fn default_origin() -> f64 {
    0.5
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
}

impl Layer for ArtworkLayer {
    fn render(&self, img: VipsImage, ib: &ImgBackend, template: &Template) -> Result<VipsImage> {
        let path = self
            .find_file(template)
            .ok_or_else(|| Error::ArtworkNotFound(self.id.clone()))?;
        let artwork = ib.load_image(path)?;
        let artwork = ib.scale_to_fit(&artwork, self.w, self.h, self.fit)?;
        let artwork = if let Some(stroke) = self.stroke {
            ib.stroke(&artwork, stroke)?
        } else {
            artwork
        };
        let (artwork, dx, dy) = ib.rotate(&artwork, self.r, self.ox, self.oy, self.origin)?;
        let (ox, oy) = match self.origin {
            Origin::Absolute => (self.ox, self.oy),
            Origin::Relative => (self.w * self.ox, self.h * self.oy),
        };
        ib.overlay(
            &img,
            &artwork,
            self.x - dx as i32,
            self.y - dy as i32,
            -ox,
            -oy,
            Origin::Absolute,
            self.blend,
        )
    }
}
