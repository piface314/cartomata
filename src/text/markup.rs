use crate::error::Result;
use crate::image::Color;
use crate::text::attr::{ITagAttr, IconAttr, ImgAttr, Scale, Points, SpanAttr, TagAttr};

#[derive(Debug, Clone)]
pub enum Markup {
    Root(Vec<Markup>),
    Text(String),
    SpanTag(Vec<SpanAttr>, Vec<Markup>),
    ImgTag(ImgAttr),
    IconTag(IconAttr),
}

impl Markup {
    pub fn push_attr(&mut self, key: &str, value: &str) -> Result<()> {
        match self {
            Self::SpanTag(attrs, _) => attrs.push(SpanAttr::from_key_value(key, value)?),
            Self::ImgTag(attrs) => attrs.push(key, value)?,
            Self::IconTag(attrs) => attrs.push(key, value)?,
            _ => unreachable!("trying to add attr to non tag"),
        };
        Ok(())
    }

    pub fn push_elem(&mut self, elem: Markup) {
        match self {
            Self::Root(v) => v.push(elem),
            Self::SpanTag(_, v) => v.push(elem),
            _ => unreachable!("trying to add elem to non span"),
        }
    }

    pub fn parsed(
        self,
        mut base_font: String,
        base_size: i32,
        base_color: Color,
    ) -> (Vec<ITagAttr>, String) {
        let mut attrs = Vec::new();
        let mut text = String::new();
        self.parsed_r(
            &mut attrs,
            &mut text,
            &mut base_font,
            base_size,
            1.0,
            base_color,
            1.0,
        );
        (attrs, text)
    }

    fn parsed_r(
        self,
        attrs: &mut Vec<ITagAttr>,
        text: &mut String,
        font: &mut String,
        mut size: i32,
        mut scale: f64,
        mut color: Color,
        mut alpha: f64,
    ) {
        match self {
            Self::Root(m) => {
                for m in m.into_iter() {
                    m.parsed_r(attrs, text, font, size, scale, color, alpha);
                }
            }
            Self::Text(t) => text.push_str(&t),
            Self::SpanTag(a, m) => {
                let i = attrs.len();
                let start_index = text.len();
                for a in a.into_iter() {
                    let ia = ITagAttr::new(TagAttr::Span(a));
                    attrs.push(ia);
                    match &attrs.first().unwrap().value {
                        TagAttr::Span(SpanAttr::Font(new_font)) => font.clone_from(new_font),
                        TagAttr::Span(SpanAttr::Size(Points(new_size))) => size = *new_size,
                        TagAttr::Span(SpanAttr::Scale(Scale(new_scale))) => scale = *new_scale,
                        TagAttr::Span(SpanAttr::Color(new_color)) => color = *new_color,
                        TagAttr::Span(SpanAttr::Alpha(new_alpha)) => alpha = *new_alpha,
                        _ => {}
                    }
                }
                let j = attrs.len();
                for m in m.into_iter() {
                    m.parsed_r(attrs, text, font, size, scale, color, alpha);
                }
                let end_index = text.len();
                for a in attrs[i..j].iter_mut() {
                    a.start_index = start_index as u32;
                    a.end_index = end_index as u32;
                }
            }
            Self::ImgTag(a) => {
                let start_index = text.len();
                text.push_str("*");
                attrs.push(ITagAttr {
                    value: TagAttr::Img(a.configured(font, size, scale, alpha)),
                    start_index: start_index as u32,
                    end_index: start_index as u32 + 1,
                })
            }
            Self::IconTag(a) => {
                let start_index = text.len();
                text.push_str("*");
                attrs.push(ITagAttr {
                    value: TagAttr::Icon(a.configured(font, size, scale, color, alpha)),
                    start_index: start_index as u32,
                    end_index: start_index as u32 + 1,
                })
            }
        }
    }
}
