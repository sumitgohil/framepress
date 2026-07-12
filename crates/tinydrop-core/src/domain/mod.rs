//! Pure domain types. No I/O, no engine awareness, no serialization to Tauri.

mod format;
mod preset;
mod result;
mod settings;

pub use format::ImageFormat;
pub use preset::{CompressionPreset, PresetSpec, PRESETS};
pub use result::{CompressionResult, QualityGate, ScoredCandidate};
pub use settings::EngineSettings;
