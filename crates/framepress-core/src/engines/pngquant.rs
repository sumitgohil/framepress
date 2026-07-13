//! Lossy, palette-based PNG compression powered by libimagequant.
//!
//! Unlike a WebP conversion, this engine keeps the PNG container and alpha
//! channel intact. It is enabled for lossy presets only; lossless presets use
//! `oxipng` exclusively.

use std::path::Path;
use std::time::Instant;

use image::ImageReader;
use imagequant::RGBA;

use crate::domain::{CompressionResult, EngineSettings, ImageFormat};
use crate::errors::{CoreError, CoreResult};
use crate::traits::CompressionEngine;

/// PNG palette quantizer. The engine name intentionally matches the familiar
/// pngquant technique used by ImageOptim, while using the Rust libimagequant
/// API directly.
#[derive(Debug, Default, Clone)]
pub struct PngQuantEngine;

impl PngQuantEngine {
    /// Construct a PNG palette quantizer.
    pub fn new() -> Self {
        Self
    }
}

impl CompressionEngine for PngQuantEngine {
    fn name(&self) -> &'static str {
        "pngquant"
    }

    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Png]
    }

    fn supports_lossless(&self, _format: ImageFormat) -> bool {
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
            .map_err(|source| CoreError::Io {
                path: input.to_path_buf(),
                source,
            })?
            .len();
        let started = Instant::now();

        let rgba = ImageReader::open(input)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("cannot open input: {error}"),
            })?
            .with_guessed_format()
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("cannot detect input format: {error}"),
            })?
            .decode()
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("decode failed: {error}"),
            })?
            .to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels = rgba
            .pixels()
            .map(|pixel| RGBA::new(pixel[0], pixel[1], pixel[2], pixel[3]))
            .collect::<Vec<_>>();

        let mut attributes = imagequant::new();
        let target_quality = settings.quality.clamp(1, 100);
        // Palette size makes the preset's size/quality trade-off explicit.
        // FramePress's DSSIM gate remains the final visual-quality authority.
        let max_colors = 32 + (u32::from(target_quality) * 224 / 100);
        attributes
            .set_max_colors(max_colors)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("could not set palette size: {error}"),
            })?;
        attributes
            .set_quality(0, target_quality)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("could not configure quality: {error}"),
            })?;
        // imagequant speed 1 is best compression; FramePress effort 10 is most work.
        attributes
            .set_speed((11 - settings.effort.clamp(1, 10)) as i32)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("could not set speed: {error}"),
            })?;

        let mut image = attributes
            .new_image(pixels, width as usize, height as usize, 0.0)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("could not prepare image: {error}"),
            })?;
        let mut quantized = attributes
            .quantize(&mut image)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("quantization failed: {error}"),
            })?;
        quantized
            .set_dithering_level(1.0)
            .map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("could not enable dithering: {error}"),
            })?;
        let (palette, indices) =
            quantized
                .remapped(&mut image)
                .map_err(|error| CoreError::Engine {
                    engine: self.name().to_string(),
                    message: format!("pixel remapping failed: {error}"),
                })?;

        let palette_bytes = palette
            .iter()
            .flat_map(|color| [color.r, color.g, color.b])
            .collect::<Vec<_>>();
        let mut transparency = palette.iter().map(|color| color.a).collect::<Vec<_>>();
        while transparency.last() == Some(&255) {
            transparency.pop();
        }

        let mut encoded = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut encoded, width, height);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(palette_bytes);
            if !transparency.is_empty() {
                encoder.set_trns(transparency);
            }
            encoder.set_compression(png::Compression::Best);
            encoder.set_filter(png::FilterType::Paeth);
            let mut writer = encoder.write_header().map_err(|error| CoreError::Engine {
                engine: self.name().to_string(),
                message: format!("could not write PNG header: {error}"),
            })?;
            writer
                .write_image_data(&indices)
                .map_err(|error| CoreError::Engine {
                    engine: self.name().to_string(),
                    message: format!("could not write PNG data: {error}"),
                })?;
        }

        std::fs::write(output, &encoded).map_err(|source| CoreError::Io {
            path: output.to_path_buf(),
            source,
        })?;

        Ok(CompressionResult {
            engine: self.name().to_string(),
            output_path: output.to_path_buf(),
            format: ImageFormat::Png,
            original_bytes,
            optimized_bytes: encoded.len() as u64,
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
    fn quantizes_to_a_smaller_valid_png() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.png");
        let output = dir.path().join("output.png");
        let image = ImageBuffer::<Rgba<u8>, _>::from_fn(192, 192, |x, y| {
            Rgba([(x * 5) as u8, (y * 3) as u8, (x ^ y) as u8, 255])
        });
        image.save(&input).unwrap();

        let result = PngQuantEngine::new()
            .optimize(&input, &output, &EngineSettings::balanced())
            .unwrap();

        assert!(result.optimized_bytes < result.original_bytes);
        assert_eq!(
            crate::optimizer::detect_format(&output).unwrap(),
            ImageFormat::Png
        );
    }
}
