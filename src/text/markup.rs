use crate::error::Result;
use crate::image::Color;
use crate::text::attr::{Gravity, ITagAttr, ImgAttr, Points, Scale, SpanAttr, TagAttr};
use crate::text::parser::TextParser;

#[derive(Debug, Clone)]
pub enum Markup {
    Root(Vec<Markup>),
    Text(String),
    SpanTag(Vec<SpanAttr>, Vec<Markup>),
    ImgTag(ImgAttr),
}

impl Markup {
    pub fn from_string(markup: &str) -> Result<Self> {
        TextParser::new(markup).parse()
    }

    pub fn push_attr(&mut self, key: &str, value: &str) -> Result<()> {
        match self {
            Self::SpanTag(attrs, _) => attrs.push(SpanAttr::from_key_value(key, value)?),
            Self::ImgTag(attrs) => attrs.push(key, value)?,
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
        base_gravity: Gravity,
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
            base_gravity,
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
        mut gravity: Gravity,
    ) {
        match self {
            Self::Root(m) => {
                for m in m.into_iter() {
                    m.parsed_r(attrs, text, font, size, scale, color, alpha, gravity);
                }
            }
            Self::Text(t) => text.push_str(&t),
            Self::SpanTag(a, m) => {
                let i = attrs.len();
                let start_index = text.len();
                for a in a.into_iter() {
                    let ia = ITagAttr::new(TagAttr::Span(a));
                    attrs.push(ia);
                    match &attrs.last().unwrap().value {
                        TagAttr::Span(SpanAttr::Font(new_font)) => font.clone_from(new_font),
                        TagAttr::Span(SpanAttr::Size(Points(new_size))) => size = *new_size,
                        TagAttr::Span(SpanAttr::Scale(Scale(new_scale))) => scale = *new_scale,
                        TagAttr::Span(SpanAttr::Color(new_color)) => color = *new_color,
                        TagAttr::Span(SpanAttr::Alpha(new_alpha)) => alpha = *new_alpha,
                        TagAttr::Span(SpanAttr::Gravity(new_gravity)) => gravity = *new_gravity,
                        _ => {}
                    }
                }
                let j = attrs.len();
                for m in m.into_iter() {
                    m.parsed_r(attrs, text, font, size, scale, color, alpha, gravity);
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
                    value: TagAttr::Img(a.configured(font, size, scale, color, alpha, gravity)),
                    start_index: start_index as u32,
                    end_index: start_index as u32 + 1,
                })
            }
        }
    }
}
