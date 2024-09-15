use crate::data::{Card, Predicate};
use crate::decode::Decoder;
use crate::error::{Error, Result};
use crate::image::ImgBackend;
use crate::layer::RenderContext;
use crate::logs::{LogMsg, ProgressBar};
use crate::template::Template;

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::num::NonZero;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};

pub struct Pipeline<C: Card, T: Template<C>> {
    template: T,
    _card: PhantomData<C>,
}

pub type CardResult<C> = std::result::Result<C, (Option<C>, Error)>;

pub enum WorkerMsg<C: Card> {
    Total {
        n: usize,
    },
    IterStart {
        index: usize,
        card_id: String,
        worker: usize,
    },
    IterErr {
        index: usize,
        card_id: String,
        card: Option<C>,
        error: Error,
        worker: usize,
    },
    IterOk {
        index: usize,
        card_id: String,
        card: C,
        worker: usize,
    },
    Err {
        error: Error,
        worker: usize,
    },
    Ok {
        worker: usize,
    },
}

impl<C: Card> WorkerMsg<C> {
    fn log_iter_start(
        tx: &Sender<LogMsg>,
        index: usize,
        card: &C,
        template: &impl Template<C>,
    ) -> Result<()> {
        let card_id = template.identify(&card);
        Self::IterStart { index, card_id, worker: 0 }.log(tx)
    }

    fn log_iter_read_err(tx: &Sender<LogMsg>, index: usize, error: Error) -> Result<()> {
        Self::IterErr { index, card_id: String::new(), card: None, error, worker: 0 }.log(tx)
    }

    fn log_iter_err(
        tx: &Sender<LogMsg>,
        index: usize,
        card: C,
        template: &impl Template<C>,
        error: Error,
    ) -> Result<()> {
        let card_id = template.identify(&card);
        Self::IterErr { index, card_id, card: Some(card), error, worker: 0 }.log(tx)
    }

    fn log_iter_ok(
        tx: &Sender<LogMsg>,
        index: usize,
        card: C,
        template: &impl Template<C>,
    ) -> Result<()> {
        let card_id = template.identify(&card);
        Self::IterOk { index, card_id, card, worker: 0 }.log(tx)
    }

    fn log_err(tx: &Sender<LogMsg>, error: Error) -> Result<()> {
        Self::Err { error, worker: 0 }.log(tx)
    }

    fn log_ok(tx: &Sender<LogMsg>) -> Result<()> {
        Self::Ok { worker: 0 }.log(tx)
    }

    fn log(self, tx: &Sender<LogMsg>) -> Result<()> {
        let (id, msg) = match self {
            Self::Total { n } => (0, LogMsg::Total(n)),
            Self::IterStart { index, card_id, worker } => (
                worker,
                LogMsg::Running(worker, format!("processing card {card_id} (#{index})...")),
            ),
            Self::IterErr { index, card: None, error, worker, .. } => (
                worker,
                LogMsg::Warn(worker, format!("failed to read card (#{index}): {error}")),
            ),
            Self::IterErr { index, card_id, card: Some(_), error, worker } => (
                worker,
                LogMsg::Warn(
                    worker,
                    format!("failed to process card {card_id} (#{index}): {error}"),
                ),
            ),
            Self::IterOk { worker, .. } => (worker, LogMsg::Progress(worker)),
            Self::Ok { worker } => (worker, LogMsg::Success(worker, "done!".to_string())),
            Self::Err { error, worker } => (worker, LogMsg::Error(worker, error.to_string())),
        };
        tx.send(msg).map_err(|e| Error::thread_send(id, e))
    }
}

macro_rules! unwrapc {
    ($v:expr) => {
        match $v {
            Ok(x) => x,
            Err(error) => return Err((None, error)),
        }
    };
    ($v:expr; with ($card:expr)) => {
        match $v {
            Ok(x) => x,
            Err(error) => return Err((Some($card), error)),
        }
    };
    ($v:expr; with ($index:expr) to $tx:expr) => {
        match $v {
            Ok(x) => x,
            Err(error) => {
                WorkerMsg::<C>::log_iter_read_err($tx, $index, error)?;
                continue;
            }
        }
    };
    ($v:expr; with ($index:expr, $card:expr, $template:expr) to $tx:expr) => {
        match $v {
            Ok(x) => x,
            Err(error) => {
                WorkerMsg::log_iter_err($tx, $index, $card, $template, error)?;
                continue;
            }
        }
    };
}

