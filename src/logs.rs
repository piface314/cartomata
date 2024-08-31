use std::io::{stderr, Error as IoError, Stderr, Write};
use std::num::NonZero;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum LogEvent {
    Count(usize),
    Total(usize),
    Info(usize, String),
    Warn(usize, String),
    Status(usize, String),
    Error(usize, String),
    Done(usize, String),
}

#[derive(Debug, Clone)]
enum WorkerStatus {
    Running(String),
    Failed(String),
    Done(String),
}

impl Default for WorkerStatus {
    fn default() -> Self {
        Self::Running(String::new())
    }
}

#[derive(Debug, Clone)]
pub struct ProgressBar<T: Write> {
    n_workers: usize,
    tty: T,
    status: Vec<WorkerStatus>,
    counts: Vec<usize>,
    total: usize,
    frame: usize,
    time: Instant,
}

impl ProgressBar<Stderr> {
    pub fn new_stderr(n_workers: NonZero<usize>) -> Result<Self, IoError> {
        Self::new(n_workers, stderr())
    }
}

impl<T: Write> ProgressBar<T> {
    const BAR_WIDTH: usize = 16;
    const WORKER_BAR_WIDTH: usize = 8;
    const WORKER_BAR_FACTOR: usize = 3;
    const FRAME_DURATION: f64 = 0.1;
    const FRAME_COUNT: usize = 256;

    pub fn new(n_workers: NonZero<usize>, tty: T) -> Result<Self, IoError> {
        let n_workers = n_workers.get();
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
            write!(pbar.tty, "\n")?;
        }
        pbar.show()?;
        Ok(pbar)
    }

    pub fn set_total(&mut self, total: usize) {
        self.total = total;
    }

    pub fn log(&mut self, event: LogEvent) -> Result<(), IoError> {
        match event {
            LogEvent::Info(id, msg) => {
                self.log_message(id, "INFO", msg, termion::color::LightBlack)?
            }
            LogEvent::Warn(id, msg) => {
                self.log_message(id, "WARN", msg, termion::color::LightYellow)?
            }
            LogEvent::Status(id, msg) => {
                self.status[id] = WorkerStatus::Running(msg);
            }
            LogEvent::Error(id, msg) => {
                self.status[id] = WorkerStatus::Failed(msg);
            }
            LogEvent::Done(id, msg) => {
                self.status[id] = WorkerStatus::Done(msg);
            }
            LogEvent::Count(0) => {
                self.counts[0] += 1;
            }
            LogEvent::Count(id) => {
                self.counts[0] += 1;
                self.counts[id] += 1;
            }
            LogEvent::Total(n) => self.set_total(n),
        };
        self.show()?;
        Ok(())
    }

    fn log_message(
        &mut self,
        id: usize,
        label: &'static str,
        msg: String,
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
                "{up}{goto}{id_color}{id:02} {color}[{label}] {reset}{msg}{clear}"
            )?;
        } else {
            write!(
                self.tty,
                "{up}{goto}{id_color}   {color}[{label}] {reset}{msg}{clear}"
            )?;
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), IoError> {
        let now = Instant::now();
        let dt = now.duration_since(self.time).as_secs_f64();
        if dt >= Self::FRAME_DURATION {
            self.time = now;
            self.frame = (self.frame + 1) % Self::FRAME_COUNT;
            self.show()?;
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
            WorkerStatus::Failed(msg) => (
                "!".repeat(Self::WORKER_BAR_WIDTH),
                termion::color::LightRed.fg_str(),
                msg,
            ),
            WorkerStatus::Done(msg) => (
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
            WorkerStatus::Failed(msg) => (
                "!".repeat(Self::BAR_WIDTH),
                termion::color::LightRed.fg_str(),
                msg,
            ),
            WorkerStatus::Done(msg) => (
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

    fn ellipsize(s: &String, w: u16, used: u16) -> String {
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
            s.clone()
        }
    }
}
