//! Stroke parameter definitions

use crate::image::color::Color;

use regex::Regex;
use serde::de::{Deserializer, Visitor};
use serde::{de, Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Copy, Clone, Default, Serialize)]
pub struct Stroke(pub f64, pub Color);

struct StrokeVisitor;

impl<'de> Visitor<'de> for StrokeVisitor {
    type Value = Stroke;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string with a number and a hex color")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        let re = Regex::new(r"\s+").unwrap();
        let mut parts = re.split(v);
        let size = parts.next().ok_or_else(|| E::custom("expected a number"))?;
        let color = parts.next().ok_or_else(|| E::custom("expected a color"))?;
        let size = size.parse::<f64>().map_err(|e| E::custom(e.to_string()))?;
        let color = color.parse::<Color>().map_err(|e| E::custom(e.to_string()))?;
        Ok(Stroke(size, color))
    }
}

impl<'de> Deserialize<'de> for Stroke {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Stroke, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StrokeVisitor)
    }
}
