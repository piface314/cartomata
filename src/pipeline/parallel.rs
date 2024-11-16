use crate::data::{Card, Predicate};
use crate::decode::Decoder;
use crate::error::{Error, Result};
use crate::image::ImgBackend;
use crate::layer::RenderContext;
use crate::template::Template;

use crate::pipeline::{Pipeline, Visitor};

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::num::NonZero;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};

macro_rules! lock {
    (read $T:literal $lock:expr) => {
        $lock.read().map_err(|e| Error::read_lock($T, e))?
    };
    (write $T:literal $lock:expr) => {
        $lock.read().map_err(|e| Error::write_lock($T, e))?
    };
    ($T:literal $lock:expr) => {
        $lock.lock().map_err(|e| Error::mutex_lock($T, e))?
    };
}

#[derive(Debug, Clone, Copy)]
pub struct ParallelismOptions {
    n_workers: usize,
    batch_size: usize,
}

impl ParallelismOptions {
    pub fn new(n_workers: NonZero<usize>) -> Self {
        let n_workers = Self::check_n_workers(n_workers);
        Self { n_workers, batch_size: n_workers * 2 }
    }

    pub fn n_workers(&self) -> usize {
        self.n_workers
    }

    fn check_n_workers(n_workers: NonZero<usize>) -> usize {
        let av_workers =
            thread::available_parallelism().unwrap_or_else(|_| NonZero::new(1).unwrap());
        n_workers.min(av_workers).get()
    }

    pub fn set_batch_size(&mut self, batch_size: Option<NonZero<usize>>) {
        if let Some(batch_size) = batch_size {
            self.batch_size = batch_size.get();
        }
    }

    pub fn with_batch_size(mut self, batch_size: Option<NonZero<usize>>) -> Self {
        self.set_batch_size(batch_size);
        self
    }
}

impl<C, T, V> Pipeline<C, T, V>
where
    C: Card + Send,
    T: Template<C> + Send + Sync + 'static,
    T::SourceKey: Send,
    V: Visitor<C, T> + Send + Clone + 'static,
{
    pub fn run_parallel(
        self,
        source_key: T::SourceKey,
        filter: Option<Predicate>,
        opt: ParallelismOptions,
    ) -> Result<PipelineJoinHandle<C, T, V>> {
        let nw = opt.n_workers;
        let batch = opt.batch_size;

        let template = Arc::new(RwLock::new(self.template));
        let visitor = self.visitor;
        let queue = Arc::new(CardQueue::<C>::new(batch));
        let img_backend = Arc::new(RwLock::new(ImgBackend::new()?));

        let handle = {
            let template = template.clone();
            let visitor = visitor.clone();
            let queue = queue.clone();

            thread::spawn(move || {
                let template = lock!(read "template" template);
                let mut source = template.source(source_key)?;
                visitor.on_start(&*template, 0);

                let mut total: usize = 0;
                let cards_iter = source
                    .read(filter)?
                    .filter(|card_res| visitor.on_read(&*template, card_res));
                for (i, card) in cards_iter.enumerate() {
                    total += 1;
                    match card {
                        Ok(card) => queue.push(i, card)?,
                        Err(e) => visitor.on_read_err(&*template, i, e),
                    }
                }
                queue.done()?;
                visitor.on_total(&*template, total);
                Ok(())
            })
        };
        let mut workers = Vec::with_capacity(nw + 1);
        workers.push(handle);

        for id in 1..=nw {
            let queue = queue.clone();
            let template = template.clone();
            let visitor = visitor.clone();
            let img_backend = img_backend.clone();

            let handle = thread::spawn(move || {
                let template = lock!(read "template" template);
                let img_backend = lock!(read "image backend" img_backend);
                let worker = Worker {
                    id,
                    queue,
                    template: &*template,
                    visitor: &visitor,
                    img_backend: &*img_backend,
                };
                let result = worker.run();
                visitor.on_finish(&*template, id, &result);
                result
            });
            workers.push(handle);
        }

        Ok(PipelineJoinHandle::new(template, visitor, workers))
    }
}

