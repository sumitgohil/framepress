//! Free-function resolver for one-off lookups. Most code should hold a
//! [`BuiltinPresetResolver`](crate::presets::BuiltinPresetResolver) instead.

use crate::domain::{CompressionPreset, EngineSettings, ImageFormat};
use crate::presets::builtin_spec;

/// Resolve `preset` for `format` without going through the trait object.
///
/// Equivalent to `BuiltinPresetResolver::resolve`. Useful in tests and one-off
/// conversion scripts.
pub fn resolve(preset: CompressionPreset, _format: ImageFormat) -> EngineSettings {
    let spec = builtin_spec(preset);
    EngineSettings {
        quality: spec.quality,
        lossless: spec.lossless,
        effort: spec.effort,
    }
}
