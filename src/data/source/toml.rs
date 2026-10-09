//! Implementation for TOML as card data source.

use crate::data::source::Result as SrcResult;
use crate::data::{Card, DataSource, Predicate};
use std::io::Read;
use std::path::Path;

/// A reader for a TOML file as a card data source.
pub struct TomlSource {
    file: std::fs::File,
}

impl TomlSource {
    /// Open a TOML file to be used a card data source.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)?;
        Ok(Self { file })
    }
}

impl<C: Card> DataSource<C> for TomlSource {
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> SrcResult<Box<dyn Iterator<Item = SrcResult<C>> + '_>> {
        let mut data = Vec::new();
        self.file.read_to_end(&mut data)?;
        let cards: Vec<C> = toml::from_slice(&data)?;
        let cards_iterator = cards.into_iter();
        match filter {
            Some(filter) => Ok(Box::new(
                cards_iterator.filter(move |c| filter.eval(c)).map(Ok),
            )),
            None => Ok(Box::new(cards_iterator.map(Ok))),
        }
    }
}