pub struct PipelineJoinHandle<C: Card, T: Template<C>, V: Visitor<C, T> = ()> {
    template: Arc<RwLock<T>>,
    visitor: V,
    handles: Vec<JoinHandle<Result<()>>>,
    _marker: PhantomData<C>,
}

impl<C, T, V> PipelineJoinHandle<C, T, V>
where
    C: Card,
    T: Template<C>,
    V: Visitor<C, T>,
{
    fn new(template: Arc<RwLock<T>>, visitor: V, handles: Vec<JoinHandle<Result<()>>>) -> Self {
        Self { template, visitor, handles, _marker: PhantomData }
    }

    pub fn join(self) -> Result<(T, V)> {
        let visitor = self.visitor;

        let mut handles = self.handles.into_iter().enumerate();

        let (i, handle) = handles.next().expect("at least 1 join handle should exist");
        let base_result = handle.join().map_err(|_| Error::thread_join(i))?;

        for (i, handle) in handles {
            let _ = handle.join().map_err(|_| Error::thread_join(i))?;
        }

        let template = Arc::into_inner(self.template)
            .expect("all handles should have been joined")
            .into_inner()
            .map_err(|e| Error::read_lock("template", e))?;
        visitor.on_finish(&template, 0, &base_result);

        Ok((template, visitor))
    }
}

struct CardQueue<C: Card> {
    queue: Mutex<CardQueueState<C>>,
    capacity: usize,
    cond: Condvar,
}

struct CardQueueState<C: Card> {
    queue: VecDeque<(usize, C)>,
    done: bool,
}

impl<C: Card> CardQueueState<C> {
    fn new(capacity: usize) -> Self {
        Self { queue: VecDeque::with_capacity(capacity), done: false }
    }
}

impl<C: Card> CardQueue<C> {
    fn new(capacity: usize) -> Self {
        Self {
            queue: Mutex::new(CardQueueState::new(capacity)),
            capacity,
            cond: Condvar::new(),
        }
    }

    fn push(&self, index: usize, card: C) -> Result<()> {
        let state = lock!("card queue" self.queue);
        let mut state = self
            .cond
            .wait_while(state, |s| s.queue.len() >= self.capacity)
            .map_err(|e| Error::mutex_lock("card queue", e))?;
        state.queue.push_back((index, card));
        self.cond.notify_one();
        Ok(())
    }

    fn pop(&self) -> Result<Option<(usize, C)>> {
        let state = lock!("card queue" self.queue);
        let mut state = self
            .cond
            .wait_while(state, |s| s.queue.is_empty() && !s.done)
            .map_err(|e| Error::mutex_lock("card queue", e))?;
        let card = state.queue.pop_front();
        self.cond.notify_all();
        Ok(card)
    }

    fn done(&self) -> Result<()> {
        let mut state = lock!("card queue" self.queue);
        state.done = true;
        self.cond.notify_all();
        Ok(())
    }
}

struct Worker<'a, C: Card, T: Template<C>, V: Visitor<C, T>> {
    pub id: usize,
    pub queue: Arc<CardQueue<C>>,
    pub template: &'a T,
    pub img_backend: &'a ImgBackend,
    pub visitor: &'a V,
}

impl<'a, C: Card + Send, T: Template<C>, V: Visitor<C, T>> Worker<'a, C, T, V> {
    fn run(&self) -> Result<()> {
        let ctx = RenderContext {
            img_map: self.template.resources(),
            font_map: self.template.fonts(),
            backend: self.img_backend,
        };
        let decoder = self.template.decoder()?;
        while let Some((i, card)) = self.queue.pop()? {
            self.visitor.on_iter_start(self.template, self.id, i, &card);
            match self.process(&decoder, &card, &ctx) {
                Ok(()) => self.visitor.on_iter_ok(self.template, self.id, i, card),
                Err(e) => self.visitor.on_iter_err(self.template, self.id, i, card, e),
            }
        }
        Ok(())
    }

    fn process(&self, decoder: &T::Decoder, card: &C, ctx: &RenderContext) -> Result<()> {
        let layers = decoder.decode(card)?;
        let img = layers.render(ctx)?;
        self.template.output(card, &img, &ctx.backend)?;
        Ok(())
    }
}
