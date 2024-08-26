//! Represents a layer to render arbitrary text, including single line labels
//! or multiline text areas.

use crate::error::Result;
use crate::image::{BlendMode, Color, Origin, Stroke, TextOrigin};
use crate::layer::{Layer, RenderContext};
use crate::text::attr::{Alignment, Direction, Gravity, GravityHint, LayoutAttr, WrapMode};
use crate::text::Markup;

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "cli", derive(LuaLayer))]
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
    #[cfg_attr(feature = "cli", serde(default))]
    pub blend: BlendMode,
    pub stroke: Option<Stroke>,
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
}

const fn default_color() -> Color {
    Color::BLACK
}

impl TextLayer {
    fn layout_params(&self) -> Vec<LayoutAttr> {
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
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage> {
        let img_map = ctx.img_map;
        let font_map = ctx.font_map;
        let ib = ctx.backend;

        let markup = Markup::from_string(&self.text)?;
        let font = self.font.as_ref().map(|x| x.as_str()).unwrap_or("default");
        let params = self.layout_params();
        let (text_img, layout) = ib.print(
            markup, &img_map, &font_map, font, self.size, self.color, &params,
        )?;
        let (text_img, dh) = if let Some(stroke) = self.stroke {
            (ib.stroke(&text_img, stroke)?, stroke.size)
        } else {
            (text_img, 0)
        };
        let h = layout.baseline() + dh;
        let (text_img, ox, oy) = ib.rotate(&text_img, self.r, self.ox, self.oy.into_origin(h))?;
        let (ox, oy) = (Origin::Absolute(ox), Origin::Absolute(oy));
        ib.overlay(&img, &text_img, self.x, self.y, ox, oy, self.blend)
    }
}