impl<C: Card, T: Template<C>> Pipeline<C, T> {
    pub fn new(template: T) -> Self {
        Self { template, _card: PhantomData }
    }

    pub fn run(
        self,
        source_key: T::SourceKey,
        filter: Option<Predicate>,
    ) -> Result<Vec<CardResult<C>>> {
        let template = self.template;
        let mut source = template.source(source_key)?;
        let decoder = template.decoder()?;
        let font_map = template.fonts();
        let img_map = template.resources();
        let backend = ImgBackend::new()?;
        let ctx = RenderContext { backend: &backend, font_map, img_map };
        let res = source
            .read(filter)?
            .map(|card_res| {
                let card = unwrapc!(card_res);
                let layers = unwrapc!(decoder.decode(&card); with (card));
                let img = unwrapc!(layers.render(&ctx); with (card));
                unwrapc!(template.output(&card, &img, &backend); with (card));
                Ok(card)
            })
            .collect();
        Ok(res)
    }

    pub fn run_with_logs(self, source_key: T::SourceKey, filter: Option<Predicate>) -> Result<()> {
        let (tx, handle) = ProgressBar::spawn_stderr(0);
        match Self::run_with_logs_internal(&tx, self.template, source_key, filter) {
            Ok(()) => WorkerMsg::<C>::log_ok(&tx)?,
            Err(e) => WorkerMsg::<C>::log_err(&tx, e)?,
        }
        drop(tx);
        handle.join().map_err(|_| Error::thread_join(0))?
    }

    pub fn run_with_logs_internal(
        tx: &Sender<LogMsg>,
        template: T,
        source_key: T::SourceKey,
        filter: Option<Predicate>,
    ) -> Result<()> {
        let mut source = template.source(source_key)?;
        let decoder = template.decoder()?;
        let font_map = template.fonts();
        let img_map = template.resources();
        let backend = ImgBackend::new()?;
        let ctx = RenderContext { backend: &backend, font_map, img_map };
        for (index, card) in source.read(filter)?.enumerate() {
            let card = unwrapc!(card; with (index) to tx);
            WorkerMsg::log_iter_start(tx, index, &card, &template)?;
            let layers = unwrapc!(decoder.decode(&card); with (index, card, &template) to tx);
            let img = unwrapc!(layers.render(&ctx); with (index, card, &template) to tx);
            unwrapc!(template.output(&card, &img, &backend); with (index, card, &template) to tx);
            WorkerMsg::log_iter_ok(tx, index, card, &template)?;
        }
        Ok(())
    }
}

impl<C: Card + Send> WorkerMsg<C> {
    fn send_total(tx: &Sender<Self>, n: usize) -> Result<()> {
        tx.send(Self::Total { n })
            .map_err(|e| Error::thread_send(0, e))
    }

    fn send_iter_start(
        tx: &Sender<Self>,
        worker: usize,
        index: usize,
        card: &C,
        template: &impl Template<C>,
    ) -> Result<()> {
        let card_id = template.identify(card);
        tx.send(Self::IterStart { index, card_id, worker })
            .map_err(|e| Error::thread_send(0, e))
    }

    fn send_iter_read_err(tx: &Sender<Self>, index: usize, error: Error) -> Result<()> {
        tx.send(Self::IterErr { index, card_id: String::new(), card: None, error, worker: 0 })
            .map_err(|e| Error::thread_send(0, e))
    }

    fn send_iter_err(
        tx: &Sender<Self>,
        worker: usize,
        index: usize,
        card: C,
        template: &impl Template<C>,
        error: Error,
    ) -> Result<()> {
        let card_id = template.identify(&card);
        tx.send(Self::IterErr { index, card_id, card: Some(card), error, worker })
            .map_err(|e| Error::thread_send(0, e))
    }

    fn send_iter_ok(
        tx: &Sender<Self>,
        worker: usize,
        index: usize,
        card: C,
        template: &impl Template<C>,
    ) -> Result<()> {
        let card_id = template.identify(&card);
        tx.send(Self::IterOk { index, card_id, card, worker })
            .map_err(|e| Error::thread_send(0, e))
    }

    fn send_ok(tx: &Sender<Self>, worker: usize) -> Result<()> {
        tx.send(Self::Ok { worker })
            .map_err(|e| Error::thread_send(0, e))
    }

    fn send_err(tx: &Sender<Self>, worker: usize, error: Error) -> Result<()> {
        tx.send(Self::Err { error, worker })
            .map_err(|e| Error::thread_send(0, e))
    }
}

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

