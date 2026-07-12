//! Engine-agnostic settings struct. Each engine interprets the fields it cares
//! about and ignores the rest, so adding a new engine does not require touching
//! every call site.

use serde::{Deserialize, Serialize};

/// Settings passed to a [`crate::traits::CompressionEngine::optimize`] call.
///
/// All fields are best-effort: engines may interpret them differently.
///
/// | Field      | Used by        | Notes                                    |
/// |------------|----------------|------------------------------------------|
/// | `quality`  | mozjpeg, webp  | 1–100 (100 = lossless).                  |
/// | `lossless` | all engines    | If true, engines should produce lossless output. |
/// | `effort`   | oxipng, webp   | 1–10, higher = more CPU, smaller output. |
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EngineSettings {
    /// Target quality, 1–100. Higher is better quality, larger file.
    pub quality: u8,
    /// Whether the engine should produce lossless output. Wins over `quality`.
    pub lossless: bool,
    /// Encoder effort, 1–10. Higher means more CPU spent searching for savings.
    pub effort: u8,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            quality: 85,
            lossless: false,
            effort: 6,
        }
    }
}

impl EngineSettings {
    /// Sane defaults for a "high quality, balanced effort" compression run.
    pub const fn balanced() -> Self {
        Self {
            quality: 85,
            lossless: false,
            effort: 6,
        }
    }

    /// Clamp every field to its valid range. Always returns a usable value
    /// — invalid inputs are repaired, not rejected.
    pub fn sanitized(self) -> Self {
        Self {
            quality: self.quality.clamp(1, 100),
            effort: self.effort.clamp(1, 10),
            lossless: self.lossless,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_balanced() {
        assert_eq!(EngineSettings::default(), EngineSettings::balanced());
    }

    #[test]
    fn sanitized_clamps_to_valid_range() {
        let s = EngineSettings {
            quality: 255,
            lossless: false,
            effort: 0,
        };
        let clamped = s.sanitized();
        assert_eq!(clamped.quality, 100);
        assert_eq!(clamped.effort, 1);
    }

    #[test]
    fn sanitized_clamps_high_effort_too() {
        let s = EngineSettings {
            quality: 50,
            lossless: true,
            effort: 99,
        };
        let clamped = s.sanitized();
        assert_eq!(clamped.quality, 50);
        assert_eq!(clamped.effort, 10);
        assert!(clamped.lossless);
    }
}
