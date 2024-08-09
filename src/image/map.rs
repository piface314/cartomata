use libvips::VipsImage;

use crate::data::Card;
use crate::error::{Error, Result};
use crate::image::color::Color;
use crate::image::ImgBackend;

use std::path::{Path, PathBuf};

pub struct ImageMap {
    pub assets_folder: PathBuf,
    pub artwork_folder: PathBuf,
    pub extensions: Vec<String>,
    pub placeholder: Option<PathBuf>,
    pub card_size: (i32, i32),
    pub background: Color,
}

impl ImageMap {
    pub fn asset_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let mut fp = self.assets_folder.clone();
        fp.push(path.as_ref());
        fp
    }

    pub fn artwork_path(&self, key: impl AsRef<str>) -> Result<PathBuf> {
        let key = key.as_ref();
        let mut path = self.artwork_folder.clone();
        path.push(key);
        self.extensions
            .iter()
            .filter_map(move |ext| {
                path.set_extension(ext);
                path.exists().then(|| path.clone())
            })
            .next()
            .ok_or_else(|| Error::ArtworkNotFound(key.to_string()))
    }

    pub fn artwork_literal_path(&self, key: impl AsRef<Path>) -> PathBuf {
        let key = key.as_ref();
        let mut path = self.artwork_folder.clone();
        path.push(key);
        path
    }
}

pub trait OutputMap {
    type C: Card;
    fn path(&self, card: &Self::C) -> PathBuf;
    fn write(&self, ib: &ImgBackend, img: &VipsImage, path: impl AsRef<Path>) -> Result<()> {
        let fp = path.as_ref();
        let fp = fp.to_string_lossy();
        img.image_write_to_file(&fp).map_err(|e| ib.err(e))
    }
}
