//! The 6 built-in presets and the resolver implementation.

use crate::domain::{CompressionPreset, EngineSettings, PresetSpec, PRESETS};
use crate::traits::PresetResolver;

/// Look up the [`PresetSpec`] for a built-in preset. Panics if the preset is
/// not one of the six — which is impossible, but the API still benefits from
/// the explicit lookup table.
pub fn builtin_spec(preset: CompressionPreset) -> &'static PresetSpec {
    PRESETS
        .iter()
        .find(|s| s.preset == preset)
        .expect("every CompressionPreset variant must appear in PRESETS")
}

/// Resolves built-in presets to [`EngineSettings`]. The default Phase 1
/// resolver — `AdaptiveOptimizer` and the queue layer both hold one of these.
#[derive(Debug, Default, Clone)]
pub struct BuiltinPresetResolver;

impl BuiltinPresetResolver {
    /// Construct a resolver. Cheap.
    pub fn new() -> Self {
        Self
    }
}

impl PresetResolver for BuiltinPresetResolver {
    fn resolve(
        &self,
        preset: CompressionPreset,
        _format: crate::domain::ImageFormat,
    ) -> EngineSettings {
        // v1: same settings regardless of input format. The adaptive
        // optimizer filters by capability at scoring time, so any per-format
        // tuning would happen there, not here.
        let spec = builtin_spec(preset);
        EngineSettings {
            quality: spec.quality,
            lossless: spec.lossless,
            effort: spec.effort,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ImageFormat;

    #[test]
    fn resolver_returns_preset_settings() {
        let r = BuiltinPresetResolver::new();
        let s = r.resolve(CompressionPreset::Lossless, ImageFormat::Png);
        assert!(s.lossless);
        assert_eq!(s.quality, 100);
    }

    #[test]
    fn resolver_for_email_is_aggressive() {
        let r = BuiltinPresetResolver::new();
        let s = r.resolve(CompressionPreset::Email, ImageFormat::Jpeg);
        assert!(!s.lossless);
        assert!(s.quality <= 75);
        assert!(s.effort >= 7);
    }

    #[test]
    fn builtin_specs_cover_all_presets() {
        for p in CompressionPreset::ALL {
            let _ = builtin_spec(*p);
        }
    }
}
