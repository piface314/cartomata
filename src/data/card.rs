//! Contains representations for card data.

use crate::error::Error;

use serde::Deserialize;
use std::collections::HashMap;

pub type CardSchema = HashMap<String, CardFieldType>;
pub type CardData<'a> = HashMap<&'a str, CardField>;

#[derive(Debug, Clone)]
pub enum CardField {
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardFieldType {
    Int,
    Float,
    String,
}

impl std::str::FromStr for CardFieldType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "int" => Ok(CardFieldType::Int),
            "float" => Ok(CardFieldType::Float),
            "string" => Ok(CardFieldType::String),
            _ => Err(Error::UnknownCardFieldType(s.to_string())),
        }
    }
}

impl CardField {
    pub fn new(ftype: CardFieldType, content: impl AsRef<str>) -> Self {
        let content = content.as_ref();
        match ftype {
            CardFieldType::Int => {
                CardField::Int(content.parse().expect("card field should be integer"))
            }
            CardFieldType::Float => {
                CardField::Float(content.parse().expect("card field should be float"))
            }
            CardFieldType::String => CardField::String(content.to_string()),
        }
    }
}
