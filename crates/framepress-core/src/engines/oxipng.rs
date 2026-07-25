//! `oxipng` engine wrapper. Pure-Rust PNG optimizer, so this engine compiles
//! and runs without any native dependencies. Lossless by design.

use std::path::Path;
use std::time::Instant;

use oxipng::Options;

use crate::domain::{CompressionResult, EngineSettings, ImageFormat};
use crate::errors::{CoreError, CoreResult};
use crate::traits::CompressionEngine;

/// PNG-only engine backed by the `oxipng` crate. Lossless only.
#[derive(Debug, Default, Clone)]
pub struct OxipngEngine;

impl OxipngEngine {
    /// Construct a new engine. Cheap — no I/O.
    pub fn new() -> Self {
        Self
    }
}

impl CompressionEngine for OxipngEngine {
    fn name(&self) -> &'static str {
        "oxipng"
    }

    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Png]
    }

    fn supports_lossless(&self, _format: ImageFormat) -> bool {
        // oxipng is lossless by definition.
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

        // Read input into memory. PNGs are typically small enough that this is
        // cheaper than re-encoding into a temp file; the optimizer's quality
        // gate also needs the original pixel data to compute DSSIM later.
        let bytes = std::fs::read(input).map_err(|e| CoreError::Io {
            path: input.to_path_buf(),
            source: e,
        })?;

        let mut options = Options::from_preset(settings.effort.min(6));
        options.force = true;
        options.optimize_alpha = true;
        options.bit_depth_reduction = true;
        options.color_type_reduction = true;
        options.palette_reduction = true;
        options.grayscale_reduction = true;
        options.interlace = Some(if settings.effort >= 8 {
            oxipng::Interlacing::Adam7
        } else {
            oxipng::Interlacing::None
        });
        options.strip = oxipng::StripChunks::Safe;

        let optimized_bytes =
            oxipng::optimize_from_memory(&bytes, &options).map_err(|e| CoreError::Engine {
                engine: "oxipng".to_string(),
                message: format!("optimization failed: {e}"),
            })?;

        std::fs::write(output, &optimized_bytes).map_err(|e| CoreError::Io {
            path: output.to_path_buf(),
            source: e,
        })?;

        let optimized_size = optimized_bytes.len() as u64;

        Ok(CompressionResult {
            engine: "oxipng".to_string(),
            output_path: output.to_path_buf(),
            format: ImageFormat::Png,
            original_bytes,
            optimized_bytes: optimized_size,
            dssim: Some(0.0), // lossless → identical pixels
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
        let e = OxipngEngine::new();
        assert_eq!(e.name(), "oxipng");
        assert_eq!(e.supported_formats(), &[ImageFormat::Png]);
        assert!(e.supports_lossless(ImageFormat::Png));
    }

    #[test]
    fn optimize_produces_smaller_or_equal_png() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");

        // Generate a noisy RGBA PNG so oxipng has something to compress.
        let img = ImageBuffer::from_fn(64, 64, |x, y| {
            Rgba([(x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8, 255])
        });
        img.save(&input).unwrap();

        let output = dir.path().join("out.png");
        let result = OxipngEngine::new()
            .optimize(&input, &output, &EngineSettings::balanced())
            .expect("optimize should succeed");

        assert!(result.is_ok());
        assert_eq!(result.engine, "oxipng".to_string());
        assert!(result.optimized_bytes <= result.original_bytes);
    }

    #[test]
    fn optimize_returns_error_for_missing_input() {
        let dir = tempdir().unwrap();
        let result = OxipngEngine::new().optimize(
            &dir.path().join("does-not-exist.png"),
            &dir.path().join("out.png"),
            &EngineSettings::balanced(),
        );
        assert!(matches!(result, Err(CoreError::InputNotFound(_))));
    }
}
