//! Friendly stage logging + progress bars (stderr only).
//!
//! - Interactive TTY: `indicatif` bars/spinners (throttled ticks; page-level updates).
//! - Piped / CI: plain `[stage] …` lines so progress stays visible without ANSI noise.
//!
//! Never updates per OCR character — only per stage / page — so cost is negligible.

use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::time::Duration;

/// Named pipeline stages shown to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Init,
    Cache,
    Render,
    Preprocess,
    Ocr,
    Structure,
    Classify,
    Rollup,
    Write,
    Done,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Init => "init",
            Stage::Cache => "cache",
            Stage::Render => "render",
            Stage::Preprocess => "preprocess",
            Stage::Ocr => "ocr",
            Stage::Structure => "structure",
            Stage::Classify => "classify",
            Stage::Rollup => "rollup",
            Stage::Write => "write",
            Stage::Done => "done",
        }
    }
}

/// Progress / info sink used by [`crate::Engine`].
pub trait Reporter: Send + Sync {
    fn info(&self, stage: Stage, msg: &str);
    fn begin_bar(&self, stage: Stage, len: u64, msg: &str) -> Box<dyn ProgressTick>;
    fn begin_spinner(&self, stage: Stage, msg: &str) -> Box<dyn ProgressTick>;
}

/// Handle for a running bar or spinner.
pub trait ProgressTick: Send {
    fn set_message(&mut self, msg: &str);
    fn inc(&mut self, n: u64);
    fn finish(&mut self, msg: &str);
}

/// No output (for `--quiet` / scripting).
#[derive(Debug, Default, Clone, Copy)]
pub struct QuietReporter;

impl Reporter for QuietReporter {
    fn info(&self, _stage: Stage, _msg: &str) {}
    fn begin_bar(&self, _stage: Stage, _len: u64, _msg: &str) -> Box<dyn ProgressTick> {
        Box::new(NoopTick)
    }
    fn begin_spinner(&self, _stage: Stage, _msg: &str) -> Box<dyn ProgressTick> {
        Box::new(NoopTick)
    }
}

struct NoopTick;
impl ProgressTick for NoopTick {
    fn set_message(&mut self, _msg: &str) {}
    fn inc(&mut self, _n: u64) {}
    fn finish(&mut self, _msg: &str) {}
}

/// Plain-line tick for non-TTY (logs start + finish only; `inc` is silent).
struct LineTick {
    stage: Stage,
    finished: bool,
}

impl ProgressTick for LineTick {
    fn set_message(&mut self, _msg: &str) {}
    fn inc(&mut self, _n: u64) {}
    fn finish(&mut self, msg: &str) {
        if self.finished {
            return;
        }
        self.finished = true;
        eprintln!("[{:>11}] {}", self.stage.label(), msg);
    }
}

/// Human-friendly stderr reporter with progress bars on a TTY.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConsoleReporter;

impl ConsoleReporter {
    fn tty() -> bool {
        io::stderr().is_terminal()
    }
}

impl Reporter for ConsoleReporter {
    fn info(&self, stage: Stage, msg: &str) {
        eprintln!("[{:>11}] {}", stage.label(), msg);
    }

    fn begin_bar(&self, stage: Stage, len: u64, msg: &str) -> Box<dyn ProgressTick> {
        if len == 0 {
            self.info(stage, msg);
            return Box::new(NoopTick);
        }
        if !Self::tty() {
            self.info(stage, &format!("start · {msg} (0/{len})"));
            return Box::new(LineTick {
                stage,
                finished: false,
            });
        }
        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::with_template(
                "[{prefix:>11}] {bar:32.cyan/blue} {pos}/{len} {elapsed_precise} {msg}",
            )
            .expect("progress template")
            .progress_chars("##-"),
        );
        pb.set_prefix(stage.label());
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(200));
        Box::new(BarTick { pb, finished: false })
    }

    fn begin_spinner(&self, stage: Stage, msg: &str) -> Box<dyn ProgressTick> {
        if !Self::tty() {
            self.info(stage, &format!("start · {msg}"));
            return Box::new(LineTick {
                stage,
                finished: false,
            });
        }
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "[{prefix:>11}] {spinner:.cyan} {elapsed_precise} {msg}",
            )
            .expect("spinner template")
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        pb.set_prefix(stage.label());
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(120));
        Box::new(BarTick { pb, finished: false })
    }
}

struct BarTick {
    pb: ProgressBar,
    finished: bool,
}

impl ProgressTick for BarTick {
    fn set_message(&mut self, msg: &str) {
        self.pb.set_message(msg.to_string());
    }

    fn inc(&mut self, n: u64) {
        self.pb.inc(n);
    }

    fn finish(&mut self, msg: &str) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.pb.finish_with_message(msg.to_string());
    }
}

impl Drop for BarTick {
    fn drop(&mut self) {
        if !self.finished {
            self.finished = true;
            self.pb.finish_and_clear();
        }
    }
}

/// Build the default reporter for CLI use.
pub fn default_reporter(quiet: bool) -> Arc<dyn Reporter> {
    if quiet {
        Arc::new(QuietReporter)
    } else {
        Arc::new(ConsoleReporter)
    }
}
