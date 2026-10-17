//! Origin position parameter.

use crate::data::{Access, ValueRef};
use logos::{Lexer, Logos};
use serde::{de, ser, Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub enum Origin {
    Absolute(f64),
    Relative(f64),
}

#[derive(Debug, Copy, Clone)]
pub enum TextOrigin {
    Absolute(f64),
    Relative(f64),
    Baseline,
}

impl Default for Origin {
    fn default() -> Self {
        Self::Absolute(0.0)
    }
}

impl Default for TextOrigin {
    fn default() -> Self {
        Self::Absolute(0.0)
    }
}

impl From<Origin> for TextOrigin {
    fn from(value: Origin) -> Self {
        match value {
            Origin::Absolute(v) => Self::Absolute(v),
            Origin::Relative(v) => Self::Relative(v),
        }
    }
}

impl Origin {
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::Absolute(x) => *x,
            Self::Relative(a) => a * x,
        }
    }

    pub fn half() -> Self {
        Self::Relative(0.5)
    }

    pub fn full() -> Self {
        Self::Relative(1.0)
    }
}

impl TextOrigin {
    pub fn into_origin(&self, h: i32) -> Origin {
        match self {
            Self::Absolute(x) => Origin::Absolute(*x),
            Self::Relative(a) => Origin::Relative(*a),
            Self::Baseline => Origin::Absolute((h / pango::SCALE) as f64),
        }
    }
}

macro_rules! visit_int {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Self::Value::Absolute(v as f64))
        }
    };
}

macro_rules! visit_float {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Self::Value::Absolute(v as f64))
        }
    };
}

#[derive(Debug, Logos)]
#[logos(skip r"[ \t\n\f]+")]
enum OriginToken {
    #[regex(r"[+-]?\d+", |lex| lex.slice().parse::<i64>().unwrap())]
    ValInt(i64),
    #[regex(r"[+-]?(\d*\.\d+(e[+-]?\d+)?|\d+e[+-]?\d+)", |lex| lex.slice().parse::<f64>().unwrap(), ignore(case))]
    ValFloat(f64),
    #[token("%")]
    Percent,
    #[token("px")]
    Px,
}

struct OriginVisitor;

impl<'de> de::Visitor<'de> for OriginVisitor {
    type Value = Origin;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an integer or a float")
    }

    visit_int!(visit_i8 i8);
    visit_int!(visit_i16 i16);
    visit_int!(visit_i32 i32);
    visit_int!(visit_i64 i64);
    visit_float!(visit_f32 f32);
    visit_float!(visit_f64 f64);

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        macro_rules! e {
            () => {
                E::custom(
                    "expected a number, optionally followed by relative (%) or absolute (px) unit",
                )
            };
        }
        let mut lex: Lexer<OriginToken> = Lexer::new(v);
        let v = match lex.next().transpose().map_err(|_| e!())? {
            Some(OriginToken::ValInt(v)) => Ok(v as f64),
            Some(OriginToken::ValFloat(v)) => Ok(v),
            _ => Err(e!()),
        }?;
        match lex.next().transpose().map_err(|_| e!())? {
            Some(OriginToken::Px) => Ok(Origin::Absolute(v)),
            Some(OriginToken::Percent) => Ok(Origin::Relative(v / 100.0)),
            None => Ok(Origin::Absolute(v)),
            _ => Err(e!()),
        }
    }
}

impl<'de> Deserialize<'de> for Origin {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Origin, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(OriginVisitor)
    }
}

impl Serialize for Origin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Self::Relative(v) => serializer.serialize_str(&format!("{}%", v * 100.0)),
            Self::Absolute(v) => serializer.serialize_str(&format!("{v}px")),
        }
    }
}

impl Access for Origin {
    fn access(&'_ self, _parts: &[crate::data::PredicatePathPart]) -> ValueRef<'_> {
        ValueRef::Null
    }
}

struct TextOriginVisitor;

impl<'de> de::Visitor<'de> for TextOriginVisitor {
    type Value = TextOrigin;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an integer, a float or `baseline`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "baseline" => Ok(Self::Value::Baseline),
            _ => OriginVisitor.visit_str(v).map(|o| o.into()),
        }
    }

    visit_int!(visit_i8 i8);
    visit_int!(visit_i16 i16);
    visit_int!(visit_i32 i32);
    visit_int!(visit_i64 i64);
    visit_float!(visit_f32 f32);
    visit_float!(visit_f64 f64);
}

impl<'de> Deserialize<'de> for TextOrigin {
    fn deserialize<D>(deserializer: D) -> std::result::Result<TextOrigin, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(TextOriginVisitor)
    }
}

impl Serialize for TextOrigin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Self::Baseline => serializer.serialize_str("baseline"),
            Self::Relative(v) => serializer.serialize_str(&format!("{}%", v * 100.0)),
            Self::Absolute(v) => serializer.serialize_str(&format!("{v}px")),
        }
    }
}

impl Access for TextOrigin {
    fn access(&'_ self, _parts: &[crate::data::PredicatePathPart]) -> ValueRef<'_> {
        ValueRef::Null
    }
}
