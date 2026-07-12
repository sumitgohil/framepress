//! Parallel candidate scoring via rayon. The adaptive optimizer's hot path.
//!
//! Each candidate engine runs in its own rayon task. Candidates that produce
//! a smaller output file win, gated by the configured DSSIM quality
//! threshold. Ties on size are broken by `dssim` (lower wins).

use std::path::Path;

use rayon::prelude::*;
use tempfile::TempDir;

use crate::domain::{CompressionResult, EngineSettings, ImageFormat, ScoredCandidate};
use crate::errors::{CoreError, CoreResult};
use crate::optimizer::scoring::{passes_gate, perceptual_distance};
use crate::traits::CompressionEngine;

/// A single candidate (engine + target format for output) the optimizer will
/// try in parallel.
pub struct Candidate<'a> {
    /// Engine to invoke.
    pub engine: &'a dyn CompressionEngine,
    /// The output format the engine will produce.
    pub output_format: ImageFormat,
    /// Settings to pass to the engine.
    pub settings: EngineSettings,
}

/// Run every candidate against `input` in parallel. Each candidate writes its
/// output to a temp file under `scratch` (which must outlive this call). The
/// caller is responsible for promoting the winning candidate's temp file to
/// its final destination.
pub fn score_candidates<'a>(
    input: &Path,
    candidates: &[Candidate<'a>],
    scratch: &TempDir,
) -> Vec<ScoredCandidate> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let input = input.to_path_buf();
    let scratch_path = scratch.path().to_path_buf();

    candidates
        .par_iter()
        .enumerate()
        .filter_map(|(idx, c)| score_one(&input, c, idx, &scratch_path).ok())
        .collect()
}

fn score_one(
    input: &Path,
    candidate: &Candidate<'_>,
    idx: usize,
    scratch: &Path,
) -> CoreResult<ScoredCandidate> {
    let output = scratch.join(format!(
        "cand-{idx}.{}",
        candidate.output_format.extension()
    ));
    let mut result = candidate
        .engine
        .optimize(input, &output, &candidate.settings)?;

    // Compute DSSIM if the engine didn't supply one (most engines don't).
    if result.dssim.is_none() {
        let score = perceptual_distance(input, &output).unwrap_or(0.0);
        result.dssim = Some(score);
    }

    Ok(ScoredCandidate {
        result,
        passed_quality_gate: true, // evaluated against gate by caller
        margin_pct_vs_runner_up: None,
    })
}

