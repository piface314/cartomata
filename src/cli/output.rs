use crate::cli::card::DynCard;
use crate::error::Result;
use crate::image::ImgBackend;

use libvips::VipsImage;
use regex::Regex;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

pub struct OutputMap {
    pub prefix: PathBuf,
    pub resize: Resize,
    pub pattern: String,
    pub ext: String,
}

impl OutputMap {
    pub fn new(pattern: String) -> Self {
        Self {
            prefix: PathBuf::new(),
            resize: Resize::default(),
            pattern,
            ext: String::from("png"),
        }
    }

    pub fn set_prefix(&mut self, prefix: Option<PathBuf>) {
        if let Some(prefix) = prefix {
            self.prefix = prefix;
        }
    }

    pub fn set_resize(&mut self, resize: Option<Resize>) {
        if let Some(resize) = resize {
            self.resize = resize;
        }
    }

    pub fn set_ext(&mut self, ext: Option<String>) {
        if let Some(ext) = ext {
            self.ext = ext;
        }
    }

    pub fn identify(&self, card: &DynCard) -> String {
        let re = Regex::new(r"\{([^}]+)\}").unwrap();
        re.replace_all(self.pattern.as_str(), |captures: &regex::Captures| {
            card.0
                .get(captures.get(1).unwrap().as_str())
                .map(|v| v.to_string())
                .unwrap_or_default()
        })
        .to_string()
    }

    pub fn write(&self, card: &DynCard, img: &VipsImage, ib: &ImgBackend) -> Result<()> {
        let img = ib.scale_to(img, self.resize.width, self.resize.height)?;
        let mut path = self.prefix.clone();
        path.push(self.identify(card));
        path.set_extension(self.ext.clone());
        ib.write(&img, path)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Resize {
    width: Option<i32>,
    height: Option<i32>,
}

impl FromStr for Resize {
    type Err = &'static str;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let re = Regex::new(r"^(\d+)?\s*x\s*(\d+)?$").unwrap();

        let captures = re
            .captures(s)
            .ok_or("string not in form WxH where W and H are optional integer numbers")?;
        let width = captures.get(1).map(|m| m.as_str().parse().unwrap());
        let height = captures.get(2).map(|m| m.as_str().parse().unwrap());
        Ok(Self { width, height })
    }
}

struct ResizeVisitor;

impl<'de> Visitor<'de> for ResizeVisitor {
    type Value = Resize;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the form WxH where W and H are optional integer numbers")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<Resize>().map_err(|e| E::custom(e))
    }
}

impl<'de> Deserialize<'de> for Resize {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ResizeVisitor)
    }
}
