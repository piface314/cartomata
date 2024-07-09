//! Implements utilities to create color values.

use regex::Regex;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Default)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: Option<f64>,
}

impl Color {
    pub fn rgb(&self) -> (f64, f64, f64) {
        (self.r, self.g, self.b)
    }

    pub fn rgba(&self) -> (f64, f64, f64, f64) {
        (self.r, self.g, self.b, self.a.unwrap_or(1.0))
    }

    pub fn has_alpha(&self) -> bool {
        self.a.is_some()
    }

    pub fn pango_rgb(&self) -> (u16, u16, u16) {
        (
            Self::pango_channel(self.r),
            Self::pango_channel(self.g),
            Self::pango_channel(self.b),
        )
    }

    fn pango_channel(x: f64) -> u16 {
        let c = (x * 255.0) as u16;
        c | c << 8
    }
}

impl FromStr for Color {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re =
            Regex::new(r"^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})?$")
                .unwrap();

        let captures = re
            .captures(s)
            .ok_or("string not in form #RRGGBB or #RRGGBBAA")?;
        let mut values = captures
            .iter()
            .skip(1)
            .map(|c| c.map(|v| u8::from_str_radix(v.as_str(), 16).unwrap()));
        let r = values.next().unwrap().unwrap_or(0) as f64 / 255.0;
        let g = values.next().unwrap().unwrap_or(0) as f64 / 255.0;
        let b = values.next().unwrap().unwrap_or(0) as f64 / 255.0;
        let a = values.next().unwrap().map(|x| x as f64 / 255.0);
        Ok(Color { r, g, b, a })
    }
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the form #RRGGBBAA or #RRGGBB")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<Color>().map_err(|e| E::custom(e))
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ColorVisitor)
    }
}