impl<C: Card + Send, T: Template<C> + Send + Sync + 'static> Pipeline<C, T> {
    pub fn run_parallel(
        self,
        n_workers: NonZero<usize>,
        source_key: T::SourceKey,
        filter: Option<Predicate>,
    ) -> Result<(Receiver<WorkerMsg<C>>, Vec<JoinHandle<Result<()>>>)> {
        let nw = Self::check_n_workers(n_workers);
        let queue = Arc::new(CardQueue::<C>::new(nw * 2));
        let img_backend = Arc::new(RwLock::new(ImgBackend::new()?));
        let template = Arc::new(RwLock::new(self.template));
        let (tx, rx) = mpsc::channel();

        let handle = {
            let tx = tx.clone();
            let queue = queue.clone();
            let template = lock!(read "template" template);
            let mut source = template.source(source_key)?;
            thread::spawn(move || {
                let mut total: usize = 0;
                for (index, card) in source.read(filter)?.enumerate() {
                    total += 1;
                    match card {
                        Ok(card) => queue.push(index, card)?,
                        Err(e) => WorkerMsg::send_iter_read_err(&tx, index, e)?,
                    }
                }
                queue.done()?;
                WorkerMsg::send_total(&tx, total)
            })
        };
        let mut workers = Vec::with_capacity(nw + 1);
        workers.push(handle);

        for id in 1..=nw {
            let tx = tx.clone();
            let queue = queue.clone();
            let template = template.clone();
            let img_backend = img_backend.clone();

            let handle = thread::spawn(move || {
                let worker = Worker { id, tx: tx.clone(), queue, template, img_backend };
                match worker.run() {
                    Ok(()) => WorkerMsg::send_ok(&tx, id),
                    Err(error) => WorkerMsg::send_err(&tx, id, error),
                }
            });
            workers.push(handle);
        }
        Ok((rx, workers))
    }

    pub fn run_parallel_with_logs(
        self,
        n_workers: NonZero<usize>,
        source_key: T::SourceKey,
        filter: Option<Predicate>,
    ) -> Result<()> {
        let (rx, workers) = self.run_parallel(n_workers, source_key, filter)?;
        let (tx, handle) = ProgressBar::spawn_stderr(workers.len() - 1);
        while let Ok(event) = rx.recv() {
            event.log(&tx)?;
        }
        for (i, worker) in workers.into_iter().enumerate() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tx
                    .send(LogMsg::Error(i, e.to_string()))
                    .map_err(|e| Error::thread_send(0, e))?,
                Err(_) => tx
                    .send(LogMsg::Error(i, Error::Unknown.to_string()))
                    .map_err(|e| Error::thread_send(0, e))?,
            }
        }
        tx.send(LogMsg::Success(0, "done!".to_string()))
            .map_err(|e| Error::thread_send(0, e))?;
        drop(tx);
        handle.join().map_err(|_| Error::thread_join(0))?
    }

    fn check_n_workers(n_workers: NonZero<usize>) -> usize {
        let av_workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        n_workers.get().clamp(1, av_workers)
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

struct Worker<C: Card, T: Template<C>> {
    pub id: usize,
    pub tx: Sender<WorkerMsg<C>>,
    pub queue: Arc<CardQueue<C>>,
    pub template: Arc<RwLock<T>>,
    pub img_backend: Arc<RwLock<ImgBackend>>,
}

impl<C: Card + Send, T: Template<C>> Worker<C, T> {
    fn run(&self) -> Result<()> {
        let template = lock!(read "template" self.template);
        let img_backend = lock!(read "image backend" self.img_backend);
        let ctx = RenderContext {
            img_map: template.resources(),
            font_map: template.fonts(),
            backend: &img_backend,
        };
        let decoder = template.decoder()?;
        while let Some((index, card)) = self.queue.pop()? {
            WorkerMsg::send_iter_start(&self.tx, self.id, index, &card, &*template)?;
            match self.process(&template, &decoder, &card, &ctx) {
                Ok(()) => WorkerMsg::send_iter_ok(&self.tx, self.id, index, card, &*template)?,
                Err(e) => WorkerMsg::send_iter_err(&self.tx, self.id, index, card, &*template, e)?,
            }
        }
        Ok(())
    }

    fn process(
        &self,
        template: &T,
        decoder: &T::Decoder,
        card: &C,
        ctx: &RenderContext,
    ) -> Result<()> {
        let stack = decoder.decode(card)?;
        let img = stack.render(ctx)?;
        template.output(card, &img, &ctx.backend)?;
        Ok(())
    }
}
