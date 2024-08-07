//! Text attribute values and conversions.

use crate::error::{Error, Result};
use crate::image::{Color, ImgBackend, Origin};
use crate::text::FontMap;

use libvips::VipsImage;
use regex::Regex;
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ITagAttr {
    pub value: TagAttr,
    pub start_index: u32,
    pub end_index: u32,
}

impl ITagAttr {
    pub fn new(value: TagAttr) -> Self {
        Self {
            value,
            start_index: 0,
            end_index: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TagAttr {
    Span(SpanAttr),
    Img(ImgAttr),
    Icon(IconAttr),
}

macro_rules! attr_parse {
    ($key:expr; $val:expr; $($pat:expr, $att:ident, $T:ty);*) => {{
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
        $vis:vis enum $Attr:ident {
            $( $k:literal => $Variant:ident($T:ty) ),*,
        }
    ) => {
        $(#[$outer])*
        $vis enum $Attr {
            $( $Variant($T) ),*
        }

        impl $Attr {
            pub fn from_key_value(key: &str, value: &str) -> Result<Self> {
                attr_parse!(key; value; $( $k, $Variant, $T );*)
            }
        }
    };
}

macro_rules! struct_attr {
    (
        $(#[$outer:meta])*
        $vis:vis struct $Attr:ident {
            $( $k:literal => $Field:ident: $T:ty ),*
        }
    ) => {
        $(#[$outer])*
        $vis struct $Attr {
            $( pub $Field: Option<$T>, )*
        }

        impl $Attr {
            pub fn push(&mut self, key: &str, value: &str) -> Result<()> {
                match key {
                    $($k => {
                        let parsed = value.parse::<$T>().map_err(|e|
                            Error::TextAttrParseError(key.to_string(), e.to_string())
                        )?;
                        self.$Field = Some(parsed);
                        Ok(())
                    } ) *
                    _ => Err(Error::TextInvalidAttr(key.to_string())),
                }
            }
        }
    };
}

enum_attr! {
    #[derive(Debug, Clone)]
    pub enum SpanAttr {
        "font" => Font(String),
        "features" => Features(String),
        "size" => Size(Points),
        "scale" => Scale(Scale),
        "color" => Color(Color),
        "alpha" => Alpha(f64),
        "bg-color" => BgColor(Color),
        "bg-alpha" => BgAlpha(f64),
        "underline" => Underline(Underline),
        "underline-color" => UnderlineColor(Color),
        "overline" => Overline(Overline),
        "overline-color" => OverlineColor(Color),
        "rise" => Rise(Points),
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
    (AttrFontDesc ($fm:ident($font:ident)) >> $attrs:ident at $i:ident, $j:ident) => {{
        let desc = $fm.get_desc(&$font).ok_or_else(|| Error::FontCacheMiss($font.clone()))?;
        $attrs.insert(indexed!(pango::AttrFontDesc::new(&desc); at $i, $j));
    }};
    ($Attr:ident ($val:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        $attrs.insert(indexed!(pango::$Attr::new($val); at $i, $j))
    }};
    (AttrColor $fn:ident($val:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        let (r, g, b) = $val.pango_rgb();
        $attrs.insert(indexed!(pango::AttrColor::$fn(r, g, b); at $i, $j));
    }};
    (AttrAlpha $fn:ident($val:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        push!(AttrInt $fn(Color::pango_channel($val)) >> $attrs at $i, $j)
    }};
    (AttrColor $fnc:ident $fna:ident($v:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        push!(AttrColor $fnc ($v) >> $attrs at $i, $j);
        if let Some(a) = $v.a {
            push!(AttrAlpha $fna (a) >> $attrs at $i, $j);
        }
    }};
    ($Attr:ident $fn:ident($val:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        $attrs.insert(indexed!(pango::$Attr::$fn($val); at $i, $j))
    }};
    ($T:ident into $Attr:ident $fn:ident($val:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        let v: pango::$T = $val.into();
        $attrs.insert(indexed!(pango::$Attr::$fn(v); at $i, $j))
    }};
    (Segment into AttrInt ($val:expr) >> $attrs:ident at $i:ident, $j:ident) => {{
        match $val {
            Segment::Word => $attrs.insert(indexed!(pango::AttrInt::new_word(); at $i, $j)),
            Segment::Sentence => $attrs.insert(indexed!(pango::AttrInt::new_sentence(); at $i, $j)),
        }
    }};
}

impl SpanAttr {
    pub fn push_pango_attrs(
        self,
        fm: &FontMap,
        attrs: &mut pango::AttrList,
        i: u32,
        j: u32,
    ) -> Result<()> {
        match self {
            Self::Font(x) => push!(AttrFontDesc (fm(x)) >> attrs at i, j),
            Self::Features(x) => push!(AttrFontFeatures (&x) >> attrs at i, j),
            Self::Size(Points(x)) => push!(AttrSize (x) >> attrs at i, j),
            Self::Scale(Scale(x)) => push!(AttrFloat new_scale (x) >> attrs at i, j),
            Self::Color(x) => {
                push!(AttrColor new_foreground new_foreground_alpha (x) >> attrs at i, j)
            }
            Self::Alpha(a) => push!(AttrAlpha new_foreground_alpha (a) >> attrs at i, j),
            Self::BgColor(x) => {
                push!(AttrColor new_background new_background_alpha (x) >> attrs at i, j)
            }
            Self::BgAlpha(x) => push!(AttrAlpha new_background_alpha (x) >> attrs at i, j),
            Self::Underline(x) => push!(Underline into AttrInt new_underline (x) >> attrs at i, j),
            Self::UnderlineColor(x) => push!(AttrColor new_underline_color (x) >> attrs at i, j),
            Self::Overline(x) => push!(Overline into AttrInt new_overline (x) >> attrs at i, j),
            Self::OverlineColor(x) => push!(AttrColor new_overline_color (x) >> attrs at i, j),
            Self::Rise(Points(x)) => push!(AttrInt new_rise (x) >> attrs at i, j),
            Self::BaselineShift(x) => {
                push!(BaselineShift into AttrInt new_baseline_shift (x) >> attrs at i, j)
            }
            Self::Strikethrough(x) => push!(AttrInt new_strikethrough (x) >> attrs at i, j),
            Self::StrikethroughColor(x) => {
                push!(AttrColor new_strikethrough_color (x) >> attrs at i, j)
            }
            Self::Fallback(x) => push!(AttrInt new_fallback (x) >> attrs at i, j),
            Self::Lang(x) => {
                push!(AttrLanguage (&pango::Language::from_string(&x)) >> attrs at i, j)
            }
            Self::LetterSpacing(x) => push!(AttrInt new_letter_spacing (x) >> attrs at i, j),
            Self::Gravity(x) => push!(Gravity into AttrInt new_gravity (x) >> attrs at i, j),
            Self::GravityHint(x) => {
                push!(GravityHint into AttrInt new_gravity_hint (x) >> attrs at i, j)
            }
            Self::Show(ShowFlags(x)) => push!(AttrInt new_show (x) >> attrs at i, j),
            Self::InsertHyphens(x) => push!(AttrInt new_insert_hyphens (x) >> attrs at i, j),
            Self::AllowBreaks(x) => push!(AttrInt new_allow_breaks (x) >> attrs at i, j),
            Self::LineHeight(x) => push!(AttrFloat new_line_height (x) >> attrs at i, j),
            Self::TextTransform(x) => {
                push!(TextTransform into AttrInt new_text_transform (x) >> attrs at i, j)
            }
            Self::Segment(x) => push!(Segment into AttrInt (x) >> attrs at i, j),
        }
        Ok(())
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
        "font" => font: String,
        "size" => size: i32,
        "gravity" => gravity: Gravity
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
        "font" => font: String,
        "size" => size: i32,
        "gravity" => gravity: Gravity
    }
}

macro_rules! set_if_none {
    ($self:ident.$field:ident = $val:expr) => {
        if $self.$field.is_none() {
            $self.$field = Some($val);
        }
    };
}

impl ImgAttr {
    #[must_use]
    pub fn configured(
        mut self,
        font: &str,
        size: i32,
        scale: f64,
        alpha: f64,
        gravity: Gravity,
    ) -> Self {
        set_if_none!(self.font = font.to_string());
        set_if_none!(self.size = size);
        set_if_none!(self.scale = Scale(scale));
        set_if_none!(self.alpha = alpha);
        set_if_none!(self.gravity = gravity);
        self
    }

    pub fn push_pango_attrs(
        self,
        ib: &mut ImgBackend,
        prefix: Option<&PathBuf>,
        fm: &FontMap,
        ctx: &pango::Context,
        attrs: &mut pango::AttrList,
        i: u32,
        j: u32,
    ) -> Option<VipsImage> {
        let fp = img_src_fp(prefix, self.src.as_ref()?);
        let fp = &fp.to_string_lossy();
        ib.cache(fp).ok()?;
        let (cached_img, new_img) = open_img(ib, fp);
        let img = cached_img.or(new_img.as_ref())?;
        let img = rotate_img(ib, img, self.gravity.unwrap_or(Gravity::South))?;
        let metrics = get_metrics(fm, ctx, self.font.as_ref()?, self.size?)?;
        let img = resize_img(ib, &img, &metrics, self.width, self.height, self.scale)?;
        let img = recolor_img(ib, img, None, self.alpha)?;
        push_img_rect(attrs, i, j, &img, &metrics);
        Some(img)
    }
}

impl IconAttr {
    #[must_use]
    pub fn configured(
        mut self,
        font: &str,
        size: i32,
        scale: f64,
        color: Color,
        alpha: f64,
        gravity: Gravity,
    ) -> Self {
        set_if_none!(self.font = font.to_string());
        set_if_none!(self.size = size);
        set_if_none!(self.color = color);
        set_if_none!(self.scale = Scale(scale));
        set_if_none!(self.alpha = alpha);
        set_if_none!(self.gravity = gravity);
        self
    }

    pub fn push_pango_attrs(
        &self,
        ib: &mut ImgBackend,
        prefix: Option<&PathBuf>,
        fm: &FontMap,
        ctx: &pango::Context,
        attrs: &mut pango::AttrList,
        i: u32,
        j: u32,
    ) -> Option<VipsImage> {
        let fp = img_src_fp(prefix, self.src.as_ref()?);
        let fp = &fp.to_string_lossy();
        ib.cache(fp).ok()?;
        let (cached_img, new_img) = open_img(ib, fp);
        let img = cached_img.or(new_img.as_ref())?;
        let img = rotate_img(ib, img, self.gravity.unwrap_or(Gravity::South))?;
        let metrics = get_metrics(fm, ctx, self.font.as_ref()?, self.size?)?;
        let img = resize_img(ib, &img, &metrics, self.width, self.height, self.scale)?;
        let img = recolor_img(ib, img, self.color, self.alpha)?;
        push_img_rect(attrs, i, j, &img, &metrics);
        Some(img)
    }
}

fn img_src_fp(prefix: Option<&PathBuf>, src: &str) -> PathBuf {
    let mut fp = prefix.cloned().unwrap_or_else(|| PathBuf::new());
    fp.push(src);
    fp
}

fn open_img<'i>(ib: &'i ImgBackend, src: &str) -> (Option<&'i VipsImage>, Option<VipsImage>) {
    let cached_img = ib.get_cached(&src).ok();
    let new_img = if cached_img.is_none() {
        ib.open(&src).ok()
    } else {
        None
    };
    (cached_img, new_img)
}

fn get_metrics(
    fm: &FontMap,
    ctx: &pango::Context,
    font: &String,
    size: i32,
) -> Option<pango::FontMetrics> {
    let desc = fm.get_desc_abs(font, size)?;
    Some(ctx.metrics(Some(&desc), None))
}

fn rotate_img(ib: &ImgBackend, img: &VipsImage, gravity: Gravity) -> Option<VipsImage> {
    let (img, _, _) = match gravity {
        Gravity::North => ib.rotate(&img, 180.0, Origin::default(), Origin::default()).ok()?,
        Gravity::East => ib.rotate(&img, -90.0, Origin::default(), Origin::default()).ok()?,
        Gravity::West => ib.rotate(&img, 90.0, Origin::default(), Origin::default()).ok()?,
        _ => ib.rotate(&img, 0.0, Origin::default(), Origin::default()).ok()?,
    };
    Some(img)
}

fn resize_img(
    ib: &ImgBackend,
    img: &VipsImage,
    metrics: &pango::FontMetrics,
    width: Option<i32>,
    height: Option<i32>,
    scale: Option<Scale>,
) -> Option<VipsImage> {
    match (width, height, scale) {
        (None, None, Some(Scale(s))) => ib
            .scale_to(
                img,
                None,
                Some(s * (metrics.height() / pango::SCALE) as f64),
            )
            .ok(),
        (width, height, _) => ib
            .scale_to(img, width.map(|v| v as f64), height.map(|v| v as f64))
            .ok(),
    }
}

fn recolor_img(
    ib: &ImgBackend,
    img: VipsImage,
    color: Option<Color>,
    alpha: Option<f64>,
) -> Option<VipsImage> {
    match (color, alpha) {
        (Some(Color { r, g, b, .. }), Some(a)) => {
            ib.set_color(&img, Color::from_rgba(r, g, b, a)).ok()
        }
        (Some(color), None) => ib.set_color(&img, color).ok(),
        (_, Some(a)) => ib.set_opacity(&img, a).ok(),
        _ => Some(img),
    }
}

fn push_img_rect(
    attrs: &mut pango::AttrList,
    i: u32,
    j: u32,
    img: &VipsImage,
    metrics: &pango::FontMetrics,
) {
    let asc_ratio = metrics.ascent() as f64 / metrics.height() as f64;
    let (w, h) = (
        img.get_width() * pango::SCALE,
        img.get_height() * pango::SCALE,
    );
    let y = (h as f64 * asc_ratio) as i32;
    let rect = pango::Rectangle::new(0, -y, w, h);
    attrs.insert(indexed!(pango::AttrShape::new(&rect, &rect); at i, j));
}

#[derive(Debug, Copy, Clone)]
pub struct Points(pub i32);

impl FromStr for Points {
    type Err = &'static str;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse_abs_or_pt(s)
            .map(Self)
            .ok_or("expected an integer, or a number in pt")
    }
}

impl Points {
    fn parse_abs_or_pt(s: &str) -> Option<i32> {
        let re = Regex::new(r"^([+-]?\d+(.\d+)?)(\s*pt)?$").unwrap();
        let captures = re.captures(s)?;
        if let Some(_) = captures.get(3) {
            let x = f64::from_str(captures.get(1).unwrap().as_str()).unwrap();
            Some((x * pango::SCALE as f64) as i32)
        } else {
            let x = f64::from_str(captures.get(1).unwrap().as_str()).unwrap();
            Some(x as i32)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scale(pub f64);

impl FromStr for Scale {
    type Err = &'static str;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

macro_rules! into_pango {
    (
        $(#[$outer:meta])*
        $vis:vis enum $Enum:ident {
            $( $key:literal => $Variant:ident ),*
        }
    ) => {
        $(#[$outer])*
        $vis enum $Enum {
            $( $Variant ),*
        }

        impl FromStr for $Enum {
            type Err = &'static str;
            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                match s {
                    $( $key => Ok(Self::$Variant), )*
                    _ => Err(
                        concat!("expected one of", $(" `", $key, "`"),*)
                    ),
                }
            }
        }

        impl Into<pango::$Enum> for $Enum {
            fn into(self) -> pango::$Enum {
                match self {
                    $( Self::$Variant => pango::$Enum::$Variant ),*
                }
            }
        }
    };
    (
        $(#[$outer:meta])*
        $vis:vis enum $Enum:ident {
            $( $Variant:ident ),*
        }
    ) => {
        $(#[$outer])*
        $vis enum $Enum {
            $( $Variant ),*
        }

        impl Into<pango::$Enum> for $Enum {
            fn into(self) -> pango::$Enum {
                match self {
                    $( Self::$Variant => pango::$Enum::$Variant ),*
                }
            }
        }
    };
}

into_pango! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum Underline {
        "none" => None,
        "single" => Single,
        "double" => Double,
        "low" => Low,
        "error" => Error
    }
}

into_pango! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum Overline {
        "none" => None,
        "single" => Single
    }
}

into_pango! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum BaselineShift {
        "none" => None,
        "superscript" => Superscript,
        "subscript" => Subscript
    }
}

into_pango! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum Gravity {
        "south" => South,
        "east" => East,
        "north" => North,
        "west" => West,
        "auto" => Auto
    }
}

impl From<pango::Gravity> for Gravity {
    fn from(value: pango::Gravity) -> Self {
        match value {
            pango::Gravity::South => Self::South,
            pango::Gravity::East => Self::East,
            pango::Gravity::North => Self::North,
            pango::Gravity::West => Self::West,
            _ => Self::Auto,
        }
    }
}

into_pango! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum GravityHint {
        "natural" => Natural,
        "strong" => Strong,
        "line" => Line
    }
}

into_pango! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum TextTransform {
        "none" => None,
        "lowercase" => Lowercase,
        "uppercase" => Uppercase,
        "capitalize" => Capitalize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ShowFlags(pub pango::ShowFlags);

impl FromStr for ShowFlags {
    type Err = &'static str;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

impl Into<pango::ShowFlags> for ShowFlags {
    fn into(self) -> pango::ShowFlags {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
pub enum Segment {
    Word,
    Sentence,
}

impl FromStr for Segment {
    type Err = &'static str;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "word" => Ok(Self::Word),
            "sentence" => Ok(Self::Sentence),
            _ => Err("expected one of `word`, `sentence`"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LayoutAttr<'a> {
    Alignment(Alignment),
    AutoDir(bool),
    Dpi(f64),
    Direction(Direction),
    Gravity(Gravity),
    GravityHint(GravityHint),
    Indent(f64),
    Justify(bool),
    Language(&'a str),
    LineSpacing(f64),
    Spacing(f64),
    Width(i32),
    Wrap(WrapMode),
}

into_pango! {
    #[derive(Debug, Copy, Clone)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum Direction {
        Ltr,
        Rtl,
        TtbLtr,
        TtbRtl,
        WeakLtr,
        WeakRtl,
        Neutral
    }
}

into_pango! {
    #[derive(Debug, Copy, Clone)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum Alignment {
        Left,
        Center,
        Right
    }
}

into_pango! {
    #[derive(Debug, Copy, Clone)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
    pub enum WrapMode {
        Word,
        Char,
        WordChar
    }
}

impl<'a> LayoutAttr<'a> {
    pub fn configure(&self, ctx: &pango::Context, layout: &pango::Layout) {
        match self {
            Self::Dpi(x) => pangocairo::functions::context_set_resolution(&ctx, *x),
            Self::Direction(x) => ctx.set_base_dir((*x).into()),
            Self::Gravity(x) => ctx.set_base_gravity((*x).into()),
            Self::GravityHint(x) => ctx.set_gravity_hint((*x).into()),
            Self::Language(x) => ctx.set_language(Some(&pango::Language::from_string(x))),
            Self::Alignment(x) => layout.set_alignment((*x).into()),
            Self::AutoDir(x) => layout.set_auto_dir(*x),
            Self::Indent(x) => layout.set_indent((x * pango::SCALE as f64) as i32),
            Self::Justify(x) => layout.set_justify(*x),
            Self::LineSpacing(x) => layout.set_line_spacing(*x as f32),
            Self::Spacing(x) => layout.set_spacing((x * pango::SCALE as f64) as i32),
            Self::Width(x) => layout.set_width(x * pango::SCALE),
            Self::Wrap(x) => layout.set_wrap((*x).into()),
        }
    }
}
