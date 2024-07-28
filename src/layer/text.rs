//! Represents a layer to render arbitrary text, including single line labels
//! or multiline text areas.

use crate::layer::Layer;
use crate::template::Template;
use crate::{error::Result, image::ImgBackend};

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Serialize, Deserialize, LuaLayer))]
pub struct TextLayer {
    pub text: String,
    pub x: i64,
    pub y: i64,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

impl TextLayer {}

impl Layer for TextLayer {
    fn render(&self, img: VipsImage, _ib: &mut ImgBackend, _template: &Template) -> Result<VipsImage> {
        Ok(img)
    }
}
