//! Contains representations for card data.

// use crate::error::Error;

use serde::Deserialize;
use std::collections::HashMap;

pub type Schema = HashMap<String, Type>;
pub type GCard<'a> = HashMap<&'a str, Value>;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Nil
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Type {
    Int,
    Float,
    String,
}

impl Value {
    pub fn parse(ftype: Type, content: impl AsRef<str>) -> Self {
        let content = content.as_ref();
        match ftype {
            Type::Int => content.parse::<i64>().map(|v| Value::Int(v)).unwrap_or(Value::Nil),
            Type::Float => content.parse::<f64>().map(|v| Value::Float(v)).unwrap_or(Value::Nil),
            Type::String => Value::String(content.to_string()),
        }
    }
}
