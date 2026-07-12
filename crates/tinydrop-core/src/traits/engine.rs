//! The `CompressionEngine` trait. Adding a new engine must require only a new
//! struct implementing this trait — no changes elsewhere. See `ARCHITECTURE.md`.

use std::path::Path;

use crate::domain::{CompressionResult, EngineSettings, ImageFormat};
use crate::errors::CoreResult;

/// A pluggable image compression engine.
///
/// Implementors are expected to be cheap to construct (cheap enough that we
/// hold one per format inside [`AdaptiveOptimizer`](crate::optimizer::AdaptiveOptimizer))
/// and `Send + Sync` so they can be moved into rayon workers.
pub trait CompressionEngine: Send + Sync {
    /// Stable, human-readable name. Used in logs, history rows, and the UI
    /// ("WebP beat PNG by 34%"). Should match the crate name and be lowercase.
    fn name(&self) -> &'static str;

    /// Image formats this engine can compress. The adaptive optimizer filters
    /// the engine registry by this list before parallel-scoring candidates.
    fn supported_formats(&self) -> &[ImageFormat];

    /// Whether this engine can produce lossless output for the given format.
    /// The adaptive optimizer uses this to avoid scoring lossy engines against
    /// lossless-only presets.
    fn supports_lossless(&self, format: ImageFormat) -> bool;

    /// Compress `input` and write the optimized file to `output`. The engine
    /// must create `output` atomically — partial files left behind on error
    /// are a UX bug.
    fn optimize(
        &self,
        input: &Path,
        output: &Path,
        settings: &EngineSettings,
    ) -> CoreResult<CompressionResult>;
}
