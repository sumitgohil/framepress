//! Adaptive optimizer. The product's signature component — runs multiple
//! candidate engines in parallel against a single input and picks the
//! smallest output that still passes the DSSIM quality gate.
//!
//! ## Async boundary
//!
//! The optimizer is CPU-bound and synchronous. The Tauri command layer (and
//! the queue processor in Branch 5) wraps the public [`AdaptiveOptimizer::optimize`]
//! call in `tokio::task::spawn_blocking` to keep the runtime responsive.
//!
//! ## Candidate selection
//!
//! For each input format, the optimizer builds a candidate engine list:
//!
//! | Format | Candidates                                            |
//! |--------|-------------------------------------------------------|
//! | PNG    | oxipng (lossless)                                     |
//! | JPEG   | mozjpeg (lossy)                                       |
//! | WebP   | webp re-encode at preset's quality                    |
//! | GIF/SVG| pass-through (no re-encode in Phase 1)                |

mod detection;
mod parallel;
mod scoring;

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::domain::{
    CompressionPreset, CompressionResult, EngineSettings, ImageFormat, QualityGate,
};
use crate::errors::{CoreError, CoreResult};
use crate::optimizer::parallel::{pick_winner, score_candidates, Candidate};
use crate::presets::{builtin_spec, BuiltinPresetResolver};
use crate::traits::{CompressionEngine, PresetResolver};

pub use detection::{detect_format, has_alpha};

/// Adaptive optimizer. Holds the registered engines and a preset resolver.
pub struct AdaptiveOptimizer {
    engines: Vec<Box<dyn CompressionEngine>>,
    resolver: BuiltinPresetResolver,
    quality_gate: QualityGate,
}

impl AdaptiveOptimizer {
    /// Construct an optimizer with the default engine registry.
    pub fn new(engines: Vec<Box<dyn CompressionEngine>>) -> Self {
        Self {
            engines,
            resolver: BuiltinPresetResolver::new(),
            // Presets supply the normal quality gate. This is an optional
            // global ceiling for callers that need a stricter policy.
            quality_gate: QualityGate { max_dssim: 1.0 },
        }
    }

    /// Override the DSSIM quality gate threshold. Typical range: `0.0` (no
    /// perceptual loss) to `0.01` (visibly degraded but tiny files).
    pub fn with_quality_gate(mut self, gate: QualityGate) -> Self {
        self.quality_gate = gate;
        self
    }

    /// The configured quality gate.
    pub fn quality_gate(&self) -> QualityGate {
        self.quality_gate
    }

    /// Engines known to this optimizer.
    pub fn engines(&self) -> &[Box<dyn CompressionEngine>] {
        &self.engines
    }

    /// Resolve the [`EngineSettings`] for `preset` and `format`.
    pub fn resolve_settings(
        &self,
        preset: CompressionPreset,
        format: ImageFormat,
    ) -> EngineSettings {
        self.resolver.resolve(preset, format)
    }

    /// Engines that can compress `format`. Convenience accessor used by the
    /// adaptive scoring loop and the queue validation pass.
    pub fn candidates_for(&self, format: ImageFormat) -> Vec<&dyn CompressionEngine> {
        self.engines
            .iter()
            .filter(|e| e.supported_formats().contains(&format))
            .map(|e| e.as_ref())
            .collect()
    }

    /// Run a single named engine against `input` → `output`. Non-adaptive
    /// path used by tests and the early integration points.
    pub fn run_single(
        &self,
        engine_name: &str,
        input: &Path,
        output: &Path,
        settings: &EngineSettings,
    ) -> CoreResult<CompressionResult> {
        let engine = self
            .engines
            .iter()
            .find(|e| e.name() == engine_name)
            .ok_or_else(|| CoreError::Engine {
                engine: engine_name.to_string(),
                message: format!("engine '{engine_name}' is not registered"),
            })?;
        engine.optimize(input, output, settings)
    }

    /// Detect the input format from a file's contents and resolve its output
    /// path with the standard convention: same stem as input, in the same
    /// directory, suffixed with `-tinydrop`.
    pub fn plan_output_path(input: &Path, format: ImageFormat) -> PathBuf {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        parent.join(format!("{stem}-tinydrop.{}", format.extension()))
    }

    /// Build the candidate list for `format` and `preset`. Used by
    /// [`optimize`](Self::optimize) but exposed for testing.
    pub fn build_candidates(
        &self,
        format: ImageFormat,
        preset: CompressionPreset,
    ) -> Vec<Candidate<'_>> {
        let settings = self.resolve_settings(preset, format);
        let engines = self.candidates_for(format);

