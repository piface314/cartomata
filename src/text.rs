//! Implements text markup parsing and related utility functions.

pub mod attr;
mod font;
mod markup;
mod parser;

use crate::image::Color;
use attr::LayoutAttr;
pub use font::{FontMap, FontPath};
pub use markup::Markup;
pub use parser::{escape, unescape};

pub trait PangoLayoutExt {
    fn combined_pixel_extents(&self) -> pango::Rectangle;
}

impl PangoLayoutExt for pango::Layout {
    fn combined_pixel_extents(&self) -> pango::Rectangle {
        let (ink_rect, log_rect) = self.pixel_extents();
        let left = ink_rect.x().min(log_rect.x());
        let top = ink_rect.y().min(log_rect.y());
        let right = (ink_rect.x() + ink_rect.width()).max(log_rect.x() + log_rect.width());
        let bottom = (ink_rect.y() + ink_rect.height()).max(log_rect.y() + log_rect.height());
        let width = right - left;
        let height = bottom - top;
        pango::Rectangle::new(left, top, width, height)
    }
}

#[derive(Debug, Clone)]
pub struct PrintOptions<'s> {
    pub font: &'s str,
    pub size: f32,
    pub color: Color,
    pub params: &'s [LayoutAttr<'s>],
    pub ink_extents: bool,
}

impl Default for PrintOptions<'static> {
    fn default() -> Self {
        Self {
            font: "default",
            size: 12.0,
            color: Color::BLACK,
            params: &[],
            ink_extents: false,
        }
    }
}
