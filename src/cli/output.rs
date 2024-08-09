use crate::cli::card::DynCard;
use crate::error::Result;
use crate::image::ImgBackend;
use crate::image::OutputMap;

use libvips::VipsImage;
use regex::Regex;
use std::path::{Path, PathBuf};

pub struct DynOutputMap {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub pattern: String,
}

impl OutputMap for DynOutputMap {
    type C = DynCard;

    fn path(&self, card: &Self::C) -> PathBuf {
        let re = Regex::new(r"\{[^}]+\}").unwrap();
        PathBuf::from(
            re.replace_all(self.pattern.as_str(), |captures: &regex::Captures| {
                card.0
                    .get(captures.get(0).unwrap().as_str())
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            })
            .to_string(),
        )
    }

    fn write(&self, ib: &ImgBackend, img: &VipsImage, path: impl AsRef<Path>) -> Result<()> {
        let img = ib.scale_to(
            img,
            self.width.map(|x| x as f64),
            self.height.map(|x| x as f64),
        )?;
        OutputMap::write(self, ib, &img, path)
    }
}
