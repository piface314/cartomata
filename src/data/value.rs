use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;


/// Represents the possible values a card field can take.
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
}

impl Default for Value {
    fn default() -> Self {
        Self::Nil
    }
}

macro_rules! value_from {
    ($($V:ty)+ => $Variant:ident($T:ty)) => {
        $(
            impl From<$V> for Value {
                fn from(value: $V) -> Self {
                    Self::$Variant(value as $T)
                }
            }
        )*
    };
}

value_from!(i64 i32 i16 i8 u64 u32 u16 u8 => Int(i64));
value_from!(f64 f32 => Float(f64));
value_from!(bool => Bool(bool));

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Str(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(value)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Int(a), Self::Float(b)) => *a as f64 == *b,
            (Self::Float(a), Self::Int(b)) => *a == *b as f64,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Str(a), Self::Int(b)) => a.parse::<i64>().map(|a| a == *b).unwrap_or(false),
            (Self::Str(a), Self::Float(b)) => a.parse::<f64>().map(|a| a == *b).unwrap_or(false),
            (Self::Str(a), Self::Bool(b)) => a.parse::<bool>().map(|a| a == *b).unwrap_or(false),
            (Self::Int(a), Self::Str(b)) => b.parse::<i64>().map(|b| *a == b).unwrap_or(false),
            (Self::Float(a), Self::Str(b)) => b.parse::<f64>().map(|b| *a == b).unwrap_or(false),
            (Self::Bool(a), Self::Str(b)) => b.parse::<bool>().map(|b| *a == b).unwrap_or(false),
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Nil, Self::Nil) => true,
            (_, _) => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a.partial_cmp(b),
            (Self::Int(a), Self::Float(b)) => (*a as f64).partial_cmp(b),
            (Self::Float(a), Self::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Self::Float(a), Self::Float(b)) => a.partial_cmp(b),
            (Self::Str(a), Self::Str(b)) => a.partial_cmp(b),
            (Self::Str(a), Self::Int(b)) => {
                a.parse::<i64>().map(|a| a.partial_cmp(b)).unwrap_or(None)
            }
            (Self::Str(a), Self::Float(b)) => {
                a.parse::<f64>().map(|a| a.partial_cmp(b)).unwrap_or(None)
            }
            (Self::Str(a), Self::Bool(b)) => {
                a.parse::<bool>().map(|a| a.partial_cmp(b)).unwrap_or(None)
            }
            (Self::Int(a), Self::Str(b)) => {
                b.parse::<i64>().map(|b| a.partial_cmp(&b)).unwrap_or(None)
            }
            (Self::Float(a), Self::Str(b)) => {
                b.parse::<f64>().map(|b| a.partial_cmp(&b)).unwrap_or(None)
            }
            (Self::Bool(a), Self::Str(b)) => {
                b.parse::<bool>().map(|b| a.partial_cmp(&b)).unwrap_or(None)
            }
            (Self::Bool(a), Self::Bool(b)) => a.partial_cmp(b),
            (_, _) => None,
        }
    }
}

struct ValueVisitor;

macro_rules! visit {
    ($fn:ident $S:ty => $Variant:ident($T:ty)) => {
        fn $fn<E: de::Error>(self, v: $S) -> std::result::Result<Self::Value, E> {
            Ok(Value::$Variant(v as $T))
        }
    };
    ($fn:ident $S:ty => $Variant:ident) => {
        fn $fn<E: de::Error>(self, v: $S) -> std::result::Result<Self::Value, E> {
            Ok(Value::$Variant(v))
        }
    };
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string, int, float, bool or none")
    }

    visit!(visit_i64 i64 => Int);
    visit!(visit_i32 i32 => Int(i64));
    visit!(visit_i16 i16 => Int(i64));
    visit!(visit_i8  i8  => Int(i64));
    visit!(visit_u64 u64 => Int(i64));
    visit!(visit_u32 u32 => Int(i64));
    visit!(visit_u16 u16 => Int(i64));
    visit!(visit_u8  u8  => Int(i64));

    visit!(visit_f64 f64 => Float);
    visit!(visit_f32 f32 => Float(f64));

    visit!(visit_bool bool => Bool);

    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        Ok(Value::Str(v.to_string()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
        Ok(Value::Str(v))
    }

    fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(Value::Nil)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(v) => write!(f, "{v}"),
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Str(v) => write!(f, "{v}"),
            Value::Nil => write!(f, ""),
        }
    }
}
