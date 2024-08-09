//! Contains representations for card data.

use crate::data::Card;
use mlua::{IntoLua, Lua, Result as LuaResult, Value as LuaValue};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
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

impl<'lua> IntoLua<'lua> for Value {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self {
            Value::Bool(v) => Ok(LuaValue::Boolean(v)),
            Value::Int(v) => Ok(LuaValue::Integer(v)),
            Value::Float(v) => Ok(LuaValue::Number(v)),
            Value::String(v) => lua.create_string(v.as_bytes()).map(LuaValue::String),
            Value::Nil => Ok(LuaValue::Nil),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynCard(pub HashMap<String, Value>);

impl Card for DynCard {
    fn get_int(&self, field: &str) -> Option<i64> {
        match self.0.get(field) {
            Some(Value::Int(x)) => Some(*x),
            Some(Value::Float(x)) => Some(*x as i64),
            Some(Value::Bool(x)) => Some(*x as i64),
            Some(Value::String(x)) => x.parse::<i64>().ok(),
            _ => None,
        }
    }

    fn get_float(&self, field: &str) -> Option<f64> {
        match self.0.get(field) {
            Some(Value::Int(x)) => Some(*x as f64),
            Some(Value::Float(x)) => Some(*x),
            Some(Value::Bool(x)) => Some(if *x {1.0} else {0.0}),
            Some(Value::String(x)) => x.parse::<f64>().ok(),
            _ => None,
        }
    }

    fn get_bool(&self, field: &str) -> Option<bool> {
        match self.0.get(field) {
            Some(Value::Int(x)) => Some(*x != 0),
            Some(Value::Float(x)) => Some(*x != 0.0),
            Some(Value::Bool(x)) => Some(*x),
            Some(Value::String(x)) => x.parse::<bool>().ok(),
            _ => None,
        }
    }

    fn get_string(&self, field: &str) -> Option<String> {
        match self.0.get(field) {
            Some(v) => Some(v.to_string()),
            _ => None,
        }
    }
}

struct DynCardVisitor;

impl<'de> Visitor<'de> for DynCardVisitor {
    type Value = DynCard;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: de::MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut items = HashMap::new();
        while let Some((k, v)) = map.next_entry::<String, Value>()? {
            items.insert(k, v);
        }
        Ok(DynCard(items))
    }
}

#[cfg(feature = "cli")]
impl<'de> Deserialize<'de> for DynCard {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DynCardVisitor)
    }
}
