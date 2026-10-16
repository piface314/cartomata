//! Represents a layer to render arbitrary text, including single line labels
//! or multiline text areas.

use crate::error::ImgError;
use crate::image::{BlendMode, Color, ImgBackend, Origin, Stroke, TextOrigin};
use crate::layer::{Layer, RenderContext};
use crate::text::attr::{Direction, Gravity, GravityHint, LayoutAttr};
use crate::text::Markup;

#[cfg(feature = "cli")]
use cartomata_derive::LuaLayer;
use libvips::VipsImage;
#[cfg(feature = "cli")]
use mlua::LuaSerdeExt;
#[cfg(feature = "cli")]
use serde::Deserialize;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize))]
#[cfg_attr(feature = "cli", derive(LuaLayer))]
pub struct LabelLayer {
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
    #[cfg_attr(feature = "cli", serde(default = "default_text_origin"))]
    pub oy: TextOrigin,
    #[cfg_attr(feature = "cli", serde(default))]
    pub blend: BlendMode,
    pub stroke: Option<Stroke>,
    pub auto_dir: Option<bool>,
    pub dpi: Option<f64>,
    pub direction: Option<Direction>,
    pub gravity: Option<Gravity>,
    pub gravity_hint: Option<GravityHint>,
    pub language: Option<String>,
}

const fn default_color() -> Color {
    Color::BLACK
}

const fn default_text_origin() -> TextOrigin {
    TextOrigin::Baseline
}

impl LabelLayer {
    pub fn new(
        text: impl Into<String>,
        x: impl Into<i32>,
        y: impl Into<i32>,
        size: impl Into<f64>,
    ) -> Self {
        Self {
            text: text.into(),
            x: x.into(),
            y: y.into(),
            size: size.into(),
            font: Default::default(),
            color: default_color(),
            w: Default::default(),
            r: 0.0,
            ox: Default::default(),
            oy: default_text_origin(),
            blend: Default::default(),
            stroke: Default::default(),
            auto_dir: Default::default(),
            dpi: Default::default(),
            direction: Default::default(),
            gravity: Default::default(),
            gravity_hint: Default::default(),
            language: Default::default(),
        }
    }

    #[must_use]
    pub fn with_font(mut self, font: impl Into<String>) -> Self {
        self.font = Some(font.into());
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
    pub fn with_rotation(mut self, r: impl Into<f64>) -> Self {
        self.r = r.into();
        self
    }

    #[must_use]
    pub fn with_origin(mut self, ox: Origin, oy: TextOrigin) -> Self {
        self.ox = ox;
        self.oy = oy;
        self
    }

    #[must_use]
    pub fn with_blend_mode(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }

    #[must_use]
    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
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

    fn resize(&self, ib: &ImgBackend, img: VipsImage) -> Result<VipsImage, ImgError> {
        if let Some(w) = self.w {
            let iw = img.get_width();
            if iw > w {
                let s = w as f64 / iw as f64;
                ib.scale(&img, 1.0, s)
            } else {
                Ok(img)
            }
        } else {
            Ok(img)
        }
    }
}

impl Layer for LabelLayer {
    fn render(&self, img: VipsImage, ctx: &RenderContext) -> Result<VipsImage, ImgError> {
        let img_map = ctx.img_map;
        let font_map = ctx.font_map;
        let ib = ctx.backend;

        let markup = Markup::from_string(&self.text)?;
        let font = self.font.as_ref().map(|x| x.as_str()).unwrap_or("default");
        let params = self.layout_params();
        let (text_img, layout) = ib.print(
            markup, &img_map, &font_map, font, self.size, self.color, &params,
        )?;
        let text_img = self.resize(&ib, text_img)?;
        let (text_img, dh) = if let Some(stroke) = self.stroke {
            (ib.stroke_checked(text_img, stroke)?, stroke.size)
        } else {
            (text_img, 0)
        };
        let h = layout.baseline() + dh;
        let (text_img, ox, oy) =
            ib.rotate_checked(text_img, self.r, self.ox, self.oy.into_origin(h))?;
        let (ox, oy) = (Origin::Absolute(ox), Origin::Absolute(oy));
        ib.overlay(&img, &text_img, self.x, self.y, ox, oy, self.blend)
    }
}
