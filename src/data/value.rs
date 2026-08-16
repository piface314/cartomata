use itertools::Itertools;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;

/// Represents the possible values a card field can take.
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Seq(Vec<Value>),
    Map(HashMap<String, Value>),
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
        Value::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value::Seq(value.into_iter().map(|v| v.into()).collect())
    }
}

impl<T: Into<Value>> From<HashMap<String, T>> for Value {
    fn from(value: HashMap<String, T>) -> Self {
        Value::Map(value.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map(|v| v.into()).unwrap_or_default()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Int(a), Self::Float(b)) => *a as f64 == *b,
            (Self::Float(a), Self::Int(b)) => *a == *b as f64,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::String(a), Self::Int(b)) => a.parse::<i64>().map(|a| a == *b).unwrap_or(false),
            (Self::String(a), Self::Float(b)) => a.parse::<f64>().map(|a| a == *b).unwrap_or(false),
            (Self::String(a), Self::Bool(b)) => a.parse::<bool>().map(|a| a == *b).unwrap_or(false),
            (Self::Int(a), Self::String(b)) => b.parse::<i64>().map(|b| *a == b).unwrap_or(false),
            (Self::Float(a), Self::String(b)) => b.parse::<f64>().map(|b| *a == b).unwrap_or(false),
            (Self::Bool(a), Self::String(b)) => b.parse::<bool>().map(|b| *a == b).unwrap_or(false),
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Seq(a), Self::Seq(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
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
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::String(a), Self::Int(b)) => {
                a.parse::<i64>().map(|a| a.partial_cmp(b)).unwrap_or(None)
            }
            (Self::String(a), Self::Float(b)) => {
                a.parse::<f64>().map(|a| a.partial_cmp(b)).unwrap_or(None)
            }
            (Self::String(a), Self::Bool(b)) => {
                a.parse::<bool>().map(|a| a.partial_cmp(b)).unwrap_or(None)
            }
            (Self::Int(a), Self::String(b)) => {
                b.parse::<i64>().map(|b| a.partial_cmp(&b)).unwrap_or(None)
            }
            (Self::Float(a), Self::String(b)) => {
                b.parse::<f64>().map(|b| a.partial_cmp(&b)).unwrap_or(None)
            }
            (Self::Bool(a), Self::String(b)) => {
                b.parse::<bool>().map(|b| a.partial_cmp(&b)).unwrap_or(None)
            }
            (Self::Bool(a), Self::Bool(b)) => a.partial_cmp(b),
            (Self::Seq(a), Self::Seq(b)) => a.partial_cmp(b),
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
        formatter.write_str("a string, int, float, bool, seq, map or none")
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
        Ok(Value::String(v.to_string()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(elem) = seq.next_element::<Value>()? {
            values.push(elem);
        }
        Ok(Value::Seq(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut values = HashMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let elem = map.next_value::<Value>()?;
            values.insert(key, elem);
        }
        Ok(Value::Map(values))
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
            Value::String(v) => write!(f, "{v}"),
            Value::Seq(seq) => {
                let contents = seq.iter().map(|v| format!("{v:?}")).join(", ");
                write!(f, "[{contents}]")
            }
            Value::Map(map) => {
                let contents = map.iter().map(|(k, v)| format!("{k}: {v:?}")).join(", ");
                write!(f, "{{{contents}}}")
            }
            Value::Nil => write!(f, ""),
        }
    }
}
