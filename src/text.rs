//! Implements text markup parsing and related utility functions.

pub mod attr;
mod font;
mod markup;
mod parser;

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

