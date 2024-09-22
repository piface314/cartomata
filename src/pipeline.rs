mod parallel;
mod sequential;

use crate::data::Card;
use crate::error::{Error, Result};
use crate::logs::{LogMsg, ProgressBar};
pub use crate::pipeline::parallel::ParallelismOptions;
use crate::template::Template;

use std::marker::PhantomData;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

pub struct Pipeline<C: Card, T: Template<C>, V: Visitor<C, T> = ()> {
    pub(crate) template: T,
    pub(crate) visitor: V,
    _card: PhantomData<C>,
}

impl<C: Card, T: Template<C>, V: Visitor<C, T>> Pipeline<C, T, V> {
    pub fn new(template: T, visitor: V) -> Self {
        Self { template, visitor, _card: PhantomData }
    }
}

#[allow(unused_variables)]
pub trait Visitor<C: Card, T: Template<C>> {
    fn on_start(&self, template: &T, worker: usize) {}

    fn on_total(&self, template: &T, total: usize) {}

    fn on_read(&self, template: &T, card: &Result<C>) -> bool {
        true
    }

    fn on_read_err(&self, template: &T, i: usize, error: Error) {}

    fn on_iter_start(&self, template: &T, worker: usize, i: usize, card: &C) {}

    fn on_iter_ok(&self, template: &T, worker: usize, i: usize, card: C) {}

    fn on_iter_err(&self, template: &T, worker: usize, i: usize, card: C, error: Error) {}

    fn on_finish(&self, template: &T, worker: usize, result: &Result<()>) {}
}

impl<C: Card, T: Template<C>> Visitor<C, T> for () {}

#[derive(Debug, Clone)]
pub struct LogVisitor {
    tx: Sender<LogMsg>,
}

impl LogVisitor {
    pub fn new(n_workers: usize) -> (Self, JoinHandle<Result<()>>) {
        let (tx, handle) = ProgressBar::spawn_stderr(n_workers);
        (Self { tx }, handle)
    }

    fn log(&self, msg: LogMsg) {
        self.tx.send(msg).unwrap_or(())
    }
}

impl<C: Card, T: Template<C>> Visitor<C, T> for LogVisitor {
    fn on_start(&self, template: &T, worker: usize) {
        if worker > 0 {
            return;
        }
        match template.name() {
            Some(name) => self.log(LogMsg::Running(
                0,
                format!(
                    "running template {}{name}{}",
                    termion::color::Fg(termion::color::LightYellow),
                    termion::style::Reset
                ),
            )),
            None => {}
        }
    }

    fn on_total(&self, _template: &T, total: usize) {
        self.log(LogMsg::Total(total))
    }

    fn on_read_err(&self, _template: &T, i: usize, error: Error) {
        self.log(LogMsg::Warn(
            0,
            format!("failed to read card (#{i}): {error}"),
        ));
    }

    fn on_iter_start(&self, template: &T, worker: usize, i: usize, card: &C) {
        let card_id = template.identify(card);
        self.log(LogMsg::Running(
            worker,
            format!("processing card {card_id} (#{i})..."),
        ))
    }

    fn on_iter_ok(&self, _template: &T, worker: usize, _i: usize, _card: C) {
        self.log(LogMsg::Progress(worker));
    }

    fn on_iter_err(&self, template: &T, worker: usize, i: usize, card: C, error: Error) {
        let card_id = template.identify(&card);
        self.log(LogMsg::Warn(
            worker,
            format!("failed to process card {card_id} (#{i}): {error}"),
        ))
    }

    fn on_finish(&self, template: &T, worker: usize, result: &Result<()>) {
        let msg = match (result, worker, template.name()) {
            (Ok(()), 0, Some(name)) => LogMsg::Success(
                worker,
                format!(
                    "finished template {}{name}{}!",
                    termion::color::Fg(termion::color::LightYellow),
                    termion::style::Reset
                ),
            ),
            (Ok(()), _, _) => LogMsg::Success(worker, String::from("done!")),
            (Err(e), _, _) => LogMsg::Error(worker, e.to_string()),
        };
        self.log(msg)
    }
}
