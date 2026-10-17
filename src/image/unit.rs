//! Origin position parameter.

use crate::data::{Access, ValueRef};
use logos::{Lexer, Logos};
use serde::{de, ser, Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub enum Unit {
    Px(f64),
    Pct(f64),
}

#[derive(Debug, Copy, Clone)]
pub enum TextUnit {
    Px(f64),
    Pct(f64),
    Baseline,
}

impl Default for Unit {
    fn default() -> Self {
        Self::Px(0.0)
    }
}

impl Default for TextUnit {
    fn default() -> Self {
        Self::Px(0.0)
    }
}

impl From<Unit> for TextUnit {
    fn from(value: Unit) -> Self {
        match value {
            Unit::Px(v) => Self::Px(v),
            Unit::Pct(v) => Self::Pct(v),
        }
    }
}

impl Unit {
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::Px(x) => *x,
            Self::Pct(a) => a * x,
        }
    }

    pub fn half() -> Self {
        Self::Pct(0.5)
    }

    pub fn full() -> Self {
        Self::Pct(1.0)
    }
}

impl TextUnit {
    pub fn into_unit(&self, h: i32) -> Unit {
        match self {
            Self::Px(x) => Unit::Px(*x),
            Self::Pct(a) => Unit::Pct(*a),
            Self::Baseline => Unit::Px((h / pango::SCALE) as f64),
        }
    }
}

macro_rules! visit_int {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Self::Value::Px(v as f64))
        }
    };
}

macro_rules! visit_float {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Self::Value::Px(v as f64))
        }
    };
}

#[derive(Debug, Logos)]
#[logos(skip r"[ \t\n\f]+")]
enum UnitToken {
    #[regex(r"[+-]?\d+", |lex| lex.slice().parse::<i64>().unwrap())]
    ValInt(i64),
    #[regex(r"[+-]?(\d*\.\d+(e[+-]?\d+)?|\d+e[+-]?\d+)", |lex| lex.slice().parse::<f64>().unwrap(), ignore(case))]
    ValFloat(f64),
    #[token("%")]
    Percent,
    #[token("px")]
    Px,
}

struct UnitVisitor;

impl<'de> de::Visitor<'de> for UnitVisitor {
    type Value = Unit;

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
        let mut lex: Lexer<UnitToken> = Lexer::new(v);
        let v = match lex.next().transpose().map_err(|_| e!())? {
            Some(UnitToken::ValInt(v)) => Ok(v as f64),
            Some(UnitToken::ValFloat(v)) => Ok(v),
            _ => Err(e!()),
        }?;
        match lex.next().transpose().map_err(|_| e!())? {
            Some(UnitToken::Px) => Ok(Unit::Px(v)),
            Some(UnitToken::Percent) => Ok(Unit::Pct(v / 100.0)),
            None => Ok(Unit::Px(v)),
            _ => Err(e!()),
        }
    }
}

impl<'de> Deserialize<'de> for Unit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Unit, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(UnitVisitor)
    }
}

impl Serialize for Unit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Self::Pct(v) => serializer.serialize_str(&format!("{}%", v * 100.0)),
            Self::Px(v) => serializer.serialize_str(&format!("{v}px")),
        }
    }
}

impl Access for Unit {
    fn access(&'_ self, _parts: &[crate::data::PredicatePathPart]) -> ValueRef<'_> {
        ValueRef::Null
    }
}

struct TextUnitVisitor;

impl<'de> de::Visitor<'de> for TextUnitVisitor {
    type Value = TextUnit;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an integer, a float or `baseline`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "baseline" => Ok(Self::Value::Baseline),
            _ => UnitVisitor.visit_str(v).map(|o| o.into()),
        }
    }

    visit_int!(visit_i8 i8);
    visit_int!(visit_i16 i16);
    visit_int!(visit_i32 i32);
    visit_int!(visit_i64 i64);
    visit_float!(visit_f32 f32);
    visit_float!(visit_f64 f64);
}

impl<'de> Deserialize<'de> for TextUnit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<TextUnit, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(TextUnitVisitor)
    }
}

impl Serialize for TextUnit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Self::Baseline => serializer.serialize_str("baseline"),
            Self::Pct(v) => serializer.serialize_str(&format!("{}%", v * 100.0)),
            Self::Px(v) => serializer.serialize_str(&format!("{v}px")),
        }
    }
}

impl Access for TextUnit {
    fn access(&'_ self, _parts: &[crate::data::PredicatePathPart]) -> ValueRef<'_> {
        ValueRef::Null
    }
}
