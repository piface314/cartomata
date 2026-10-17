//! Represents a layer to render arbitrary text, including single line labels
//! or multiline text areas.

use crate::error::ImgError;
use crate::image::{Color, Origin, TextOrigin};
use crate::layer::{Layer, LayerFx, LayerOut, RenderContext};
use crate::text::attr::{Alignment, Direction, Gravity, GravityHint, LayoutAttr, WrapMode};
use crate::text::Markup;

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::Deserialize;

#[derive(Debug)]
#[cfg_attr(feature = "cli", derive(LuaLayer))]
#[cfg_attr(feature = "cli", derive(Deserialize))]
#[cfg_attr(feature = "cli", cartomata_use_crate)]
pub struct TextLayer {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub size: f64,
    pub font: Option<String>,
    #[cfg_attr(feature = "cli", serde(default = "default_color"))]
    pub color: Color,
    pub w: Option<i32>,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub ox: Origin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub oy: TextOrigin,
    pub align: Option<Alignment>,
    pub auto_dir: Option<bool>,
    pub dpi: Option<f64>,
    pub direction: Option<Direction>,
    pub gravity: Option<Gravity>,
    pub gravity_hint: Option<GravityHint>,
    pub indent: Option<f64>,
    pub justify: Option<bool>,
    pub language: Option<String>,
    pub line_spacing: Option<f64>,
    pub spacing: Option<f64>,
    pub wrap: Option<WrapMode>,
    #[cfg_attr(feature = "cli", serde(skip))]
    effects: Vec<LayerFx>,
}

#[cfg(feature = "cli")]
const fn default_color() -> Color {
    Color::BLACK
}

impl TextLayer {
    fn layout_params(&'_ self) -> Vec<LayoutAttr<'_>> {
        let mut params = Vec::new();
        self.align.map(|x| params.push(LayoutAttr::Alignment(x)));
        self.auto_dir.map(|x| params.push(LayoutAttr::AutoDir(x)));
        self.dpi.map(|x| params.push(LayoutAttr::Dpi(x)));
        self.direction
            .map(|x| params.push(LayoutAttr::Direction(x)));
        self.gravity.map(|x| params.push(LayoutAttr::Gravity(x)));
        self.gravity_hint
            .map(|x| params.push(LayoutAttr::GravityHint(x)));
        self.indent.map(|x| params.push(LayoutAttr::Indent(x)));
        self.justify.map(|x| params.push(LayoutAttr::Justify(x)));
        self.language
            .as_ref()
            .map(|x| params.push(LayoutAttr::Language(x)));
        self.line_spacing
            .map(|x| params.push(LayoutAttr::LineSpacing(x)));
        self.spacing.map(|x| params.push(LayoutAttr::Spacing(x)));
        self.w.map(|x| params.push(LayoutAttr::Width(x)));
        self.wrap.map(|x| params.push(LayoutAttr::Wrap(x)));
        params
    }
}

impl Layer for TextLayer {
    fn effects(&self) -> &Vec<super::LayerFx> {
        &self.effects
    }
    fn effects_mut(&mut self) -> &mut Vec<super::LayerFx> {
        &mut self.effects
    }
    fn render(&mut self, ctx: &RenderContext) -> Result<LayerOut, ImgError> {
        let img_map = ctx.img_map;
        let font_map = ctx.font_map;
        let ib = ctx.backend;

        let markup = Markup::from_string(&self.text)?;
        let font = self.font.as_ref().map(|x| x.as_str()).unwrap_or("default");
        let params = self.layout_params();
        let (text_img, layout) = ib.print(
            markup, &img_map, &font_map, font, self.size, self.color, &params,
        )?;
        let h = layout.baseline();
        let (img, ox, oy) = ib.rotate_checked(text_img, self.r, self.ox, self.oy.into_origin(h))?;
        Ok(LayerOut::new(
            img,
            (self.x as f64 - ox).round() as i32,
            (self.y as f64 - oy).round() as i32,
        ))
    }
}
