use crate::data::{Card, DataSource, Predicate};
use crate::decode::{Decoder, DecoderFactory};
use crate::error::{Error, Result};
use crate::image::{ImageMap, ImgBackend, OutputMap};
use crate::layer::RenderContext;
use crate::text::FontMap;

use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};

pub struct Pipeline<C: Card, D: DecoderFactory<C>, O: OutputMap<C>> {
    n_workers: usize,
    source: Box<dyn DataSource<C>>,
    decoder_factory: D,
    img_map: ImageMap,
    font_map: FontMap,
    img_backend: ImgBackend,
    out_map: O,
}

impl<C: Card, D: DecoderFactory<C> + 'static, O: OutputMap<C> + 'static> Pipeline<C, D, O> {
    const MAX_WORKERS: usize = 4;

    pub fn new(
        n_workers: Option<NonZero<usize>>,
        source: Box<dyn DataSource<C>>,
        decoder_factory: D,
        img_map: ImageMap,
        font_map: FontMap,
        out_map: O,
    ) -> Result<Self> {
        let n_workers = match n_workers {
            Some(n) => n.get(),
            None => thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        };
        let n_workers = n_workers.clamp(1, Self::MAX_WORKERS);
        Ok(Self {
            n_workers,
            source,
            decoder_factory,
            img_map,
            font_map,
            img_backend: ImgBackend::new()?,
            out_map,
        })
    }

    pub fn run(mut self, filter: Option<Predicate>) -> Result<()> {
        let n_workers = self.n_workers;
        let queue = Arc::new(CardQueue::new(n_workers * 2));
        let factory = Arc::new(RwLock::new(self.decoder_factory));
        let img_map = Arc::new(RwLock::new(self.img_map));
        let img_backend = Arc::new(RwLock::new(self.img_backend));
        let font_map = Arc::new(RwLock::new(self.font_map));
        let out_map = Arc::new(RwLock::new(self.out_map));
        let (tx, rx) = mpsc::channel();

        let handles: Vec<JoinHandle<Result<()>>> = (0..n_workers)
            .map(|id| {
                let tx = tx.clone();
                let queue = queue.clone();
                let factory = factory.clone();
                let img_map = img_map.clone();
                let out_map = out_map.clone();
                let img_backend = img_backend.clone();
                let img_map = img_map.clone();
                let font_map = font_map.clone();

                thread::spawn(move || {
                    let factory = factory
                        .read()
                        .map_err(|e| Error::ReadLockError("DecoderFactory", e.to_string()))?;
                    let decoder = factory.create()?;
                    let worker = Worker {
                        id,
                        tx,
                        queue,
                        decoder,
                        img_backend,
                        img_map,
                        font_map,
                        out_map,
                    };
                    worker.run()
                })
            })
            .collect();

        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    (id, WorkerEvent::Info(e)) => eprintln!("{id:02}::info: {e}"),
                    (id, WorkerEvent::Warn(e)) => eprintln!("{id:02}::warn: {e}"),
                    (id, WorkerEvent::Done) => eprintln!("{id:02}::finished"),
                }
            }
        });

        for card in self.source.read(filter)? {
            match card {
                Ok(card) => queue.push(card)?,
                Err(e) => eprintln!("src::warn: {e}"),
            }
        }
        queue.done()?;

        for (id, handle) in handles.into_iter().enumerate() {
            let thread_result = handle.join().map_err(|_| Error::JoinError(id))?;
            thread_result.unwrap_or_else(|e| eprintln!("{id}::error: {e}"));
        }
        Ok(())
    }
}

macro_rules! lock {
    (read $T:literal $lock:expr) => {
        $lock
            .read()
            .map_err(|e| Error::ReadLockError($T, e.to_string()))?
    };
    (write $T:literal $lock:expr) => {
        $lock
            .read()
            .map_err(|e| Error::WriteLockError($T, e.to_string()))?
    };
    ($T:literal $lock:expr) => {
        $lock
            .lock()
            .map_err(|e| Error::MutexLockError($T, e.to_string()))?
    };
}

struct CardQueue<C: Card> {
    queue: Mutex<(VecDeque<C>, bool)>,
    capacity: usize,
    cond: Condvar,
}

impl<C: Card> CardQueue<C> {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Mutex::new((VecDeque::with_capacity(capacity), false)),
            capacity,
            cond: Condvar::new(),
        }
    }

    pub fn push(&self, card: C) -> Result<()> {
        let queue = lock!("CardQueue" self.queue);
        let mut queue = self
            .cond
            .wait_while(queue, |(q, _)| q.len() >= self.capacity)
            .map_err(|e| Error::MutexLockError("CardQueue", e.to_string()))?;
        queue.0.push_back(card);
        self.cond.notify_one();
        Ok(())
    }

    pub fn pop(&self) -> Result<Option<C>> {
        let queue = lock!("CardQueue" self.queue);
        let mut queue = self
            .cond
            .wait_while(queue, |(q, done)| q.is_empty() && !*done)
            .map_err(|e| Error::MutexLockError("CardQueue", e.to_string()))?;
        let card = queue.0.pop_front();
        self.cond.notify_all();
        Ok(card)
    }

    pub fn done(&self) -> Result<()> {
        let mut queue = lock!("CardQueue" self.queue);
        (*queue).1 = true;
        self.cond.notify_all();
        Ok(())
    }
}

enum WorkerEvent {
    Info(String),
    Warn(Error),
    Done,
}

struct Worker<C: Card, D: Decoder<C>, O: OutputMap<C>> {
    pub id: usize,
    pub tx: Sender<(usize, WorkerEvent)>,
    pub queue: Arc<CardQueue<C>>,
    pub decoder: D,
    pub img_map: Arc<RwLock<ImageMap>>,
    pub font_map: Arc<RwLock<FontMap>>,
    pub img_backend: Arc<RwLock<ImgBackend>>,
    pub out_map: Arc<RwLock<O>>,
}

impl<C: Card, D: Decoder<C>, O: OutputMap<C>> Worker<C, D, O> {
    pub fn run(&self) -> Result<()> {
        let img_map = lock!(read "ImageMap" self.img_map);
        let font_map = lock!(read "FontMap" self.font_map);
        let img_backend = lock!(read "ImgBackend" self.img_backend);
        let ctx = RenderContext {
            img_map: &img_map,
            font_map: &font_map,
            backend: &img_backend,
        };
        while let Some(card) = self.queue.pop()? {
            let card_id = card.get("id");
            match self.process(card, &ctx) {
                Ok(()) => self.info(format!("Processing card {card_id}"))?,
                Err(e) => self.warn(e)?,
            }
        }
        self.done()?;
        Ok(())
    }

    fn process(&self, card: C, ctx: &RenderContext) -> Result<()> {
        let out_map = lock!(read "OutputMap" self.out_map);
        let path = out_map.path(&card);
        let stack = self.decoder.decode(card)?;
        let img = stack.render(ctx)?;
        out_map.write(&ctx.backend, &img, path)?;
        Ok(())
    }

    fn info(&self, msg: String) -> Result<()> {
        self.tx
            .send((self.id, WorkerEvent::Info(msg)))
            .map_err(|e| Error::SendError(self.id, e.to_string()))
    }

    fn warn(&self, error: Error) -> Result<()> {
        self.tx
            .send((self.id, WorkerEvent::Warn(error)))
            .map_err(|e| Error::SendError(self.id, e.to_string()))
    }

    fn done(&self) -> Result<()> {
        self.tx
            .send((self.id, WorkerEvent::Done))
            .map_err(|e| Error::SendError(self.id, e.to_string()))
    }
}