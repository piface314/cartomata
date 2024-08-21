use crate::cli::card::DynCard;
use crate::error::Result;
use crate::image::ImgBackend;
use crate::image::OutputMap;

use libvips::VipsImage;
use regex::Regex;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct DynOutputMap {
    pub resize: Resize,
    pub pattern: String,
}

impl OutputMap for DynOutputMap {
    type C = DynCard;

    fn path(&self, card: &Self::C) -> PathBuf {
        let re = Regex::new(r"\{([^}]+)\}").unwrap();
        PathBuf::from(
            re.replace_all(self.pattern.as_str(), |captures: &regex::Captures| {
                card.0
                    .get(captures.get(1).unwrap().as_str())
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            })
            .to_string(),
        )
    }

    fn write(&self, ib: &ImgBackend, img: &VipsImage, path: impl AsRef<Path>) -> Result<()> {
        let img = ib.scale_to(img, self.resize.width, self.resize.height)?;
        let fp = path.as_ref();
        let fp = fp.to_string_lossy();
        img.image_write_to_file(&fp).map_err(|e| ib.err(e))
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
        let re =
            Regex::new(r"^(\d+)?\s*x\s*(\d+)?$")
                .unwrap();

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
            E: de::Error, {
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