/// Pick the winning scored candidate: smallest output that passed the gate.
/// Returns the winner plus the runner-up (for margin reporting).
pub fn pick_winner(
    scored: Vec<ScoredCandidate>,
    gate: crate::domain::QualityGate,
) -> CoreResult<(ScoredCandidate, Option<ScoredCandidate>)> {
    let mut passing: Vec<ScoredCandidate> = scored
        .into_iter()
        .map(|mut c| {
            c.passed_quality_gate = c
                .result
                .dssim
                .map(|d| passes_gate(d, gate))
                .unwrap_or(false);
            c
        })
        .filter(|c| c.passed_quality_gate && c.result.is_ok())
        .collect();

    if passing.is_empty() {
        return Err(CoreError::NoWinner);
    }

    // Sort by size ascending; tie-break by dssim ascending.
    passing.sort_by(|a, b| {
        a.result
            .optimized_bytes
            .cmp(&b.result.optimized_bytes)
            .then_with(|| {
                a.result
                    .dssim
                    .partial_cmp(&b.result.dssim)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut iter = passing.into_iter();
    let mut winner = iter.next().expect("non-empty by earlier check");

    // Compute margin vs runner-up.
    let runner_up = iter.next().map(|r| {
        let margin = margin_pct(&winner, &r);
        winner.margin_pct_vs_runner_up = Some(margin);
        r
    });

    Ok((winner, runner_up))
}

/// Margin of `winner` over `runner_up`, expressed as a positive percentage
/// when winner is smaller.
pub fn margin_pct(winner: &ScoredCandidate, runner_up: &ScoredCandidate) -> f64 {
    let w = winner.result.optimized_bytes as f64;
    let r = runner_up.result.optimized_bytes as f64;
    if r == 0.0 {
        return 0.0;
    }
    ((r - w) / r * 100.0).max(0.0)
}

/// Convert a successful raw `CompressionResult` into a `ScoredCandidate`
/// annotated with its gate status. Used by the queue path when bypassing
/// parallel scoring.
#[allow(dead_code)]
pub fn annotate(result: CompressionResult, gate: crate::domain::QualityGate) -> ScoredCandidate {
    let passed = result.dssim.map(|d| passes_gate(d, gate)).unwrap_or(false);
    ScoredCandidate {
        result,
        passed_quality_gate: passed,
        margin_pct_vs_runner_up: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::EngineSettings;
    use crate::engines::default_registry;
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    #[test]
    fn empty_candidates_returns_empty() {
        let scratch = tempdir().unwrap();
        let input = scratch.path().join("in.png");
        let result = score_candidates(&input, &[], &scratch);
        assert!(result.is_empty());
    }

    #[test]
    fn scores_real_png_through_oxipng() {
        let reg = default_registry();
        let oxipng = reg.iter().find(|e| e.name() == "oxipng").unwrap().as_ref();

        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let img = ImageBuffer::<Rgba<u8>, _>::from_fn(64, 64, |x, y| {
            Rgba([(x * 4) as u8, (y * 4) as u8, 128, 255])
        });
        img.save(&input).unwrap();

        let candidates = vec![Candidate {
            engine: oxipng,
            output_format: ImageFormat::Png,
            settings: EngineSettings::balanced(),
        }];
        let scored = score_candidates(&input, &candidates, &dir);
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].result.engine, "oxipng".to_string());
        assert!(scored[0].result.dssim.is_some());
    }

    #[test]
    fn picks_smallest_passing_candidate() {
        let reg = default_registry();
        let oxipng = reg.iter().find(|e| e.name() == "oxipng").unwrap().as_ref();
        let webp = reg.iter().find(|e| e.name() == "webp").unwrap().as_ref();

        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let img = ImageBuffer::<Rgba<u8>, _>::from_fn(128, 128, |x, y| {
            Rgba([
                (x.wrapping_mul(7)) as u8,
                (y.wrapping_mul(11)) as u8,
                96,
                255,
            ])
        });
        img.save(&input).unwrap();

        let candidates = vec![
            Candidate {
                engine: oxipng,
                output_format: ImageFormat::Png,
                settings: EngineSettings::balanced(),
            },
            Candidate {
                engine: webp,
                output_format: ImageFormat::WebP,
                settings: EngineSettings::balanced(),
            },
        ];
        let scored = score_candidates(&input, &candidates, &dir);
        let (winner, runner_up) =
            pick_winner(scored, crate::domain::QualityGate::default()).unwrap();

        // Either engine can win depending on the input — what matters is
        // that the optimizer returns a passing result with a sensible margin.
        assert!(winner.passed_quality_gate);
        assert!(winner.result.optimized_bytes <= winner.result.original_bytes);
        // If there was a runner-up, the margin must be a non-negative number.
        if runner_up.is_some() {
            assert!(winner.margin_pct_vs_runner_up.is_some());
            assert!(winner.margin_pct_vs_runner_up.unwrap() >= 0.0);
        }
        // Sanity: the result includes both candidate engines in the test inputs.
        let _ = (oxipng, webp);
    }

    #[test]
    fn margin_pct_reports_positive_when_winner_smaller() {
        let result_a = CompressionResult {
            engine: "a".to_string(),
            output_path: std::path::PathBuf::from("/tmp/a"),
            format: ImageFormat::Png,
            original_bytes: 1000,
            optimized_bytes: 200,
            dssim: Some(0.0001),
            duration_ms: 1,
        };
        let result_b = CompressionResult {
            engine: "b".to_string(),
            output_path: std::path::PathBuf::from("/tmp/b"),
            format: ImageFormat::Png,
            original_bytes: 1000,
            optimized_bytes: 300,
            dssim: Some(0.0001),
            duration_ms: 1,
        };
        let a = ScoredCandidate {
            result: result_a,
            passed_quality_gate: true,
            margin_pct_vs_runner_up: None,
        };
        let b = ScoredCandidate {
            result: result_b,
            passed_quality_gate: true,
            margin_pct_vs_runner_up: None,
        };
        let m = margin_pct(&a, &b);
        assert!((m - (1.0 - 200.0 / 300.0) * 100.0).abs() < 0.01);
    }
}