        engines
            .into_iter()
            .filter_map(|engine| {
                // Automatic optimization preserves the uploaded format.
                // WebP can decode PNG/JPEG, but it must only be used for an
                // explicit conversion/export request, never silently here.
                if engine.name() == "webp" && format != ImageFormat::WebP {
                    return None;
                }
                // Lossless presets may only use engines that can produce lossless output.
                if settings.lossless && !engine.supports_lossless(format) {
                    return None;
                }
                Some(Candidate {
                    engine,
                    output_format: format,
                    settings,
                })
            })
            .collect()
    }

    /// **The headline API.** Run all eligible candidate engines against
    /// `input` in parallel, gate by DSSIM, return the winner.
    ///
    /// On success, the winning output is written to `final_output`. The
    /// runner-up (if any) is included in the returned
    /// [`ScoredCandidate`](crate::domain::ScoredCandidate) so the UI can show
    /// the margin between the format-preserving candidates.
    pub fn optimize(
        &self,
        input: &Path,
        preset: CompressionPreset,
        final_output: &Path,
    ) -> CoreResult<crate::domain::ScoredCandidate> {
        // 1. Detect input format.
        let format = detection::detect_format(input)?;

        // 2. Pass-through for non-reencode-supported formats.
        if !format.is_reencode_supported() {
            std::fs::copy(input, final_output).map_err(|e| CoreError::Io {
                path: final_output.to_path_buf(),
                source: e,
            })?;
            let bytes = std::fs::metadata(final_output)
                .map_err(|e| CoreError::Io {
                    path: final_output.to_path_buf(),
                    source: e,
                })?
                .len();
            return Ok(crate::domain::ScoredCandidate {
                result: CompressionResult {
                    engine: "passthrough".to_string(),
                    output_path: final_output.to_path_buf(),
                    format,
                    original_bytes: bytes,
                    optimized_bytes: bytes,
                    dssim: Some(0.0),
                    duration_ms: 0,
                },
                passed_quality_gate: true,
                margin_pct_vs_runner_up: None,
            });
        }

        // 3. Build candidates.
        let candidates = self.build_candidates(format, preset);
        if candidates.is_empty() {
            return Err(CoreError::NoWinner);
        }

        // 4. Score in parallel via rayon.
        let scratch = TempDir::new().map_err(|e| CoreError::Io {
            path: input.to_path_buf(),
            source: e,
        })?;
        let scored = score_candidates(input, &candidates, &scratch);

        // Mirror ImageOptim's lossy policy: don't replace a file for a
        // token-size win. A lossy result must save at least 5%; otherwise we
        // keep the original untouched. Lossless presets are allowed to keep
        // any byte-for-byte improvement.
        let scored = if builtin_spec(preset).lossless {
            scored
        } else {
            scored
                .into_iter()
                .filter(|candidate| {
                    candidate.result.optimized_bytes.saturating_mul(100)
                        <= candidate.result.original_bytes.saturating_mul(95)
                })
                .collect()
        };

        // 5. Pick the smallest passing candidate.
        let preset_gate = QualityGate {
            max_dssim: builtin_spec(preset).max_dssim,
        };
        let gate = QualityGate {
            max_dssim: preset_gate.max_dssim.min(self.quality_gate.max_dssim),
        };
        let (mut winner, runner_up) = match pick_winner(scored, gate) {
            Ok(winner) => winner,
            // If every lossy candidate is too large or fails the requested
            // visual-quality gate, preserve the original rather than making
            // the user's file larger or failing the whole queue item.
            Err(CoreError::NoWinner) => return self.copy_original(input, final_output, format),
            Err(error) => return Err(error),
        };

        if winner.result.optimized_bytes >= winner.result.original_bytes {
            return self.copy_original(input, final_output, format);
        }

        // 6. Promote winner's temp file to final_output.
        let output_path = final_output.with_extension(winner.result.format.extension());
        std::fs::copy(&winner.result.output_path, &output_path).map_err(|e| CoreError::Io {
            path: output_path.clone(),
            source: e,
        })?;
        winner.result.output_path = output_path;

        // 7. Compute margin vs runner-up (already done by pick_winner, but
        //    the winner's margin is also stored on the winner itself).
        if let Some(r) = runner_up {
            winner.margin_pct_vs_runner_up = Some(parallel::margin_pct(&winner, &r));
        }

        Ok(winner)
    }

    fn copy_original(
        &self,
        input: &Path,
        final_output: &Path,
        format: ImageFormat,
    ) -> CoreResult<crate::domain::ScoredCandidate> {
        let output_path = final_output.with_extension(format.extension());
        std::fs::copy(input, &output_path).map_err(|e| CoreError::Io {
            path: output_path.clone(),
            source: e,
        })?;
        let bytes = std::fs::metadata(input)
            .map_err(|e| CoreError::Io {
                path: input.to_path_buf(),
                source: e,
            })?
            .len();
        Ok(crate::domain::ScoredCandidate {
            result: CompressionResult {
                engine: "original".to_string(),
                output_path,
                format,
                original_bytes: bytes,
                optimized_bytes: bytes,
                dssim: Some(0.0),
                duration_ms: 0,
            },
            passed_quality_gate: true,
            margin_pct_vs_runner_up: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::default_registry;
    use tempfile::tempdir;

    #[test]
    fn default_registry_produces_optimizer_with_four_engines() {
        let opt = AdaptiveOptimizer::new(default_registry());
        assert_eq!(opt.engines().len(), 4);
    }

    #[test]
    fn png_candidates_preserve_png_output() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let candidates =
            opt.build_candidates(ImageFormat::Png, CompressionPreset::MaximumCompression);

        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .any(|candidate| candidate.engine.name() == "oxipng"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.engine.name() == "pngquant"));
        assert!(candidates
            .iter()
            .all(|candidate| candidate.output_format == ImageFormat::Png));
    }

    #[test]
    fn jpeg_candidates_preserve_jpeg_output() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let candidates = opt.build_candidates(ImageFormat::Jpeg, CompressionPreset::Email);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].engine.name(), "mozjpeg");
        assert!(candidates
            .iter()
            .all(|candidate| candidate.output_format == ImageFormat::Jpeg));
    }

    #[test]
    fn webp_candidates_preserve_webp_output() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let candidates = opt.build_candidates(ImageFormat::WebP, CompressionPreset::Email);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].engine.name(), "webp");
        assert_eq!(candidates[0].output_format, ImageFormat::WebP);
    }

    #[test]
    fn optimization_never_promotes_a_larger_file() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("in-tinydrop.png");
        let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(128, 128, |x, y| {
            image::Rgba([(x * 2) as u8, (y * 2) as u8, 160, 255])
        });
        image.save(&input).unwrap();

        let result = opt
            .optimize(&input, CompressionPreset::Email, &output)
            .expect("email optimization should always preserve a usable image");

        assert!(result.result.optimized_bytes <= result.result.original_bytes);
        assert!(result.result.output_path.is_file());
        assert_eq!(
            result
                .result
                .output_path
                .extension()
                .and_then(|ext| ext.to_str()),
            Some(result.result.format.extension())
        );
    }

    #[test]
    fn run_single_optimizes_with_named_engine() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.png");
        let result = opt.run_single(
            "not-a-real-engine",
            &input,
            &output,
            &EngineSettings::balanced(),
        );
        assert!(matches!(result, Err(CoreError::Engine { .. })));
    }

    #[test]
    fn plan_output_path_preserves_stem_and_directory() {
        let p = AdaptiveOptimizer::plan_output_path(
            Path::new("/Users/me/Pictures/foo.png"),
            ImageFormat::Png,
        );
        assert_eq!(p, Path::new("/Users/me/Pictures/foo-tinydrop.png"));
    }

    #[test]
    fn optimize_picks_winner_for_png() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.png");
        // Create a noisy RGBA PNG that should favor webp over oxipng.
        let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(96, 96, |x, y| {
            image::Rgba([
                (x.wrapping_mul(7)) as u8,
                (y.wrapping_mul(11)) as u8,
                (x.wrapping_add(y)) as u8,
                255,
            ])
        });
        img.save(&input).unwrap();

        let winner = opt
            .optimize(&input, CompressionPreset::Website, &output)
            .expect("optimize should succeed");
        assert!(winner.passed_quality_gate);
        assert_eq!(winner.result.format, ImageFormat::Png);
        assert_eq!(
            winner
                .result
                .output_path
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("png")
        );
        assert!(winner.result.optimized_bytes <= winner.result.original_bytes);
    }

    #[test]
    fn png_output_stays_png_for_every_preset() {
        let opt = AdaptiveOptimizer::new(default_registry());
        let dir = tempdir().unwrap();
        let input = dir.path().join("source.png");
        let output = dir.path().join("source-tinydrop.png");
        let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(64, 64, |x, y| {
            image::Rgba([(x * 3) as u8, (y * 3) as u8, 120, 255])
        });
        image.save(&input).unwrap();

        for &preset in CompressionPreset::ALL {
            let result = opt.optimize(&input, preset, &output).unwrap();
            assert_eq!(result.result.format, ImageFormat::Png, "{preset:?}");
            assert_eq!(
                result
                    .result
                    .output_path
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("png"),
                "{preset:?}"
            );
        }
    }
}
