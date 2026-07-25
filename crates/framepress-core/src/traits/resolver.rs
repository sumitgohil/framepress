//! The `PresetResolver` trait. Maps a (preset, format) pair to concrete
//! [`EngineSettings`] an engine will accept.

use crate::domain::{CompressionPreset, EngineSettings, ImageFormat};

/// Resolves a preset to concrete settings for a given input format.
///
/// Splitting preset knowledge out of the engine trait is deliberate: it keeps
/// engines unaware of "preset" as a concept, so a future custom-preset editor
/// doesn't require touching any engine code.
pub trait PresetResolver: Send + Sync {
    /// Resolve `preset` to settings appropriate for `format`.
    fn resolve(&self, preset: CompressionPreset, format: ImageFormat) -> EngineSettings;
}
