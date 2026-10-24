//! Represents a layer to render arbitrary text, bounded inside a certain width.

use crate::error::ImgError;
use crate::image::{Color, Position, TextUnit, Unit};
use crate::layer::{Layer, LayerFx, LayerOut, RenderContext};
use crate::text::attr::{Alignment, Direction, Gravity, GravityHint, LayoutAttr, WrapMode};
use crate::text::Markup;
use libvips::ops as vips;

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
    pub x: Position,
    pub y: Position,
    pub w: i32,
    pub font: Option<String>,
    #[cfg_attr(feature = "cli", serde(default = "default_size"))]
    pub size: f64,
    #[cfg_attr(feature = "cli", serde(default = "default_color"))]
    pub color: Color,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub ox: Unit,
    #[cfg_attr(feature = "cli", serde(default))]
    pub oy: TextUnit,
    pub ink_extents: bool,
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

const fn default_color() -> Color {
    Color::BLACK
}

const fn default_size() -> f64 {
    12.0
}

impl TextLayer {
    pub fn new(
        text: impl Into<String>,
        x: impl Into<Position>,
        y: impl Into<Position>,
        w: impl Into<i32>,
    ) -> Self {
        Self {
            text: text.into(),
            x: x.into(),
            y: y.into(),
            w: w.into(),
            font: Default::default(),
            size: default_size(),
            color: default_color(),
            r: Default::default(),
            ox: Default::default(),
            oy: Default::default(),
            ink_extents: Default::default(),
            align: Default::default(),
            auto_dir: Default::default(),
            dpi: Default::default(),
            direction: Default::default(),
            gravity: Default::default(),
            gravity_hint: Default::default(),
            indent: Default::default(),
            justify: Default::default(),
            language: Default::default(),
            line_spacing: Default::default(),
            spacing: Default::default(),
            wrap: Default::default(),
            effects: Default::default(),
        }
    }

    #[must_use]
    pub fn with_font(mut self, font: impl Into<String>, size: impl Into<f64>) -> Self {
        self.font = Some(font.into());
        self.size = size.into();
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn with_rotation(mut self, r: impl Into<f64>) -> Self {
        self.r = r.into();
        self
    }

    #[must_use]
    pub fn with_origin(mut self, ox: Unit, oy: TextUnit) -> Self {
        self.ox = ox;
        self.oy = oy;
        self
    }

    #[must_use]
    pub fn with_ink_extents(mut self, ink_extents: bool) -> Self {
        self.ink_extents = ink_extents;
        self
    }

    #[must_use]
    pub fn with_align(mut self, align: Alignment) -> Self {
        self.align = Some(align);
        self
    }

    #[must_use]
    pub fn with_auto_dir(mut self, auto_dir: bool) -> Self {
        self.auto_dir = Some(auto_dir);
        self
    }

    #[must_use]
    pub fn with_dpi(mut self, dpi: impl Into<f64>) -> Self {
        self.dpi = Some(dpi.into());
        self
    }

    #[must_use]
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    #[must_use]
    pub fn with_gravity(mut self, gravity: Gravity) -> Self {
        self.gravity = Some(gravity);
        self
    }

    #[must_use]
    pub fn with_gravity_hint(mut self, gravity_hint: GravityHint) -> Self {
        self.gravity_hint = Some(gravity_hint);
        self
    }

    #[must_use]
    pub fn with_indent(mut self, indent: f64) -> Self {
        self.indent = Some(indent);
        self
    }

    #[must_use]
    pub fn with_justify(mut self, justify: bool) -> Self {
        self.justify = Some(justify);
        self
    }

    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    #[must_use]
    pub fn with_line_spacing(mut self, line_spacing: f64) -> Self {
        self.line_spacing = Some(line_spacing);
        self
    }

    #[must_use]
    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = Some(spacing);
        self
    }

    #[must_use]
    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = Some(wrap);
        self
    }

    fn layout_params(&'_ self) -> Vec<LayoutAttr<'_>> {
        let mut params = vec![LayoutAttr::Width(self.w)];
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
        let text_img = if self.ink_extents {
            let (rect, _) = layout.extents();
            let x = (rect.x() as f64 / pango::SCALE as f64).round() as i32;
            let y = (rect.y() as f64 / pango::SCALE as f64).round() as i32;
            let w = (rect.width() as f64 / pango::SCALE as f64).round() as i32;
            let h = (rect.height() as f64 / pango::SCALE as f64).round() as i32;
            vips::extract_area(&text_img, x, y, w, h).map_err(|e| ib.err(e))?
        } else {
            text_img
        };
        let h = layout.baseline();
        let (img, ox, oy) = ib.rotate_checked(text_img, self.r, self.ox, self.oy.into_unit(h))?;
        let (x, y) = (self.x.apply(ctx), self.y.apply(ctx));
        Ok(LayerOut::new(
            img,
            (x - ox).round() as i32,
            (y - oy).round() as i32,
        ))
    }
}
