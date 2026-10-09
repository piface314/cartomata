//! Implementation for JSON as card data source.

use crate::data::source::Result as SrcResult;
use crate::data::{Card, DataSource, Predicate};
use std::path::Path;

/// A reader for a JSON file as a card data source.
pub struct JsonSource {
    file: std::fs::File,
}

impl JsonSource {
    /// Open a JSON file to be used a card data source.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)?;
        Ok(Self { file })
    }
}

impl<C: Card> DataSource<C> for JsonSource {
    fn read(
        &mut self,
        filter: Option<Predicate>,
    ) -> SrcResult<Box<dyn Iterator<Item = SrcResult<C>> + '_>> {
        let cards: Vec<C> = serde_json::from_reader(&self.file)?;
        let cards_iterator = cards.into_iter();
        match filter {
            Some(filter) => Ok(Box::new(
                cards_iterator.filter(move |c| filter.eval(c)).map(Ok),
            )),
            None => Ok(Box::new(cards_iterator.map(Ok))),
        }
    }
}
