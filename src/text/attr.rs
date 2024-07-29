//! Text attribute values and conversions.

use crate::image::{Color, ImgBackend};
use crate::text::FontManager;

use fontconfig::Pattern;
use libvips::VipsImage;
use regex::Regex;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct IndexedAttr<'f> {
    pub value: Attr<'f>,
    pub start_index: u32,
    pub end_index: u32,
}

impl<'f> IndexedAttr<'f> {
    pub fn new(value: Attr<'f>) -> Self {
        Self {
            value,
            start_index: 0,
            end_index: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Attr<'f> {
    Span(SpanAttr<'f>),
    Img(ImgAttr),
    Icon(IconAttr),
}

macro_rules! attr_parse {
    ($fm:expr; $key:expr; $val:expr; $($pat:expr, $att:ident, $T:ty);*) => {{
        match $key {
            $($pat => Ok(Self::$att($val.parse::<$T>().map_err(|e|
                crate::error::Error::TextAttrParseError($key.to_string(), e.to_string())
            )?)), ) *
            _ => Err(crate::error::Error::TextInvalidAttr($key.to_string())),
        }
    }};
}

macro_rules! enum_attr {
    (
        $(#[$outer:meta])*
        $vis:vis enum $Attr:ident <$lt:lifetime> {
            $FontVar:ident($FT:ty),
            $( $k:literal => $Variant:ident($T:ty) ),*,
        }
    ) => {
        $(#[$outer])*
        $vis enum $Attr<$lt> {
            $FontVar($FT, pango::FontDescription),
            $( $Variant($T) ),*
        }

        impl<$lt> $Attr<$lt> {
            pub fn from_key_value(fm: &$lt FontManager<$lt>, key: &str, value: &str) -> crate::error::Result<Self> {
                if key == "font" {
                    match fm.check(value) {
                        Some((f, pat)) => {
                            let desc = pango::FontDescription::from_string(pat.name().unwrap_or(""));
                            Ok(Self::$FontVar(f, desc))
                        },
                        None => Err(crate::error::Error::FontCacheMiss(value.to_string()))
                    }
                } else {
                    attr_parse!(fm; key; value; $( $k, $Variant, $T );*)
                }
            }
        }
    };
}

macro_rules! struct_attr {
    (
        $(#[$outer:meta])*
        $vis:vis struct $Attr:ident {
            $( $k:literal => $Field:ident: $T:ty ),*,
            $( $ExtraField:ident: $ET:ty ),*
        }
    ) => {
        $(#[$outer])*
        $vis struct $Attr {
            $( pub $Field: Option<$T>, )*
            $( pub $ExtraField: Option<$ET>, )*
        }

        impl $Attr {
            pub fn push(&mut self, key: &str, value: &str) -> crate::error::Result<()> {
                match key {
                    $($k => {
                        let parsed = value.parse::<$T>().map_err(|e|
                            crate::error::Error::TextAttrParseError(key.to_string(), e.to_string())
                        )?;
                        self.$Field = Some(parsed);
                        Ok(())
                    } ) *
                    _ => Err(crate::error::Error::TextInvalidAttr(key.to_string())),
                }
            }
        }
    };
}

enum_attr! {
    #[derive(Debug, Clone)]
    pub enum SpanAttr<'f> {
        Font(&'f str),
        "features" => Features(String),
        "size" => Size(Size),
        "scale" => Scale(Scale),
        "color" => Color(Color),
        "alpha" => Alpha(f64),
        "bg-color" => BgColor(Color),
        "bg-alpha" => BgAlpha(f64),
        "underline" => Underline(Underline),
        "underline-color" => UnderlineColor(Color),
        "overline" => Overline(Overline),
        "overline-color" => OverlineColor(Color),
        "rise" => Rise(Size),
        "baseline-shift" => BaselineShift(BaselineShift),
        "strikethrough" => Strikethrough(bool),
        "strikethrough-color" => StrikethroughColor(Color),
        "fallback" => Fallback(bool),
        "lang" => Lang(String),
        "letter-spacing" => LetterSpacing(i32),
        "gravity" => Gravity(Gravity),
        "gravity-hint" => GravityHint(GravityHint),
        "show" => Show(ShowFlags),
        "insert-hyphens" => InsertHyphens(bool),
        "allow-breaks" => AllowBreaks(bool),
        "line-height" => LineHeight(f64),
        "text-transform" => TextTransform(TextTransform),
        "segment" => Segment(Segment),
    }
}

macro_rules! indexed {
    ($attr:expr; at $i:ident, $j:ident) => {{
        let mut attr = $attr;
        attr.set_start_index($i);
        attr.set_end_index($j);
        attr
    }};
}

macro_rules! push {
    ($attrs:ident << AttrInt $fn:ident($v:ident) at $i:ident, $j:ident) => {
        $attrs.insert(indexed!(pango::AttrInt::$fn(*$v); at $i, $j))
    };
    ($attrs:ident << AttrFloat $fn:ident($v:ident) at $i:ident, $j:ident) => {
        $attrs.insert(indexed!(pango::AttrFloat::$fn(*$v); at $i, $j))
    };
    ($attrs:ident << AttrColor $fn:ident($v:expr) at $i:ident, $j:ident) => {{
        let (r, g, b) = $v.pango_rgb();
        $attrs.insert(indexed!(pango::AttrColor::$fn(r, g, b); at $i, $j));
    }};
    ($attrs:ident << AttrAlpha $fn:ident($v:expr) at $i:ident, $j:ident) => {
        $attrs.insert(indexed!(pango::AttrInt::$fn(Color::pango_channel(*$v)); at $i, $j))
    };
    ($attrs:ident << AttrRgba $fnc:ident $fna:ident($v:expr) at $i:ident, $j:ident) => {{
        push!($attrs << AttrColor $fnc($v) at $i, $j);
        if let Some(a) = &$v.a {
            push!($attrs << AttrAlpha $fna(a) at $i, $j);
        }
    }};
}

impl<'f> SpanAttr<'f> {
    pub fn push_pango_attrs(&self, attrs: &mut pango::AttrList, i: u32, j: u32) {
        match self {
            Self::Font(_, desc) => attrs.insert(indexed!(pango::AttrFontDesc::new(desc); at i, j)),
            Self::Features(features) => {
                attrs.insert(indexed!(pango::AttrFontFeatures::new(features); at i, j))
            }
            Self::Size(Size(size)) => attrs.insert(indexed!(pango::AttrSize::new(*size); at i, j)),
            Self::Scale(Scale(scale)) => push!(attrs << AttrFloat new_scale(scale) at i, j),
            Self::Color(color) => {
                push!(attrs << AttrRgba new_foreground new_foreground_alpha(color) at i, j)
            }
            Self::Alpha(a) => push!(attrs << AttrAlpha new_background_alpha(a) at i, j),
            Self::BgColor(color) => {
                push!(attrs << AttrRgba new_background new_background_alpha(color) at i, j);
            }
            Self::BgAlpha(a) => push!(attrs << AttrAlpha new_background_alpha(a) at i, j),
            Self::Underline(Underline(underline)) => {
                push!(attrs << AttrInt new_underline(underline) at i, j)
            }
            Self::UnderlineColor(color) => {
                push!(attrs << AttrColor new_underline_color(color) at i, j)
            }
            Self::Overline(Overline(overline)) => {
                push!(attrs << AttrInt new_overline(overline) at i, j)
            }
            Self::OverlineColor(color) => {
                push!(attrs << AttrColor new_overline_color(color) at i, j)
            }
            Self::Rise(Size(rise)) => push!(attrs << AttrInt new_rise(rise) at i, j),
            Self::BaselineShift(BaselineShift(shift)) => {
                push!(attrs << AttrInt new_baseline_shift(shift) at i, j)
            }
            Self::Strikethrough(s) => push!(attrs << AttrInt new_strikethrough(s) at i, j),
            Self::StrikethroughColor(color) => {
                push!(attrs << AttrColor new_strikethrough_color(color) at i, j)
            }
            Self::Fallback(f) => push!(attrs << AttrInt new_fallback(f) at i, j),
            Self::Lang(lang) => attrs.insert(
                indexed!(pango::AttrLanguage::new( &pango::Language::from_string(lang), ); at i, j),
            ),
            Self::LetterSpacing(ls) => push!(attrs << AttrInt new_letter_spacing(ls) at i, j),
            Self::Gravity(Gravity(g)) => push!(attrs << AttrInt new_gravity(g) at i, j),
            Self::GravityHint(GravityHint(gh)) => {
                push!(attrs << AttrInt new_gravity_hint(gh) at i, j)
            }
            Self::Show(ShowFlags(f)) => push!(attrs << AttrInt new_show(f) at i, j),
            Self::InsertHyphens(f) => push!(attrs << AttrInt new_insert_hyphens(f) at i, j),
            Self::AllowBreaks(f) => push!(attrs << AttrInt new_allow_breaks(f) at i, j),
            Self::LineHeight(f) => push!(attrs << AttrFloat new_line_height(f) at i, j),
            Self::TextTransform(TextTransform(f)) => {
                push!(attrs << AttrInt new_text_transform(f) at i, j)
            }
            Self::Segment(Segment::Word) => {
                attrs.insert(indexed!(pango::AttrInt::new_word(); at i, j))
            }
            Self::Segment(Segment::Sentence) => {
                attrs.insert(indexed!(pango::AttrInt::new_sentence(); at i, j))
            }
        }
    }
}

struct_attr! {
    #[derive(Debug, Clone, Default)]
    pub struct ImgAttr {
        "src" => src: String,
        "width" => width: i32,
        "height" => height: i32,
        "scale" => scale: Scale,
        "alpha" => alpha: f64,
        desc: pango::FontDescription
    }
}

impl ImgAttr {
    #[must_use]
    pub fn configured(mut self, font: &Pattern, size: i32, scale: f64, alpha: f64) -> Self {
        let size = size as f64 * self.scale.map(|s| s.0).unwrap_or(scale);
        self.desc = font
            .name()
            .map(|name| pango::FontDescription::from_string(&format!("{name} {size}")));
        if self.alpha.is_none() {
            self.alpha = Some(alpha);
        }
        self
    }

    pub fn push_pango_attrs(
        &self,
        ib: &ImgBackend,
        attrs: &mut pango::AttrList,
        ctx: &pango::Context,
        i: u32,
        j: u32,
    ) -> Option<VipsImage> {
        if self.src.is_none() {
            return None;
        }
        let metrics = ctx.metrics(self.desc.as_ref(), None);
        let fp = self.src.as_ref().unwrap();
        let cached_img = ib.get_cached(fp).ok();
        let new_img = if cached_img.is_none() {
            ib.open(fp).ok()
        } else {
            None
        };
        let img = cached_img.or(new_img.as_ref())?;
        let img = match self {
            Self {
                width: None,
                height: None,
                scale: Some(Scale(s)),
                ..
            } => ib
                .scale_to(img, None, Some(s * metrics.height() as f64))
                .ok()?,
            Self { width, height, .. } => ib
                .scale_to(img, width.map(|v| v as f64), height.map(|v| v as f64))
                .ok()?,
        };
        let img = match self.alpha {
            Some(a) => ib.set_opacity(&img, a).ok()?,
            None => img,
        };
        let asc_ratio = metrics.ascent() as f64 / metrics.height() as f64;
        let (w, h) = (img.get_width() * pango::SCALE, img.get_height() * pango::SCALE);
        let y = (h as f64 * asc_ratio) as i32;
        let rect = pango::Rectangle::new(0, -y, w, h);
        attrs.insert(indexed!(pango::AttrShape::new(&rect, &rect); at i, j));
        Some(img)
    }
}

struct_attr! {
    #[derive(Debug, Clone, Default)]
    pub struct IconAttr {
        "src" => src: String,
        "width" => width: i32,
        "height" => height: i32,
        "scale" => scale: Scale,
        "color" => color: Color,
        "alpha" => alpha: f64,
        desc: pango::FontDescription
    }
}

impl IconAttr {
    #[must_use]
    pub fn configured(
        mut self,
        font: &Pattern,
        size: i32,
        scale: f64,
        color: Color,
        alpha: f64,
    ) -> Self {
        let size = size as f64 * self.scale.map(|s| s.0).unwrap_or(scale);
        self.desc = font
            .name()
            .map(|name| pango::FontDescription::from_string(&format!("{name} {size}")));
        if self.color.is_none() {
            self.color = Some(color);
        }
        if self.alpha.is_none() {
            self.alpha = Some(alpha);
        }
        self
    }

    pub fn push_pango_attrs(
        &self,
        ib: &ImgBackend,
        attrs: &mut pango::AttrList,
        ctx: &pango::Context,
        i: u32,
        j: u32,
    ) -> Option<VipsImage> {
        if self.src.is_none() {
            return None;
        }
        let metrics = ctx.metrics(self.desc.as_ref(), None);
        let fp = self.src.as_ref().unwrap();
        let cached_img = ib.get_cached(fp).ok();
        let new_img = if cached_img.is_none() {
            ib.open(fp).ok()
        } else {
            None
        };
        let img = cached_img.or(new_img.as_ref())?;
        let img = match self {
            Self {
                width: None,
                height: None,
                scale: Some(Scale(s)),
                ..
            } => ib
                .scale_to(img, None, Some(s * metrics.height() as f64))
                .ok()?,
            Self { width, height, .. } => ib
                .scale_to(img, width.map(|v| v as f64), height.map(|v| v as f64))
                .ok()?,
        };
        let img = match self {
            Self {
                color: Some(Color { r, g, b, .. }),
                alpha: Some(a),
                ..
            } => ib.set_color(&img, Color::from_rgba(*r, *g, *b, *a)).expect("1"),
            Self {
                color: Some(color),
                alpha: None,
                ..
            } => ib.set_color(&img, *color).ok()?,
            Self { alpha: Some(a), .. } => ib.set_opacity(&img, *a).expect("2"),
            _ => img,
        };
        let asc_ratio = metrics.ascent() as f64 / metrics.height() as f64;
        let (w, h) = (img.get_width() * pango::SCALE, img.get_height() * pango::SCALE);
        let y = (h as f64 * asc_ratio) as i32;
        let rect = pango::Rectangle::new(0, -y, w, h);
        attrs.insert(indexed!(pango::AttrShape::new(&rect, &rect); at i, j));
        Some(img)
    }
}

fn parse_abs_or_pt(s: &str) -> Option<i32> {
    let re = Regex::new(r"^([+-]?\d+(.\d+)?)(\s*pt)?$").unwrap();
    let captures = re.captures(s)?;
    let using_pt = captures.get(3).is_some();
    if using_pt {
        let x = f64::from_str(captures.get(1).unwrap().as_str()).unwrap();
        Some((x * pango::SCALE as f64) as i32)
    } else {
        let x = f64::from_str(captures.get(1).unwrap().as_str()).unwrap();
        Some(x as i32)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Size(pub i32);

impl FromStr for Size {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_abs_or_pt(s)
            .map(Self)
            .ok_or("expected an integer, or a number in pt")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scale(pub f64);

impl FromStr for Scale {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "xx-small" => Ok(Self(pango::SCALE_XX_SMALL)),
            "x-small" => Ok(Self(pango::SCALE_X_SMALL)),
            "small" => Ok(Self(pango::SCALE_SMALL)),
            "medium" => Ok(Self(pango::SCALE_MEDIUM)),
            "large" => Ok(Self(pango::SCALE_LARGE)),
            "x-large" => Ok(Self(pango::SCALE_X_LARGE)),
            "xx-large" => Ok(Self(pango::SCALE_XX_LARGE)),
            _ => {
                let re = Regex::new(r"^(\d+(.\d+)?)%$").unwrap();
                let captures = re
                    .captures(s)
                    .ok_or("expected a percentage (e.g. `120%`)")?;
                Ok(Self(
                    f64::from_str(captures.get(1).unwrap().as_str()).unwrap() / 100.0,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Overline(pub pango::Overline);

impl FromStr for Overline {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self(pango::Overline::None)),
            "single" => Ok(Self(pango::Overline::Single)),
            _ => Err("expected one of `none`, `single`"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Underline(pub pango::Underline);

impl FromStr for Underline {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self(pango::Underline::None)),
            "single" => Ok(Self(pango::Underline::Single)),
            "double" => Ok(Self(pango::Underline::Double)),
            "low" => Ok(Self(pango::Underline::Low)),
            "error" => Ok(Self(pango::Underline::Error)),
            _ => Err("expected one of `none`, `single`, `double`, `low`, `error`"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BaselineShift(pub pango::BaselineShift);

impl FromStr for BaselineShift {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self(pango::BaselineShift::None)),
            "superscript" => Ok(Self(pango::BaselineShift::Superscript)),
            "subscript" => Ok(Self(pango::BaselineShift::Subscript)),
            _ => {
                let v = parse_abs_or_pt(s).ok_or(
                    "expected either `none`, `superscript`, `subscript`, \
                    an integer, or a number in pt",
                )?;
                Ok(Self(pango::BaselineShift::__Unknown(v)))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Gravity(pub pango::Gravity);

impl FromStr for Gravity {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "south" => Ok(Self(pango::Gravity::South)),
            "east" => Ok(Self(pango::Gravity::East)),
            "north" => Ok(Self(pango::Gravity::North)),
            "west" => Ok(Self(pango::Gravity::West)),
            "auto" => Ok(Self(pango::Gravity::Auto)),
            _ => Err("expected one of `south`, `east`, `north`, `west`, `auto`"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GravityHint(pub pango::GravityHint);

impl FromStr for GravityHint {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "natural" => Ok(Self(pango::GravityHint::Natural)),
            "strong" => Ok(Self(pango::GravityHint::Strong)),
            "line" => Ok(Self(pango::GravityHint::Line)),
            _ => Err("expected one of `natural`, `strong`, `line`"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ShowFlags(pub pango::ShowFlags);

impl FromStr for ShowFlags {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut flags: pango::ShowFlags = pango::ShowFlags::empty();
        for s in s.split('|') {
            let other = match s {
                "none" => pango::ShowFlags::NONE,
                "spaces" => pango::ShowFlags::SPACES,
                "line-breaks" => pango::ShowFlags::LINE_BREAKS,
                "ignorables" => pango::ShowFlags::IGNORABLES,
                _ => return Err("expected either `none`, `spaces`, `line-breaks`, `ignorables`, separated by `|`"),
            };
            flags.insert(other);
        }
        Ok(Self(flags))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextTransform(pub pango::TextTransform);

impl FromStr for TextTransform {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self(pango::TextTransform::None)),
            "lowercase" => Ok(Self(pango::TextTransform::Lowercase)),
            "uppercase" => Ok(Self(pango::TextTransform::Uppercase)),
            "capitalize" => Ok(Self(pango::TextTransform::Capitalize)),
            _ => Err("expected one of `none`, `lowercase`, `uppercase`, `capitalize`"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Segment {
    Word,
    Sentence,
}

impl FromStr for Segment {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "word" => Ok(Self::Word),
            "sentence" => Ok(Self::Sentence),
            _ => Err("expected one of `word`, `sentence`"),
        }
    }
}
