//! Contains representations for card data.

use serde::de::{self, DeserializeOwned, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::collections::HashMap;
pub use cartomata_derive::Card;

pub type Schema = HashMap<String, Type>;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string, i64, f64 or bool")
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
        Ok(Value::Int(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
        Ok(Value::Int(v as i64))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
        Ok(Value::Float(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        Ok(Value::String(v.to_string()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
        Ok(Value::Bool(v))
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
            Value::Nil => write!(f, ""),
        }
    }
}

pub trait Card: DeserializeOwned {
    fn id(&self) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct DynCard(HashMap<String, Value>);

impl Card for DynCard {
    fn id(&self) -> Option<String> {
        match self.0.get("id") {
            Some(v) => Some(v.to_string()),
            None => None,
        }
    }
}

struct DynCardVisitor;

impl<'de> Visitor<'de> for DynCardVisitor {
    type Value = DynCard;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> {
        let mut items = HashMap::new();
        while let Some((k, v)) = map.next_entry::<String, Value>()? {
            items.insert(k, v);
        }
        Ok(DynCard(items))
    }
}

impl<'de> Deserialize<'de> for DynCard {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DynCardVisitor)
    }
}
