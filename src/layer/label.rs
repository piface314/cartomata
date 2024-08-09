//! Represents a layer to render arbitrary text, including single line labels
//! or multiline text areas.

use crate::error::Result;
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
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Serialize, Deserialize, LuaLayer))]
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
    fn layout_params(&self) -> Vec<LayoutAttr> {
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

    fn resize(&self, ib: &ImgBackend, img: VipsImage) -> Result<VipsImage> {
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
    fn render(&self, img: VipsImage, ctx: &mut RenderContext) -> Result<VipsImage> {
        let markup = Markup::from_string(&self.text)?;
        let font = self.font.as_ref().map(|x| x.as_str()).unwrap_or("default");
        let params = self.layout_params();
        let (text_img, layout) = ctx.backend.print(
            markup,
            ctx.img_map,
            ctx.font_map,
            font,
            self.size,
            self.color,
            &params,
        )?;
        let text_img = self.resize(&ctx.backend, text_img)?;
        let (text_img, dh) = if let Some(stroke) = self.stroke {
            (ctx.backend.stroke(&text_img, stroke)?, stroke.size)
        } else {
            (text_img, 0)
        };
        let h = layout.baseline() + dh;
        let (text_img, ox, oy) =
            ctx.backend
                .rotate(&text_img, self.r, self.ox, self.oy.into_origin(h))?;
        let (ox, oy) = (Origin::Absolute(ox), Origin::Absolute(oy));
        ctx.backend
            .overlay(&img, &text_img, self.x, self.y, ox, oy, self.blend)
    }
}
