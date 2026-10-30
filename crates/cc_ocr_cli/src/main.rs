//! CLI: load situation profile → build engine → run.

use anyhow::{Context, Result};
use cc_ocr_core::{default_profile_path, default_reporter, load_profile, EngineOpts, Stage};
use cc_ocr_plugin_api::DEFAULT_DPI;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod registry;

#[derive(Parser, Debug)]
#[command(
    name = "cc_ocr",
    about = "Pluggable conversation chat OCR & classification engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Situation profile TOML (plugins + params). Default: situations/fiverr_inbox.toml
    #[arg(long, global = true)]
    situation: Option<PathBuf>,

    /// Render DPI (PDF only); overrides profile when set
    #[arg(long, global = true, default_value_t = DEFAULT_DPI)]
    dpi: u32,

    /// Ignore JSON cache and re-OCR
    #[arg(long, global = true)]
    force: bool,

    /// Keep temporary page images under out/tmp
    #[arg(long, global = true)]
    keep_tmp: bool,

    /// Python interpreter for tf_sidecar (overrides profile)
    #[arg(long, global = true)]
    python: Option<String>,

    /// Suppress stage logs and progress bars (stdout paths unchanged)
    #[arg(long, short = 'q', global = true)]
    quiet: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Stages 1–4: render → preprocess → OCR → structure → out/cache
    Ocr { input: PathBuf },
    /// Classify messages from a cache JSON (or PDF/PNG via OCR first)
    Classify { input: PathBuf },
    /// Full pipeline: OCR + classify + rollup → out/labels
    Run { input: PathBuf },
    /// Recompute rollup from an existing labels JSON (prints to stdout)
    Rollup { labels_json: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let reporter = default_reporter(cli.quiet);

    let profile_path = cli.situation.clone().unwrap_or_else(default_profile_path);
    let mut profile = load_profile(&profile_path)
        .with_context(|| format!("load situation profile {}", profile_path.display()))?;

    if let Some(python) = &cli.python {
        profile.params.python = python.clone();
    }

    let opts = EngineOpts {
        dpi: if cli.dpi != DEFAULT_DPI {
            cli.dpi
        } else {
            profile.params.dpi
        },
        force: cli.force,
        keep_tmp: cli.keep_tmp,
    };

    let engine = registry::build_engine(&profile, opts, reporter.clone())?;
    reporter.info(
        Stage::Init,
        &format!(
            "situation={} ocr={} classifier={} · {}",
            engine.situation.id(),
            engine.ocr.id(),
            engine.classifier.id(),
            profile_path.display()
        ),
    );

    match cli.command {
        Commands::Ocr { input } => {
            let (_conv, path) = engine.run_ocr(&input)?;
            println!("{}", path.display());
        }
        Commands::Classify { input } => {
            let (_result, path) = engine.run_classify(&input)?;
            println!("{}", path.display());
        }
        Commands::Run { input } => {
            let (_result, path) = engine.run_full(&input)?;
            println!("{}", path.display());
        }
        Commands::Rollup { labels_json } => {
            let roll = engine.run_rollup(&labels_json)?;
            println!("{}", serde_json::to_string_pretty(&roll)?);
        }
    }

    Ok(())
}
