use crate::data::{Card, DataSource, Predicate};
use crate::decode::Decoder;
use crate::error::Result;
use crate::image::{ImageMap, ImgBackend, OutputMap};
use crate::layer::RenderContext;
use crate::text::FontMap;
use crate::Error;

use std::collections::VecDeque;
use std::num::NonZero;
// use std::sync::{Arc, Mutex};
use std::thread;

macro_rules! warn {
    ($res:expr) => {
        match $res {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: {e}");
                continue;
            }
        }
    };
}

pub struct Pipeline<C: Card, D: Decoder<C>, O: OutputMap<C>> {
    n_workers: NonZero<usize>,
    queue: VecDeque<C>,
    source: Box<dyn DataSource<C>>,
    decoder: D,
    img_map: ImageMap,
    font_map: FontMap,
    img_backend: ImgBackend,
    out_map: O,
}

impl<C: Card, D: Decoder<C>, O: OutputMap<C>> Pipeline<C, D, O> {
    pub fn new(
        n_workers: Option<NonZero<usize>>,
        source: Box<dyn DataSource<C>>,
        decoder: D,
        img_map: ImageMap,
        font_map: FontMap,
        out_map: O,
    ) -> Result<Self> {
        let n_workers = match n_workers {
            Some(n) => n,
            None => thread::available_parallelism().map_err(|_| Error::FontConfigInitError)?,
        };
        Ok(Self {
            n_workers,
            queue: VecDeque::with_capacity(n_workers.get() * 2),
            source,
            decoder,
            img_map,
            font_map,
            img_backend: ImgBackend::new()?,
            out_map,
        })
    }

    pub fn run(&mut self, filter: Option<Predicate>) -> Result<()> {
        let mut ctx = RenderContext {
            backend: &mut self.img_backend,
            font_map: &self.font_map,
            img_map: &self.img_map,
        };
        for card_result in self.source.read(filter)? {
            let card = warn!(card_result);
            let path = self.out_map.path(&card);
            let stack = warn!(self.decoder.decode(card));
            let img = warn!(stack.render(&mut ctx));
            warn!(self.out_map.write(ctx.backend, &img, path));
        }
        Ok(())
    }
}
