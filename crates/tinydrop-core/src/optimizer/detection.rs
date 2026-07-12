//! Format detection and transparency sniffing.
//!
//! The adaptive optimizer uses these to build the candidate engine list for a
//! given input file.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::domain::ImageFormat;
use crate::errors::{CoreError, CoreResult};

/// Read the first 16 bytes of `path` and identify the format via magic
/// numbers. Falls back to the file extension when the magic-number sniff is
/// inconclusive (rare in practice).
pub fn detect_format(path: &Path) -> CoreResult<ImageFormat> {
    let mut file = File::open(path).map_err(|e| CoreError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut head = [0u8; 16];
    let n = file.read(&mut head).map_err(|e| CoreError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let head = &head[..n];

    if let Some(fmt) = ImageFormat::from_magic(head) {
        return Ok(fmt);
    }

    // Fallback to extension. If we can't determine either way, error.
    ImageFormat::from_path(path).ok_or_else(|| {
        CoreError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("(none)")
                .to_string(),
        )
    })
}

/// Whether the file at `path` has transparency (alpha channel). Only PNG
/// transparency is checked in Phase 1; WebP's alpha is detected by the webp
/// engine when it loads; JPEG never has alpha.
pub fn has_alpha(path: &Path, format: ImageFormat) -> bool {
    match format {
        ImageFormat::Png => png_has_alpha(path),
        ImageFormat::WebP => webp_has_alpha(path),
        _ => false,
    }
}

/// Inspect a PNG file's IHDR + tRNS chunks to determine if it has alpha.
///
/// We check both the color type (which signals an alpha channel) and the
/// presence of a tRNS chunk (which signals palette transparency).
fn png_has_alpha(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    // IHDR is always the first chunk after the 8-byte signature.
    // Chunk layout: 4-byte length, 4-byte type, data, 4-byte crc.
    if bytes.len() < 8 + 8 + 13 {
        return false;
    }
    let ihdr_type = &bytes[12..16];
    if ihdr_type != b"IHDR" {
        return false;
    }
    // Color type is at offset 8+8+8+1 within IHDR data (after width+height).
    let color_type = bytes.get(25).copied().unwrap_or(0);
    // Color types: 0 = gray, 2 = RGB, 3 = palette, 4 = gray+alpha, 6 = RGBA.
    if matches!(color_type, 4 | 6) {
        return true;
    }
    // Palette PNGs can use a tRNS chunk for transparency.
    png_has_trns_chunk(&bytes)
}

fn png_has_trns_chunk(bytes: &[u8]) -> bool {
    // Walk the chunk list starting after the IHDR chunk.
    let mut i = 8 + 8 + 13 + 4; // skip signature + IHDR(length+type+data+crc)
    while i + 8 <= bytes.len() {
        let length =
            u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap_or([0, 0, 0, 0])) as usize;
        let chunk_type = &bytes[i + 4..i + 8];
        if chunk_type == b"tRNS" {
            return true;
        }
        if chunk_type == b"IEND" {
            break;
        }
        i += 8 + length + 4; // length + type + data + crc
    }
    false
}

/// WebP VP8/VP8L/VP8X chunk inspection for alpha. Cheap parse of the RIFF
/// container; returns true if the VP8X header has the alpha bit set.
fn webp_has_alpha(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    if bytes.len() < 30 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }
    // VP8X chunk: 4-byte type + 4-byte flags + ...
    if &bytes[12..16] != b"VP8X" {
        // VP8 (lossy) and VP8L (lossless) don't have an alpha channel by
        // themselves; we conservatively report false here. Engines can
        // reject mismatched expectations later.
        return false;
    }
    // Flags byte is at offset 20 (after 8-byte RIFF header + 8-byte chunk
    // header). Bit 4 = alpha.
    bytes.get(20).map(|b| b & 0x10 != 0).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    #[test]
    fn detects_png_with_alpha() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("alpha.png");
        let img = ImageBuffer::<Rgba<u8>, _>::from_fn(8, 8, |_, _| Rgba([0, 0, 0, 128]));
        img.save(&path).unwrap();

        assert!(has_alpha(&path, ImageFormat::Png));
    }

    #[test]
    fn detects_rgb_png_without_alpha() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("solid.png");
        // Use RGB (no alpha channel) — ImageBuffer writes a 3-channel PNG.
        let img =
            image::ImageBuffer::<image::Rgb<u8>, _>::from_fn(8, 8, |_, _| image::Rgb([255, 0, 0]));
        img.save(&path).unwrap();

        // RGB without an alpha channel → not transparent.
        assert!(!has_alpha(&path, ImageFormat::Png));
    }

    #[test]
    fn detects_format_from_magic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.png");
        let img = ImageBuffer::<Rgba<u8>, _>::from_fn(8, 8, |x, _| Rgba([x as u8, 0, 0, 255]));
        img.save(&path).unwrap();

        assert_eq!(detect_format(&path).unwrap(), ImageFormat::Png);
    }
}
