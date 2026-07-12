//! Engine implementations. Each engine is a thin wrapper that adapts the
//! generic `CompressionEngine` trait to a specific codec crate.
//!
//! Adding a new engine in the future requires only:
//! 1. Adding a new file in this module
//! 2. Implementing [`CompressionEngine`](crate::traits::CompressionEngine)
//! 3. Adding the engine to [`default_registry`] below
//!
//! Nothing else in the codebase needs to change.

mod mozjpeg;
mod oxipng;
mod webp;

pub use mozjpeg::MozJpegEngine;
pub use oxipng::OxipngEngine;
pub use webp::WebPEngine;

use crate::domain::ImageFormat;
use crate::traits::CompressionEngine;

/// Build the default Phase 1 engine registry: one engine per supported format.
///
/// Order matters only for diagnostics; the adaptive optimizer scores every
/// eligible engine in parallel and picks the winner.
pub fn default_registry() -> Vec<Box<dyn CompressionEngine>> {
    vec![
        Box::new(OxipngEngine::new()),
        Box::new(MozJpegEngine::new()),
        Box::new(WebPEngine::new()),
    ]
}

/// Format -> engines that can handle it. Helper for the adaptive optimizer
/// and for queue-time validation.
pub fn engines_for_format(
    registry: &[Box<dyn CompressionEngine>],
    format: ImageFormat,
) -> Vec<&dyn CompressionEngine> {
    registry
        .iter()
        .filter(|e| e.supported_formats().contains(&format))
        .map(|e| e.as_ref())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_three_engines() {
        let reg = default_registry();
        assert_eq!(reg.len(), 3);
        let names: Vec<_> = reg.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"oxipng"));
        assert!(names.contains(&"mozjpeg"));
        assert!(names.contains(&"webp"));
    }

    #[test]
    fn engines_for_format_filters_correctly() {
        let reg = default_registry();
        let png = engines_for_format(&reg, ImageFormat::Png);
        assert!(png.iter().any(|e| e.name() == "oxipng"));
        let jpg = engines_for_format(&reg, ImageFormat::Jpeg);
        assert!(jpg.iter().any(|e| e.name() == "mozjpeg"));
    }
}
