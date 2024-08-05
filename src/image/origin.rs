//! Origin position parameter.


#[cfg(feature = "cli")]
use serde::{de, Deserialize, Serialize};
use std::ops::Neg;

// TODO: rename this struct?

#[derive(Debug, Copy, Clone, Serialize)]
pub enum Origin {
    Absolute(f64),
    Relative(f64),
}


impl Default for Origin {
    fn default() -> Self {
        Self::Absolute(0.0)
    }
}

impl Neg for Origin {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::Absolute(x) => Self::Absolute(-x),
            Self::Relative(x) => Self::Relative(-x),
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
}

#[cfg(feature = "cli")]
struct OriginVisitor;

macro_rules! visit_int {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
            where
                E: de::Error, {
            match v {
                1 => Ok(Origin::Relative(1.0)),
                -1 => Ok(Origin::Relative(-1.0)),
                _ => Ok(Origin::Absolute(v as f64)),
            }
        }
    };
}

macro_rules! visit_float {
    ($fn:ident $T:ty) => {
        fn $fn<E>(self, v: $T) -> Result<Self::Value, E>
            where
                E: de::Error, {
            Ok(Origin::Relative(v as f64))
        }
    };
}

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