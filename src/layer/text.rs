//! Represents a layer to render arbitrary text, including single line labels
//! or multiline text areas. 

use crate::error::Result;
use crate::layer::Layer;
use crate::template::Template;

use cartomata_derive::LuaLayer;
use mlua::LuaSerdeExt;
use cairo::{Context, ImageSurface};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, LuaLayer)]
pub struct TextLayer {
    pub text: String,
    pub x: i64,
    pub y: i64,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

impl TextLayer {
    
}

impl Layer for TextLayer {
    fn render(&self, _template: &Template, _cr: &Context) -> Result<()> {
        
        Ok(())
    }
}
