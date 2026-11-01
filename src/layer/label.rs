//! Represents a layer to render single line text labels.

use crate::error::ImgError;
use crate::image::{Color, ImgBackend, Position, TextUnit, Unit};
use crate::layer::{Layer, LayerFx, LayerOut, RenderContext};
use crate::text::attr::{Direction, Gravity, GravityHint, LayoutAttr};
use crate::text::Markup;

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::ops as vips;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::Deserialize;

#[derive(Debug)]
#[cfg_attr(feature = "cli", derive(Deserialize))]
#[cfg_attr(feature = "cli", derive(LuaLayer))]
#[cfg_attr(feature = "cli", cartomata_use_crate)]
pub struct LabelLayer {
    pub text: String,
    pub x: Position,
    pub y: Position,
    pub font: Option<String>,
    #[cfg_attr(feature = "cli", serde(default = "default_size"))]
    pub size: f64,
    #[cfg_attr(feature = "cli", serde(default = "default_color"))]
    pub color: Color,
    pub w: Option<i32>,
    #[cfg_attr(feature = "cli", serde(default))]
    pub r: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub ox: Unit,
    #[cfg_attr(feature = "cli", serde(default = "default_text_unit"))]
    pub oy: TextUnit,
    #[cfg_attr(feature = "cli", serde(default = "default_sy"))]
    pub sy: f64,
    #[cfg_attr(feature = "cli", serde(default))]
    pub overflow: Overflow,
    pub ink_extents: bool,
    pub auto_dir: Option<bool>,
    pub dpi: Option<f64>,
    pub direction: Option<Direction>,
    pub gravity: Option<Gravity>,
    pub gravity_hint: Option<GravityHint>,
    pub language: Option<String>,
    #[cfg_attr(feature = "cli", serde(skip))]
    effects: Vec<LayerFx>,
}

const fn default_color() -> Color {
    Color::BLACK
}

const fn default_text_unit() -> TextUnit {
    TextUnit::Baseline
}

const fn default_size() -> f64 {
    12.0
}

const fn default_sy() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "cli", derive(Deserialize))]
#[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
pub enum Overflow {
    #[default]
    Shrink,
    Left,
    Center,
    Right,
}

impl Overflow {
    pub fn as_f64(&self) -> f64 {
        match self {
            Overflow::Shrink => 0.0,
            Overflow::Left => 0.0,
            Overflow::Center => 0.5,
            Overflow::Right => 1.0,
        }
    }
}

impl LabelLayer {
    pub fn new(text: impl Into<String>, x: impl Into<Position>, y: impl Into<Position>) -> Self {
        Self {
            text: text.into(),
            x: x.into(),
            y: y.into(),
            size: default_size(),
            font: Default::default(),
            color: default_color(),
            w: Default::default(),
            r: 0.0,
            ox: Default::default(),
            oy: default_text_unit(),
            sy: default_sy(),
            overflow: Default::default(),
            ink_extents: Default::default(),
            auto_dir: Default::default(),
            dpi: Default::default(),
            direction: Default::default(),
            gravity: Default::default(),
            gravity_hint: Default::default(),
            language: Default::default(),
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
    pub fn with_width(mut self, w: impl Into<i32>) -> Self {
        self.w = Some(w.into());
        self
    }

    #[must_use]
    pub fn with_width_opt(mut self, w: Option<i32>) -> Self {
        self.w = w;
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
    pub fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    #[must_use]
    pub fn with_y_scale(mut self, sy: impl Into<f64>) -> Self {
        self.sy = sy.into();
        self
    }

    #[must_use]
    pub fn with_ink_extents(mut self, ink_extents: bool) -> Self {
        self.ink_extents = ink_extents;
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
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    fn layout_params(&'_ self) -> Vec<LayoutAttr<'_>> {
        let mut params = Vec::new();
        self.auto_dir.map(|x| params.push(LayoutAttr::AutoDir(x)));
        self.dpi.map(|x| params.push(LayoutAttr::Dpi(x)));
        self.direction
            .map(|x| params.push(LayoutAttr::Direction(x)));
        self.gravity.map(|x| params.push(LayoutAttr::Gravity(x)));
        self.gravity_hint
            .map(|x| params.push(LayoutAttr::GravityHint(x)));
        self.language
            .as_ref()
            .map(|x| params.push(LayoutAttr::Language(x)));
        params
    }

    fn resize_x(&self, ib: &ImgBackend, img: VipsImage) -> Result<(VipsImage, Unit), ImgError> {
        let Some(w) = self.w else {
            return Ok((img, self.ox));
        };
        let iw = img.get_width();
        if iw <= w {
            let x = self.ox.apply((w - iw) as f64).round() as i32;
            let img = vips::embed(&img, x, 0, w, img.get_height())?;
            return Ok((img, self.ox));
        }
        match self.overflow {
            Overflow::Shrink => {
                let s = w as f64 / iw as f64;
                Ok((ib.scale(&img, 1.0, s)?, self.ox))
            }
            ov => {
                let w = w as f64;
                let ov = ov.as_f64();
                Ok((img, Unit::Px((iw as f64 - w) * ov + self.ox.apply(w))))
            }
        }
    }
}

impl Layer for LabelLayer {
    fn effects(&self) -> &Vec<LayerFx> {
        &self.effects
    }
    fn effects_mut(&mut self) -> &mut Vec<LayerFx> {
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
        let text_img = ib.scale_checked(text_img, 1.0, self.sy)?;
        let h = ((layout.baseline() as f64) * self.sy).round() as i32;
        let oy = self.oy.into_unit(h);
        let (text_img, ox) = self.resize_x(ib, text_img)?;
        let (img, ox, oy) = ib.rotate_checked(text_img, self.r, ox, oy)?;
        let (x, y) = (self.x.apply(ctx), self.y.apply(ctx));
        Ok(LayerOut::new(
            img,
            (x - ox).round() as i32,
            (y - oy).round() as i32,
        ))
    }
}
