//! Origin position parameter.


#[cfg(feature = "cli")]
use serde::{de, Deserialize, Serialize};

// TODO: rename this struct?

#[derive(Debug, Copy, Clone, Serialize)]
pub enum Origin {
    Absolute(f64),
    Relative(f64),
}

#[derive(Debug, Copy, Clone, Serialize)]
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

impl Origin {
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::Absolute(x) => *x,
            Self::Relative(a) => a * x,
        }
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
                E: de::Error, {
            match v {
                1 => Ok(Self::Value::Relative(1.0)),
                -1 => Ok(Self::Value::Relative(-1.0)),
                _ => Ok(Self::Value::Absolute(v as f64)),
            }
        }
    };
}

macro_rules! visit_float {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
            where
                E: de::Error, {
            Ok(Self::Value::Relative(v as f64))
        }
    };
}

#[cfg(feature = "cli")]
struct OriginVisitor;

#[cfg(feature = "cli")]
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
}

impl<'de> Deserialize<'de> for Origin {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Origin, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(OriginVisitor)
    }
}

#[cfg(feature = "cli")]
struct TextOriginVisitor;

#[cfg(feature = "cli")]
impl<'de> de::Visitor<'de> for TextOriginVisitor {
    type Value = TextOrigin;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an integer, a float or `baseline`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error, {
        match v {
            "baseline" => Ok(Self::Value::Baseline),
            _ => Err(E::custom(format!("unknown origin value {v:?}")))
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