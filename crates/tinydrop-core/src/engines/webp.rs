//! `webp` engine wrapper. Encodes both lossy and lossless WebP via libwebp.
//!
//! On macOS, `webp-sys` will build libwebp from source via `cmake` if no
//! system library is found.

use std::path::Path;
use std::time::Instant;

use image::ImageReader;

use crate::domain::{CompressionResult, EngineSettings, ImageFormat};
use crate::errors::{CoreError, CoreResult};
use crate::traits::CompressionEngine;

/// WebP engine backed by the `webp` crate. Supports both lossy and lossless
/// output depending on the active preset.
#[derive(Debug, Default, Clone)]
pub struct WebPEngine;

impl WebPEngine {
    /// Construct a new engine. Cheap — no I/O.
    pub fn new() -> Self {
        Self
    }
}

impl CompressionEngine for WebPEngine {
    fn name(&self) -> &'static str {
        "webp"
    }

    fn supported_formats(&self) -> &[ImageFormat] {
        // The webp engine can *accept* PNG/JPEG inputs and produce WebP output.
        // For Phase 1 we only expose WebP↔WebP re-encoding (the format is
        // re-encode-supported in its own right). The adaptive optimizer will
        // also try WebP lossless against a PNG input (handled in Branch 2).
        // WebP is a strong size-saving option for both photographic JPEGs
        // and PNGs. The optimizer records it as a WebP output, never as a
        // mislabeled source-format file.
        &[ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP]
    }

    fn supports_lossless(&self, _format: ImageFormat) -> bool {
        true
    }

    fn optimize(
        &self,
        input: &Path,
        output: &Path,
        settings: &EngineSettings,
    ) -> CoreResult<CompressionResult> {
        if !input.is_file() {
            return Err(CoreError::InputNotFound(input.to_path_buf()));
        }

        let original_bytes = std::fs::metadata(input)
            .map_err(|e| CoreError::Io {
                path: input.to_path_buf(),
                source: e,
            })?
            .len();

        let started = Instant::now();

        let img = ImageReader::open(input)
            .map_err(|e| CoreError::Engine {
                engine: "webp".to_string(),
                message: format!("cannot open input: {e}"),
            })?
            .with_guessed_format()
            .map_err(|e| CoreError::Engine {
                engine: "webp".to_string(),
                message: format!("cannot detect input format: {e}"),
            })?
            .decode()
            .map_err(|e| CoreError::Engine {
                engine: "webp".to_string(),
                message: format!("decode failed: {e}"),
            })?;

        let memory = if settings.lossless {
            webp::Encoder::from_image(&img)
                .map_err(|e| CoreError::Engine {
                    engine: "webp".to_string(),
                    message: format!("encoder init failed: {e}"),
                })?
                .encode_lossless()
        } else {
            webp::Encoder::from_image(&img)
                .map_err(|e| CoreError::Engine {
                    engine: "webp".to_string(),
                    message: format!("encoder init failed: {e}"),
                })?
                .encode(settings.quality.clamp(1, 100) as f32)
        };

        std::fs::write(output, &*memory).map_err(|e| CoreError::Io {
            path: output.to_path_buf(),
            source: e,
        })?;

        let optimized_bytes = std::fs::metadata(output)
            .map_err(|e| CoreError::Io {
                path: output.to_path_buf(),
                source: e,
            })?
            .len();

        Ok(CompressionResult {
            engine: "webp".to_string(),
            output_path: output.to_path_buf(),
            format: ImageFormat::WebP,
            original_bytes,
            optimized_bytes,
            // The optimizer computes DSSIM externally; the engine itself
            // cannot measure perceptual distance against the original.
            dssim: None,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    #[test]
    fn engine_metadata() {
        let e = WebPEngine::new();
        assert_eq!(e.name(), "webp");
        assert!(e.supported_formats().contains(&ImageFormat::Png));
        assert!(e.supported_formats().contains(&ImageFormat::Jpeg));
        assert!(e.supported_formats().contains(&ImageFormat::WebP));
        assert!(e.supports_lossless(ImageFormat::Png));
    }

    #[test]
    fn optimize_lossless_round_trip() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let img = ImageBuffer::from_fn(32, 32, |x, y| {
            Rgba([(x * 8) as u8, (y * 8) as u8, ((x + y) * 4) as u8, 255])
        });
        img.save(&input).unwrap();

        let output = dir.path().join("out.webp");
        let mut settings = EngineSettings::balanced();
        settings.lossless = true;
        let result = WebPEngine::new()
            .optimize(&input, &output, &settings)
            .expect("optimize should succeed");

        assert!(result.is_ok());
        assert_eq!(result.engine, "webp".to_string());

        // WebP magic: "RIFF" .... "WEBP"
        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.len() >= 12);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
    }

    #[test]
    fn optimize_lossy_produces_smaller_output() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        // Use a noise pattern that compresses well as lossy WebP.
        let img = ImageBuffer::from_fn(128, 128, |x, y| {
            Rgba([
                (x.wrapping_mul(7)) as u8,
                (y.wrapping_mul(11)) as u8,
                96,
                255,
            ])
        });
        img.save(&input).unwrap();

        let output = dir.path().join("out.webp");
        let result = WebPEngine::new()
            .optimize(&input, &output, &EngineSettings::balanced())
            .expect("optimize should succeed");

        assert!(result.optimized_bytes < result.original_bytes);
    }

    #[test]
    fn optimize_returns_error_for_missing_input() {
        let dir = tempdir().unwrap();
        let result = WebPEngine::new().optimize(
            &dir.path().join("missing.webp"),
            &dir.path().join("out.webp"),
            &EngineSettings::balanced(),
        );
        assert!(matches!(result, Err(CoreError::InputNotFound(_))));
    }
}
