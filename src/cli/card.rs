//! Contains representations for card data.

use crate::data::{Card, Value};
#[cfg(feature = "diff")]
use crate::diff::DiffHash;
use itertools::Itertools;
use md5::digest::Update;
use mlua::{IntoLua, Lua, Result as LuaResult, Value as LuaValue};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;

impl<'lua> IntoLua<'lua> for Value {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self {
            Value::Bool(v) => Ok(LuaValue::Boolean(v)),
            Value::Int(v) => Ok(LuaValue::Integer(v)),
            Value::Float(v) => Ok(LuaValue::Number(v)),
            Value::Str(v) => lua.create_string(v.as_bytes()).map(LuaValue::String),
            Value::Nil => Ok(LuaValue::Nil),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynCard(pub HashMap<String, Value>);

impl Card for DynCard {
    fn get(&self, field: &str) -> Value {
        self.0.get(field).cloned().unwrap_or_default()
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

impl<'de> Deserialize<'de> for DynCard {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DynCardVisitor)
    }
}

#[cfg(feature = "diff")]
impl DiffHash for DynCard {
    fn diff_hash(&self, state: &mut md5::Md5) {
        for (k, v) in self.0.iter().sorted_by_key(|pair| pair.0) {
            state.update(k.as_bytes());
            state.update(b":");
            state.update(format!("{v:?}").as_bytes());
        }
    }
}
