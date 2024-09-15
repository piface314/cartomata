use crate::error::Error;

use std::io::{stderr, Error as IoError, Stderr, Write};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum LogMsg {
    Total(usize),
    Progress(usize),
    Info(usize, String),
    Warn(usize, String),
    Running(usize, String),
    Error(usize, String),
    Success(usize, String),
}

#[derive(Debug, Clone)]
enum WorkerStatus {
    Running(String),
    Error(String),
    Success(String),
}

impl Default for WorkerStatus {
    fn default() -> Self {
        Self::Running(String::new())
    }
}

#[derive(Debug, Clone)]
pub struct ProgressBar<T: Write + Send> {
    n_workers: usize,
    tty: T,
    status: Vec<WorkerStatus>,
    counts: Vec<usize>,
    total: usize,
    frame: usize,
    time: Instant,
}

impl ProgressBar<Stderr> {
    pub fn new_stderr(n_workers: usize) -> Result<Self, Error> {
        Self::new(n_workers, stderr())
    }

    pub fn spawn_stderr(n_workers: usize)-> (Sender<LogMsg>, JoinHandle<Result<(), Error>>) {
        Self::spawn(n_workers, stderr())
    }
}

impl<T: Write + Send + 'static> ProgressBar<T> {
    const BAR_WIDTH: usize = 16;
    const WORKER_BAR_WIDTH: usize = 8;
    const WORKER_BAR_FACTOR: usize = 3;
    const FRAME_DURATION: f64 = 0.1;
    const FRAME_COUNT: usize = 256;

    pub fn spawn(n_workers: usize, tty: T) -> (Sender<LogMsg>, JoinHandle<Result<(), Error>>) {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut pbar = Self::new(n_workers, tty)?;
            loop {
                match rx.try_recv() {
                    Ok(LogMsg::Total(id)) => {
                        pbar.set_total(id);
                    },
                    Ok(LogMsg::Progress(id)) => {
                        pbar.progress(id);
                    },
                    Ok(LogMsg::Info(id, msg)) => {
                        pbar.info(id, msg)?;
                    },
                    Ok(LogMsg::Warn(id, msg)) => {
                        pbar.warn(id, msg)?;
                    },
                    Ok(LogMsg::Running(id, msg)) => {
                        pbar.running(id, msg);
                    },
                    Ok(LogMsg::Error(id, msg)) => {
                        pbar.error(id, msg);
                    },
                    Ok(LogMsg::Success(id, msg)) => {
                        pbar.success(id, msg);
                    },
                    Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(10)),
                    Err(TryRecvError::Disconnected) => break,
                }
                pbar.update()?;
            }
            pbar.show().map_err(Error::io_error)
        });
        (tx, handle)
    }

    pub fn new(n_workers: usize, tty: T) -> Result<Self, Error> {

        let mut pbar = Self {
            n_workers,
            tty,
            status: vec![WorkerStatus::default(); n_workers + 1],
            counts: vec![0; n_workers + 1],
            total: 0,
            frame: 0,
            time: Instant::now(),
        };
        for _ in 0..=n_workers {
            write!(pbar.tty, "\n").map_err(Error::io_error)?;
        }
        pbar.show().map_err(Error::io_error)?;
        Ok(pbar)
    }

    pub fn set_total(&mut self, total: usize) {
        self.total = total;
    }

    pub fn progress(&mut self, id: usize) {
        if id > 0 {
            self.counts[id] += 1;
        }
        self.counts[0] += 1;
    }

    pub fn info(&mut self, id: usize, msg: String) -> Result<(), Error> {
        self.log_message(id, "INFO", &msg, termion::color::LightBlack)
            .map_err(Error::io_error)
    }

    pub fn warn(&mut self, id: usize, msg: String) -> Result<(), Error> {
        self.log_message(id, "WARN", &msg, termion::color::LightYellow)
            .map_err(Error::io_error)
    }

    pub fn running(&mut self, id: usize, msg: String) {
        self.status[id] = WorkerStatus::Running(msg);
    }

    pub fn error(&mut self, id: usize, msg: String) {
        self.status[id] = WorkerStatus::Error(msg);
    }

    pub fn success(&mut self, id: usize, msg: String) {
        self.status[id] = WorkerStatus::Success(msg);
    }

    fn log_message(
        &mut self,
        id: usize,
        label: &'static str,
        msg: &str,
        color: impl termion::color::Color,
    ) -> Result<(), IoError> {
        let (_w, h) = termion::terminal_size()?;
        let nl = msg.chars().filter(|c| *c == '\n').count() as u16;
        let msg = msg
            .replace("\t", "    ")
            .replace("\n", &format!("{}\n", termion::clear::UntilNewline));
        let y = h - self.n_workers as u16 - (2 + nl);
        let up = termion::scroll::Up(1 + nl);
        let goto = termion::cursor::Goto(1, y);
        let id_color = termion::color::Fg(termion::color::LightBlack);
        let color = termion::color::Fg(color);
        let reset = termion::style::Reset;
        let clear = termion::clear::UntilNewline;
        if id > 0 {
            write!(
                self.tty,
                "{up}{goto}{id_color}{id:02} {color}[{label}] {reset}{msg}{clear}\n"
            )?;
        } else {
            let pad = if self.n_workers > 0 { "   " } else { "" };
            write!(
                self.tty,
                "{up}{goto}{id_color}{pad}{color}[{label}] {reset}{msg}{clear}\n"
            )?;
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Error> {
        let now = Instant::now();
        let dt = now.duration_since(self.time).as_secs_f64();
        if dt >= Self::FRAME_DURATION {
            self.time = now;
            self.frame = (self.frame + 1) % Self::FRAME_COUNT;
            self.show().map_err(Error::io_error)?;
        }
        Ok(())
    }

    fn show(&mut self) -> Result<(), IoError> {
        let (w, h) = termion::terminal_size()?;
        let y = h - self.n_workers as u16 - 1;
        write!(self.tty, "{}", termion::cursor::Goto(1, y))?;
        for id in 1..=self.n_workers {
            self.show_worker(w, id)?;
        }
        self.show_base(w)?;
        Ok(())
    }

    fn show_worker(&mut self, w: u16, id: usize) -> Result<(), IoError> {
        let (label, color, msg) = match &self.status[id] {
            WorkerStatus::Running(msg) => {
                let i = (self.frame + id * Self::WORKER_BAR_FACTOR) % Self::WORKER_BAR_WIDTH;
                let arrows = format!(
                    "{}▶{}",
                    "▷".repeat(i),
                    "▷".repeat(Self::WORKER_BAR_WIDTH - 1 - i)
                );
                (arrows, termion::color::Blue.fg_str(), msg)
            }
            WorkerStatus::Error(msg) => (
                "!".repeat(Self::WORKER_BAR_WIDTH),
                termion::color::LightRed.fg_str(),
                msg,
            ),
            WorkerStatus::Success(msg) => (
                "-".repeat(Self::WORKER_BAR_WIDTH),
                termion::color::LightGreen.fg_str(),
                msg,
            ),
        };
        let id_color = termion::color::Fg(termion::color::LightBlack);
        let msg = Self::ellipsize(msg, w, 18);
        let reset = termion::style::Reset;
        let clear = termion::clear::UntilNewline;
        let n = self.counts[id];
        write!(
            self.tty,
            "{id_color}{id:02} {color}[{label} {n:3}] {reset}{msg}{clear}\n",
        )?;
        Ok(())
    }

    fn show_base(&mut self, w: u16) -> Result<(), IoError> {
        let n = self.counts[0];
        let total = self.total;
        let (label, color, msg) = match &self.status[0] {
            WorkerStatus::Running(msg) => {
                let arrows = if total > 0 {
                    let done_frac = n as f64 / total as f64;
                    let done_arrows = (done_frac * Self::BAR_WIDTH as f64).round() as usize;
                    format!(
                        "{}{}",
                        "▶".repeat(done_arrows),
                        "▷".repeat(Self::BAR_WIDTH.saturating_sub(done_arrows))
                    )
                } else {
                    let mut arrows = vec!['▷'; Self::BAR_WIDTH];
                    let index = self.frame % Self::BAR_WIDTH;
                    for i in index..index + 3 {
                        arrows[i % Self::BAR_WIDTH] = '▶';
                    }
                    arrows.into_iter().collect::<String>()
                };
                (arrows, termion::color::LightBlue.fg_str(), msg)
            }
            WorkerStatus::Error(msg) => (
                "!".repeat(Self::BAR_WIDTH),
                termion::color::LightRed.fg_str(),
                msg,
            ),
            WorkerStatus::Success(msg) => (
                "=".repeat(Self::BAR_WIDTH),
                termion::color::LightGreen.fg_str(),
                msg,
            ),
        };
        let msg = Self::ellipsize(msg, w, 27);
        let reset = termion::style::Reset;
        let clear = termion::clear::UntilNewline;

        if total > 0 {
            write!(
                self.tty,
                "{color}[{label} {n:3}/{total:3}] {reset}{msg}{clear}\n",
            )?;
        } else {
            write!(self.tty, "{color}[{label} {n:3}] {reset}{msg}{clear}\n",)?;
        }
        Ok(())
    }

    fn ellipsize(s: &str, w: u16, used: u16) -> String {
        let w = w - used;
        let len = s.chars().count();
        if len >= w as usize {
            format!(
                "{}...",
                s.chars()
                    .take((w.saturating_sub(4)) as usize)
                    .collect::<String>()
            )
        } else {
            s.to_string()
        }
    }
}
