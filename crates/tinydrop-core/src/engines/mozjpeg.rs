//! `mozjpeg` engine wrapper. JPEG encoder targeting the MozJPEG codec, which
//! produces meaningfully smaller JPEGs than libjpeg at equivalent quality.
//!
//! Requires the `mozjpeg` C library to be available. On macOS, `mozjpeg-sys`
//! will build it from source via `cmake`; install `nasm` for fastest SIMD
//! paths.

use std::io::BufWriter;
use std::path::Path;
use std::time::Instant;

use image::ImageReader;

use crate::domain::{CompressionResult, EngineSettings, ImageFormat};
use crate::errors::{CoreError, CoreResult};
use crate::traits::CompressionEngine;

/// JPEG engine backed by the `mozjpeg` crate. Lossy only.
#[derive(Debug, Default, Clone)]
pub struct MozJpegEngine;

impl MozJpegEngine {
    /// Construct a new engine. Cheap — no I/O.
    pub fn new() -> Self {
        Self
    }
}

impl CompressionEngine for MozJpegEngine {
    fn name(&self) -> &'static str {
        "mozjpeg"
    }

    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Jpeg]
    }

    fn supports_lossless(&self, _format: ImageFormat) -> bool {
        // mozjpeg is inherently lossy for JPEG output.
        false
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

        // Decode the input into 8-bit RGB. MozJPEG takes raw RGB scanlines;
        // if the input has alpha, blend over white (the standard "JPEG with
        // alpha" treatment).
        let mut img = ImageReader::open(input)
            .map_err(|e| CoreError::Engine {
                engine: "mozjpeg".to_string(),
                message: format!("cannot open input: {e}"),
            })?
            .with_guessed_format()
            .map_err(|e| CoreError::Engine {
                engine: "mozjpeg".to_string(),
                message: format!("cannot detect input format: {e}"),
            })?
            .decode()
            .map_err(|e| CoreError::Engine {
                engine: "mozjpeg".to_string(),
                message: format!("decode failed: {e}"),
            })?;

        if img.color().has_alpha() {
            // Composite onto white. This matches browser behavior when JPEGs
            // are converted from PNG-with-alpha.
            let mut rgba = img.to_rgba8();
            for pixel in rgba.pixels_mut() {
                let a = pixel[3] as u32;
                if a < 255 {
                    let r = ((pixel[0] as u32 * a) + (255 * (255 - a))) / 255;
                    let g = ((pixel[1] as u32 * a) + (255 * (255 - a))) / 255;
                    let b = ((pixel[2] as u32 * a) + (255 * (255 - a))) / 255;
                    pixel[0] = r as u8;
                    pixel[1] = g as u8;
                    pixel[2] = b as u8;
                    pixel[3] = 255;
                }
            }
            img = image::DynamicImage::ImageRgba8(rgba);
        }

        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        let output_file = std::fs::File::create(output).map_err(|e| CoreError::Io {
            path: output.to_path_buf(),
            source: e,
        })?;
        let writer = BufWriter::new(output_file);

        // Build mozjpeg compression parameters.
        let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        comp.set_size(width as usize, height as usize);
        comp.set_quality(settings.quality.clamp(1, 100) as f32);

        // Higher effort → longer encode, smaller output.
        if settings.effort >= 8 {
            comp.set_optimize_scans(true);
        }
        comp.set_smoothing_factor(if settings.effort >= 9 { 50 } else { 0 });

        let mut started_comp = comp.start_compress(writer).map_err(|e| CoreError::Engine {
            engine: "mozjpeg".to_string(),
            message: format!("start_compress failed: {e}"),
        })?;

        // Mozjpeg takes one row of raw bytes per call. rgb.as_raw() is the
        // contiguous pixel buffer; row stride is `width * 3`.
        let raw = rgb.as_raw();
        let stride = (width as usize) * 3;
        for row in raw.chunks(stride) {
            started_comp
                .write_scanlines(row)
                .map_err(|e| CoreError::Engine {
                    engine: "mozjpeg".to_string(),
                    message: format!("write_scanlines failed: {e}"),
                })?;
        }

        started_comp.finish().map_err(|e| CoreError::Engine {
            engine: "mozjpeg".to_string(),
            message: format!("finish failed: {e}"),
        })?;

        let optimized_bytes = std::fs::metadata(output)
            .map_err(|e| CoreError::Io {
                path: output.to_path_buf(),
                source: e,
            })?
            .len();

        Ok(CompressionResult {
            engine: "mozjpeg".to_string(),
            output_path: output.to_path_buf(),
            format: ImageFormat::Jpeg,
            original_bytes,
            optimized_bytes,
            // DSSIM is computed by the optimizer, not the engine. mozjpeg
            // itself cannot measure perceptual distance.
            dssim: None,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tempfile::tempdir;

    #[test]
    fn engine_metadata() {
        let e = MozJpegEngine::new();
        assert_eq!(e.name(), "mozjpeg");
        assert_eq!(e.supported_formats(), &[ImageFormat::Jpeg]);
        assert!(!e.supports_lossless(ImageFormat::Jpeg));
    }

    #[test]
    fn optimize_produces_valid_jpeg() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("in.png");
        let img = ImageBuffer::from_fn(32, 32, |x, y| Rgb([(x * 8) as u8, (y * 8) as u8, 128]));
        img.save(&input).unwrap();

        let output = dir.path().join("out.jpg");
        let result = MozJpegEngine::new()
            .optimize(&input, &output, &EngineSettings::balanced())
            .expect("optimize should succeed");

        assert!(result.is_ok());
        assert_eq!(result.engine, "mozjpeg".to_string());
        // JPEG magic bytes.
        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn optimize_returns_error_for_missing_input() {
        let dir = tempdir().unwrap();
        let result = MozJpegEngine::new().optimize(
            &dir.path().join("missing.jpg"),
            &dir.path().join("out.jpg"),
            &EngineSettings::balanced(),
        );
        assert!(matches!(result, Err(CoreError::InputNotFound(_))));
    }
}
