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
        let img = ib.scale_to(img, self.width, self.height)?;
        let fp = path.as_ref();
        let fp = fp.to_string_lossy();
        img.image_write_to_file(&fp).map_err(|e| ib.err(e))
    }
}
