//! # TinyDrop core
//!
//! `tinydrop-core` owns **all** TinyDrop business logic. The Tauri/Svelte layer
//! only renders state and calls into this crate via Tauri commands.
//!
//! ## Architecture
//!
//! - [`domain`] — pure data types (formats, presets, results). No I/O.
//! - [`traits`] — engine and resolver traits. New engines slot in by implementing
//!   [`traits::CompressionEngine`]; nothing else needs to change.
//! - [`engines`] — the three Phase 1 engine wrappers (`oxipng`, `mozjpeg`, `webp`).
//! - [`presets`] — the six built-in compression presets and a resolver that maps
//!   `(preset, format)` to engine settings.
//! - [`optimizer`] — the adaptive optimizer that runs candidate engines in
//!   parallel and picks the winner against a DSSIM quality gate.
//! - [`queue`], [`history`], [`settings`] — orchestration around the engines
//!   (filled out in later Phase 1 branches).
//!
//! ## Async boundary
//!
//! `tokio` owns I/O, orchestration, and Tauri command handlers. `rayon` owns
//! CPU-bound compression work and is reached via `tokio::task::spawn_blocking`.
//! Compression is **never** called directly from an async context.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod domain;
pub mod engines;
pub mod errors;
pub mod history;
pub mod optimizer;
pub mod presets;
pub mod queue;
pub mod traits;

pub use domain::{
    CompressionPreset, CompressionResult, EngineSettings, ImageFormat, PresetSpec, QualityGate,
    ScoredCandidate,
};
pub use engines::{
    default_registry, engines_for_format, MozJpegEngine, OxipngEngine, PngQuantEngine, WebPEngine,
};
pub use errors::{CoreError, CoreResult};
pub use optimizer::AdaptiveOptimizer;
pub use presets::BuiltinPresetResolver;
pub use queue::{JobStatus, QueueItem, QueueProcessor, QueueStats};
pub use traits::{CompressionEngine, HistoryRepository, PresetResolver};
