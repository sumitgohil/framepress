//! Result types emitted by engines and the adaptive optimizer.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::ImageFormat;

/// A single engine's output for a given input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Engine that produced this result, e.g. `"oxipng"`. Owned because
    /// serialization to the frontend needs a `String`, and engine names from
    /// user input (e.g. the optimizer's "not registered" error) are runtime.
    pub engine: String,
    /// Path of the optimized file on disk.
    pub output_path: PathBuf,
    /// Format of the input file (and of the output, for Phase 1).
    pub format: ImageFormat,
    /// Bytes of the input file.
    pub original_bytes: u64,
    /// Bytes of the optimized file. `0` if the engine failed mid-run.
    pub optimized_bytes: u64,
    /// Perceptual distance (DSSIM) between input and output, in `[0.0, 1.0]`.
    /// `None` when the comparison was not run (e.g. the engine only does
    /// lossless re-pack and no pixel comparison was needed).
    pub dssim: Option<f64>,
    /// Wall-clock duration of the optimization, in milliseconds.
    pub duration_ms: u64,
}

impl CompressionResult {
    /// `true` if the engine actually produced an output (non-empty file).
    pub fn is_ok(&self) -> bool {
        self.optimized_bytes > 0
    }

    /// Percentage reduction in bytes, clamped to `[0.0, 100.0]`. Returns `0.0`
    /// when the input was already empty.
    pub fn savings_pct(&self) -> f64 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        let saved = self.original_bytes as f64 - self.optimized_bytes as f64;
        ((saved / self.original_bytes as f64) * 100.0).clamp(0.0, 100.0)
    }
}

/// A candidate result augmented with its score after the quality gate.
///
/// This is the type the adaptive optimizer emits when reporting its scoring
/// pass; it carries both the raw result and the metadata the UI needs to tell
/// the "WebP beat PNG by 34%" story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredCandidate {
    /// The underlying engine result.
    pub result: CompressionResult,
    /// Whether this candidate passed the configured DSSIM quality gate.
    pub passed_quality_gate: bool,
    /// Savings margin over the runner-up, expressed as a percentage of the
    /// runner-up's size. `None` if there is no runner-up.
    pub margin_pct_vs_runner_up: Option<f64>,
}

/// Configuration for the DSSIM quality gate used by the adaptive optimizer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QualityGate {
    /// Maximum acceptable DSSIM. A candidate with measured DSSIM above this
    /// value is rejected. `0.001` is a sane starting default.
    pub max_dssim: f64,
}

impl Default for QualityGate {
    fn default() -> Self {
        // 0.001 ≈ "imperceptible to the human eye" for most photo content.
        // Power users can lower this in Settings.
        Self { max_dssim: 0.001 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_result(orig: u64, opt: u64) -> CompressionResult {
        CompressionResult {
            engine: "test".to_string(),
            output_path: PathBuf::from("/tmp/out.png"),
            format: ImageFormat::Png,
            original_bytes: orig,
            optimized_bytes: opt,
            dssim: Some(0.0),
            duration_ms: 10,
        }
    }

    #[test]
    fn savings_pct_handles_normal_case() {
        let r = fixture_result(1000, 250);
        assert!((r.savings_pct() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn savings_pct_handles_larger_output_clamped_to_zero() {
        let r = fixture_result(1000, 1500);
        assert_eq!(r.savings_pct(), 0.0);
    }

    #[test]
    fn savings_pct_handles_zero_input() {
        let r = fixture_result(0, 0);
        assert_eq!(r.savings_pct(), 0.0);
    }

    #[test]
    fn is_ok_reflects_optimized_size() {
        assert!(fixture_result(100, 50).is_ok());
        assert!(!fixture_result(100, 0).is_ok());
    }
}
