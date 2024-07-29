use std::any::Any;

use crate::error::Result;
use crate::image::Color;
use crate::text::attr::{Attr, IconAttr, ImgAttr, IndexedAttr, Scale, Size, SpanAttr};
use crate::text::font::FontManager;

#[derive(Debug, Clone)]
pub enum Markup<'f> {
    Root(Vec<Markup<'f>>),
    Text(String),
    SpanTag(Vec<SpanAttr<'f>>, Vec<Markup<'f>>),
    ImgTag(ImgAttr),
    IconTag(IconAttr),
}

impl<'f> Markup<'f> {
    pub fn push_attr(&mut self, fm: &'f FontManager<'f>, key: &str, value: &str) -> Result<()> {
        match self {
            Self::SpanTag(attrs, _) => attrs.push(SpanAttr::from_key_value(fm, key, value)?),
            Self::ImgTag(attrs) => attrs.push(key, value)?,
            Self::IconTag(attrs) => attrs.push(key, value)?,
            _ => unreachable!("trying to add attr to non tag"),
        };
        Ok(())
    }

    pub fn push_elem(&mut self, elem: Markup<'f>) {
        match self {
            Self::Root(v) => v.push(elem),
            Self::SpanTag(_, v) => v.push(elem),
            _ => unreachable!("trying to add elem to non span"),
        }
    }

    pub fn parsed(
        self,
        fm: &'f FontManager,
        base_font: &'f str,
        base_size: i32,
        base_color: Color,
    ) -> (Vec<IndexedAttr<'f>>, String) {
        let mut attrs = Vec::new();
        let mut text = String::new();
        self.parsed_r(
            &mut attrs, &mut text, fm, base_font, base_size, 1.0, base_color, 1.0,
        );
        (attrs, text)
    }

    fn parsed_r(
        self,
        attrs: &mut Vec<IndexedAttr<'f>>,
        text: &mut String,
        fm: &'f FontManager<'f>,
        mut font: &'f str,
        mut size: i32,
        mut scale: f64,
        mut color: Color,
        mut alpha: f64,
    ) {
        match self {
            Self::Root(m) => {
                for m in m.into_iter() {
                    m.parsed_r(attrs, text, fm, font, size, scale, color, alpha);
                }
            }
            Self::Text(t) => text.push_str(&t),
            Self::SpanTag(a, m) => {
                let i = attrs.len();
                let start_index = text.len();
                for a in a.into_iter() {
                    let ia = IndexedAttr::new(Attr::Span(a));
                    attrs.push(ia);
                    if let Some(ia) = attrs.first() {
                        match &ia.value {
                            Attr::Span(SpanAttr::Font(new_font, _)) => font = new_font,
                            Attr::Span(SpanAttr::Size(Size(new_size))) => size = *new_size,
                            Attr::Span(SpanAttr::Scale(Scale(new_scale))) => scale = *new_scale,
                            Attr::Span(SpanAttr::Color(new_color)) => color = *new_color,
                            Attr::Span(SpanAttr::Alpha(new_alpha)) => alpha = *new_alpha,
                            _ => {}
                        }
                    }
                }
                let j = attrs.len();
                for m in m.into_iter() {
                    m.parsed_r(attrs, text, fm, font, size, scale, color, alpha);
                }
                let end_index = text.len();
                for a in attrs[i..j].iter_mut() {
                    a.start_index = start_index as u32;
                    a.end_index = end_index as u32;
                }
            }
            Self::ImgTag(a) => {
                let font = fm.get(font).expect("font existence checked");
                let start_index = text.len();
                text.push_str("*");
                attrs.push(IndexedAttr {
                    value: Attr::Img(a.configured(font, size, scale, alpha)),
                    start_index: start_index as u32,
                    end_index: start_index as u32 + 1,
                })
            }
            Self::IconTag(a) => {
                let font = fm.get(font).expect("font existence checked");
                let start_index = text.len();
                text.push_str("*");
                attrs.push(IndexedAttr {
                    value: Attr::Icon(a.configured(font, size, scale, color, alpha)),
                    start_index: start_index as u32,
                    end_index: start_index as u32 + 1,
                })
            }
        }
    }
}
