use crate::error::{Error, Result};
use crate::image::color::Color;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
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
        let found_path = self.extensions
            .iter()
            .filter_map(move |ext| {
                path.set_extension(ext);
                path.exists().then(|| path.clone())
            })
            .next();
        match (found_path, &self.placeholder) {
            (Some(path), _) => Ok(path),
            (None, Some(placeholder)) => Ok(placeholder.clone()),
            (None, None) => Err(Error::no_artwork(key)),
        }
    }

    pub fn artwork_literal_path(&self, key: impl AsRef<Path>) -> PathBuf {
        let key = key.as_ref();
        let mut path = self.artwork_folder.clone();
        path.push(key);
        path
    }
}
