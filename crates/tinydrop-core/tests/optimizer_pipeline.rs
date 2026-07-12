//! End-to-end optimizer pipeline test. Runs the full adaptive path against
//! every fixture in `SampleImages/` and asserts each one reaches a Completed
//! state (or an explicit Degraded state if the quality gate rejected every
//! candidate — which should not happen for the real-world fixtures shipped
//! with the repo).

use std::fs;
use std::path::PathBuf;

use tinydrop_core::{default_registry, AdaptiveOptimizer, CompressionPreset, ScoredCandidate};

fn samples_dir() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.parent()?.parent()?.join("SampleImages");
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// TinyDrop writes optimized images beside their inputs with this suffix.
/// Those runtime outputs are not source fixtures and must not be fed back
/// into the end-to-end fixture suite.
fn is_source_png(path: &std::path::Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("png")
        && !path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|stem| stem.ends_with("-tinydrop"))
}

#[test]
fn full_pipeline_completes_every_real_fixture() {
    let Some(samples) = samples_dir() else {
        eprintln!("skipping: SampleImages/ not present");
        return;
    };

    let optimizer = AdaptiveOptimizer::new(default_registry());
    let tmp = tempfile::tempdir().unwrap();

    let mut pngs: Vec<PathBuf> = fs::read_dir(&samples)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if is_source_png(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    pngs.sort();

    assert!(!pngs.is_empty(), "no PNG fixtures found");

    for input in pngs {
        let output = tmp
            .path()
            .join(input.file_name().expect("fixture has filename"));

        let result: ScoredCandidate = optimizer
            .optimize(&input, CompressionPreset::Website, &output)
            .unwrap_or_else(|e| panic!("optimize failed for {:?}: {e}", input));

        assert!(
            result.passed_quality_gate,
            "fixture {:?} failed quality gate",
            input
        );
        assert!(
            result.result.optimized_bytes > 0,
            "fixture {:?} produced zero-byte output",
            input
        );
        assert!(
            result.result.output_path.is_file(),
            "fixture {:?} did not produce output file at {:?}",
            input,
            result.result.output_path
        );
    }
}

#[test]
fn lossless_preset_always_passes_quality_gate() {
    let Some(samples) = samples_dir() else {
        return;
    };
    let optimizer = AdaptiveOptimizer::new(default_registry());
    let tmp = tempfile::tempdir().unwrap();

    for entry in fs::read_dir(&samples).unwrap().flatten() {
        let path = entry.path();
        if !is_source_png(&path) {
            continue;
        }
        let output = tmp
            .path()
            .join(path.file_name().expect("fixture has filename"));
        let result = optimizer
            .optimize(&path, CompressionPreset::Lossless, &output)
            .expect("lossless preset should always succeed");
        assert!(result.passed_quality_gate);
        assert!(result.result.output_path.is_file());
    }
}

/// The Email preset is TinyDrop's explicitly lossy delivery mode. Guard its
/// real-world effectiveness with the checked-in image corpus: each original
/// PNG must be reduced by at least 88%, while the optimizer still decides the
/// best codec and validates the visual-quality budget.
#[test]
fn email_preset_reduces_each_sample_by_at_least_eighty_eight_percent() {
    let Some(samples) = samples_dir() else {
        return;
    };
    let optimizer = AdaptiveOptimizer::new(default_registry());
    let tmp = tempfile::tempdir().unwrap();

    for entry in fs::read_dir(&samples).unwrap().flatten() {
        let input = entry.path();
        if !is_source_png(&input) {
            continue;
        }
        let output = tmp
            .path()
            .join(input.file_name().expect("fixture has filename"));
        let result = optimizer
            .optimize(&input, CompressionPreset::Email, &output)
            .unwrap_or_else(|error| panic!("email optimization failed for {input:?}: {error}"));
        let ratio = result.result.optimized_bytes as f64 / result.result.original_bytes as f64;
        println!(
            "{}: {} -> {} bytes ({:.1}% retained, {})",
            input.display(),
            result.result.original_bytes,
            result.result.optimized_bytes,
            ratio * 100.0,
            result.result.engine,
        );
        assert!(
            ratio <= 0.12,
            "Email preset did not meet the 88% reduction budget for {}",
            input.display()
        );
    }
}
