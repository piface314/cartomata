//! Contains representations for card data.

use crate::data::{Access, Value, ValueRef};
#[cfg(feature = "diff")]
use crate::diff::DiffHash;
#[cfg(feature = "diff")]
use itertools::Itertools;
#[cfg(feature = "diff")]
use md5::digest::Update;
use mlua::{IntoLua, Lua, Result as LuaResult, Value as LuaValue};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;

impl<'lua, 'v> IntoLua<'lua> for &ValueRef<'v> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self {
            ValueRef::Bool(v) => v.into_lua(lua),
            ValueRef::Number(x) => {
                if let Some(v) = x.as_f64() {
                    v.into_lua(lua)
                } else if let Some(v) = x.as_u64() {
                    v.into_lua(lua)
                } else if let Some(v) = x.as_i64() {
                    v.into_lua(lua)
                } else {
                    Ok(LuaValue::Nil)
                }
            }
            ValueRef::String(v) => v.into_lua(lua),
            ValueRef::Array(seq) => {
                let table = lua.create_table()?;
                for (index, elem) in seq.iter().enumerate() {
                    table.set(index + 1, elem)?;
                }
                Ok(LuaValue::Table(table))
            }
            ValueRef::ArrayRef(seq) => {
                let table = lua.create_table()?;
                for (index, elem) in seq.iter().enumerate() {
                    table.set(index + 1, ValueRef::from(elem))?;
                }
                Ok(LuaValue::Table(table))
            }
            ValueRef::Object(map) => {
                let table = lua.create_table()?;
                for (key, elem) in map.iter() {
                    table.set(*key, elem)?;
                }
                Ok(LuaValue::Table(table))
            }
            ValueRef::ObjectRef(map) => {
                let table = lua.create_table()?;
                for (key, elem) in map.iter() {
                    table.set(key.as_str(), ValueRef::from(elem))?;
                }
                Ok(LuaValue::Table(table))
            }
            ValueRef::Null => Ok(LuaValue::Nil),
        }
    }
}

impl<'lua, 'v> IntoLua<'lua> for ValueRef<'v> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        (&self).into_lua(lua)
    }
}

#[derive(Debug, Clone)]
pub struct DynCard(pub HashMap<String, Value>);

impl<'lua> IntoLua<'lua> for &DynCard {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let table = lua.create_table()?;
        for (key, elem) in self.0.iter() {
            table.set(key.as_str(), ValueRef::from(elem))?;
        }
        Ok(LuaValue::Table(table))
    }
}

impl<'lua> IntoLua<'lua> for DynCard {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        (&self).into_lua(lua)
    }
}

impl Access for DynCard {
    fn access(&'_ self, parts: &[crate::data::PredicatePathPart]) -> ValueRef<'_> {
        self.0.access(parts)
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
