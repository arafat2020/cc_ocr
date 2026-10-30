//! Core engine for pluggable conversation OCR & classification.

pub mod cache;
pub mod engine;
pub mod preprocess;
pub mod profile;
pub mod progress;
pub mod render;
pub mod rollup;

pub use engine::{Engine, EngineOpts};
pub use profile::{default_profile_path, load_profile, SituationProfile};
pub use progress::{default_reporter, ConsoleReporter, QuietReporter, Reporter, Stage};
