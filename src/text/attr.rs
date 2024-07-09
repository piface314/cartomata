//! Text attribute values and conversions.

use crate::color::Color;

use regex::Regex;
use std::str::FromStr;

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

macro_rules! attr {
    (
        $(#[$outer:meta])*
        $vis:vis enum $Attr:ident {
            $( $k:expr => $Variant:ident($T:ty) ),*
        }
    ) => {
        $(#[$outer])*
        $vis enum $Attr {
            $( $Variant($T) ),*
        }

        impl $Attr {
            pub fn from_key_value(key: &str, value: &str) -> crate::error::Result<Self> {
                attr_parse!(key; value; $( $k, $Variant, $T );*)
            }
        }
    };
}

attr! {
    #[derive(Debug)]
    pub enum SpanAttr {
        "font" => Font(String),
        "features" => Features(String),
        "size" => Size(Size),
        "scale" => Scale(Scale),
        "color" => Color(Color),
        "alpha" => Alpha(u8),
        "bg-color" => BgColor(Color),
        "bg-alpha" => BgAlpha(u8),
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
        "segment" => Segment(Segment)
    }
}


attr! {
    #[derive(Debug)]
    pub enum ImgAttr {
        "src" => Src(String),
        "width" => Width(u32),
        "height" => Height(u32),
        "scale" => Scale(Scale)
    }
}


attr! {
    #[derive(Debug)]
    pub enum IconAttr {
        "src" => Src(String),
        "width" => Width(u32),
        "height" => Height(u32),
        "scale" => Scale(Scale),
        "color" => Color(Color),
        "alpha" => Alpha(u8)
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

#[derive(Debug)]
pub struct Size(pub i32);

impl FromStr for Size {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_abs_or_pt(s)
            .map(Self)
            .ok_or("expected an integer, or a number in pt")
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum Segment {
    Word(pango::AttrInt),
    Sentence(pango::AttrInt),
}

impl FromStr for Segment {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "word" => Ok(Self::Word(pango::AttrInt::new_word())),
            "sentence" => Ok(Self::Sentence(pango::AttrInt::new_sentence())),
            _ => Err("expected one of `word`, `sentence`"),
        }
    }
}
