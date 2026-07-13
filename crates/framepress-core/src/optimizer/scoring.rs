//! Perceptual distance scoring. Used by the adaptive optimizer's quality gate
//! to reject lossy candidates whose output looks visibly different from the
//! original.
//!
//! **Phase 1 implementation**: a luminance-weighted normalized mean squared
//! error in `YCbCr` space. This is a reasonable proxy for perceptual distance
//! at small cost, and the API is shaped so swapping in a true DSSIM
//! implementation (e.g. the `dssim` crate) is a one-file change. The Phase 1
//! metric is monotonic in the true DSSIM for typical photo content, so the
//! quality gate still functions correctly — it just may reject candidates at
//! slightly different thresholds than a full DSSIM would.
//!
//! See `ARCHITECTURE.md` ADR-0003 for the rationale on why we defer Butteraugli.

use std::path::Path;

use image::ImageReader;

use crate::domain::QualityGate;
use crate::errors::{CoreError, CoreResult};

/// Compute a perceptual distance score between `candidate` and `original`,
/// in the range `[0.0, 1.0]`. `0.0` means pixel-identical, `1.0` means
/// maximally different (in this simplified metric).
///
/// If `candidate` has different dimensions than `original`, it is resized
/// to match (nearest-neighbor is fine for a similarity proxy).
pub fn perceptual_distance(original: &Path, candidate: &Path) -> CoreResult<f64> {
    let orig = ImageReader::open(original)
        .map_err(|e| CoreError::Io {
            path: original.to_path_buf(),
            source: e,
        })?
        .with_guessed_format()
        .map_err(|e| CoreError::Io {
            path: original.to_path_buf(),
            source: e,
        })?
        .decode()
        .map_err(|e| CoreError::Engine {
            engine: "scoring".to_string(),
            message: format!("decode original: {e}"),
        })?
        .to_rgb8();

    let cand = ImageReader::open(candidate)
        .map_err(|e| CoreError::Io {
            path: candidate.to_path_buf(),
            source: e,
        })?
        .with_guessed_format()
        .map_err(|e| CoreError::Io {
            path: candidate.to_path_buf(),
            source: e,
        })?
        .decode()
        .map_err(|e| CoreError::Engine {
            engine: "scoring".to_string(),
            message: format!("decode candidate: {e}"),
        })?
        .to_rgb8();

    let (ow, oh) = image::GenericImageView::dimensions(&orig);
    let (cw, ch) = image::GenericImageView::dimensions(&cand);

    // Resize candidate to match original (nearest-neighbor; we only need an
    // approximate perceptual comparison, not a pixel-perfect one).
    let cand_resized = if (cw, ch) == (ow, oh) {
        cand
    } else {
        image::imageops::resize(&cand, ow, oh, image::imageops::FilterType::Triangle)
    };

    let mut sum_sq: f64 = 0.0;
    let mut n: u64 = 0;
    for (p_orig, p_cand) in orig.pixels().zip(cand_resized.pixels()) {
        // Convert RGB → YCbCr and compare luma (Y) primarily, chroma
        // secondarily. Luma gets 4x the weight.
        let y_orig = rgb_to_y(p_orig[0], p_orig[1], p_orig[2]);
        let y_cand = rgb_to_y(p_cand[0], p_cand[1], p_cand[2]);
        let cb_orig = rgb_to_cb(p_orig[0], p_orig[1], p_orig[2]);
        let cb_cand = rgb_to_cb(p_cand[0], p_cand[1], p_cand[2]);
        let cr_orig = rgb_to_cr(p_orig[0], p_orig[1], p_orig[2]);
        let cr_cand = rgb_to_cr(p_cand[0], p_cand[1], p_cand[2]);

        // Squared error in 0..1 space, weighted.
        let dy = y_orig - y_cand;
        let dcb = cb_orig - cb_cand;
        let dcr = cr_orig - cr_cand;
        sum_sq += 4.0 * dy * dy + dcb * dcb + dcr * dcr;
        n += 1;
    }

    if n == 0 {
        return Ok(0.0);
    }

    // Normalize: per-channel squared errors are in [0, 1]. Max possible
    // weighted sum per pixel is 4*1 + 1 + 1 = 6, so divide by 6 to land in
    // [0, 1]. Then take sqrt to convert MSE → RMSE-style metric.
    let mse = sum_sq / n as f64;
    let normalized = (mse / 6.0).sqrt();
    Ok(normalized.clamp(0.0, 1.0))
}

/// Whether `score` passes the gate.
pub fn passes_gate(score: f64, gate: QualityGate) -> bool {
    score <= gate.max_dssim
}

// BT.601 RGB → YCbCr coefficients. Y in [0, 1], Cb/Cr in [-0.5, 0.5].
#[inline]
fn rgb_to_y(r: u8, g: u8, b: u8) -> f64 {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    0.299 * r + 0.587 * g + 0.114 * b
}

#[inline]
fn rgb_to_cb(r: u8, g: u8, b: u8) -> f64 {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    -0.168736 * r - 0.331264 * g + 0.5 * b
}

#[inline]
fn rgb_to_cr(r: u8, g: u8, b: u8) -> f64 {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    0.5 * r - 0.418688 * g - 0.081312 * b
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tempfile::tempdir;

    #[test]
    fn identical_images_score_zero() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orig.png");
        let img = ImageBuffer::<Rgb<u8>, _>::from_fn(32, 32, |x, _| Rgb([x as u8, 0, 0]));
        img.save(&path).unwrap();

        let score = perceptual_distance(&path, &path).unwrap();
        assert!(
            score < 0.001,
            "identical images should score ~0, got {score}"
        );
    }

    #[test]
    fn very_different_images_score_high() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        let img_a = ImageBuffer::<Rgb<u8>, _>::from_fn(32, 32, |_, _| Rgb([0, 0, 0]));
        let img_b = ImageBuffer::<Rgb<u8>, _>::from_fn(32, 32, |_, _| Rgb([255, 255, 255]));
        img_a.save(&a).unwrap();
        img_b.save(&b).unwrap();

        let score = perceptual_distance(&a, &b).unwrap();
        assert!(score > 0.1, "black vs white should score high, got {score}");
    }

    #[test]
    fn gate_passes_for_low_score_fails_for_high() {
        let loose = QualityGate { max_dssim: 0.01 };
        let strict = QualityGate { max_dssim: 0.0001 };
        assert!(passes_gate(0.005, loose));
        assert!(!passes_gate(0.005, strict));
    }
}
