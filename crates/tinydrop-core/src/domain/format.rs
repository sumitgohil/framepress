//! Image format enum. Centralized here so engines, presets, and the optimizer
//! all speak the same vocabulary.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The image formats TinyDrop knows about.
///
/// Phase 1 **re-encodes** only [`ImageFormat::Png`], [`ImageFormat::Jpeg`], and
/// [`ImageFormat::WebP`]. The other variants are recognized for the drop zone
/// "supported formats" affordance but pass through untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    /// PNG (lossless, supports alpha).
    Png,
    /// JPEG (lossy, no alpha).
    Jpeg,
    /// WebP (lossy or lossless).
    WebP,
    /// GIF — pass-through in Phase 1, no re-encode.
    Gif,
    /// SVG — pass-through in Phase 1, no re-encode.
    Svg,
}

impl ImageFormat {
    /// Lowercase extension **without** the leading dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Gif => "gif",
            Self::Svg => "svg",
        }
    }

    /// Whether Phase 1 actually re-encodes this format. `false` means the file
    /// passes through unchanged (still recorded in history, no compression).
    pub const fn is_reencode_supported(self) -> bool {
        matches!(self, Self::Png | Self::Jpeg | Self::WebP)
    }

    /// Detect the format from a magic-number sniff of `bytes` (the first few
    /// bytes of the file). Returns `None` if the signature is not recognized.
    pub fn from_magic(bytes: &[u8]) -> Option<Self> {
        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some(Self::Png);
        }
        // JPEG: FF D8 FF
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }
        // WebP: "RIFF" .... "WEBP"
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Some(Self::WebP);
        }
        // GIF: "GIF87a" or "GIF89a"
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return Some(Self::Gif);
        }
        // SVG: starts with '<' (XML); check for "<svg" after whitespace
        if bytes
            .iter()
            .take(1024)
            .skip_while(|&&b| b.is_ascii_whitespace())
            .take(4)
            .copied()
            .collect::<Vec<_>>()
            .starts_with(b"<svg")
        {
            return Some(Self::Svg);
        }
        None
    }

    /// Resolve a format from a file path's extension. Falls back to `None`
    /// for unknown extensions; callers should treat that as an unsupported file.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::WebP),
            "gif" => Some(Self::Gif),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ext is already lowercase ("png", "jpeg", ...). Upper-case for display.
        let upper = match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::WebP => "WEBP",
            Self::Gif => "GIF",
            Self::Svg => "SVG",
        };
        f.write_str(upper)
    }
}

impl FromStr for ImageFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "webp" => Ok(Self::WebP),
            "gif" => Ok(Self::Gif),
            "svg" => Ok(Self::Svg),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_matches_display() {
        assert_eq!(ImageFormat::Png.to_string(), "PNG");
        assert_eq!(ImageFormat::Jpeg.to_string(), "JPEG");
        assert_eq!(ImageFormat::WebP.to_string(), "WEBP");
    }

    #[test]
    fn from_magic_recognizes_known_signatures() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::from_magic(&png), Some(ImageFormat::Png));

        let jpg = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(ImageFormat::from_magic(&jpg), Some(ImageFormat::Jpeg));

        let mut webp = *b"RIFF\x00\x00\x00\x00WEBP";
        webp[4..8].copy_from_slice(&8u32.to_le_bytes());
        assert_eq!(ImageFormat::from_magic(&webp), Some(ImageFormat::WebP));

        let gif = *b"GIF89a";
        assert_eq!(ImageFormat::from_magic(&gif), Some(ImageFormat::Gif));
    }

    #[test]
    fn from_magic_returns_none_for_garbage() {
        assert_eq!(ImageFormat::from_magic(b"hello world"), None);
    }

    #[test]
    fn from_path_recognizes_extensions() {
        assert_eq!(
            ImageFormat::from_path(Path::new("foo/bar.PNG")),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("foo/bar.Jpeg")),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(ImageFormat::from_path(Path::new("foo/bar.xyz")), None);
    }

    #[test]
    fn reencode_supported_only_for_three_formats() {
        assert!(ImageFormat::Png.is_reencode_supported());
        assert!(ImageFormat::Jpeg.is_reencode_supported());
        assert!(ImageFormat::WebP.is_reencode_supported());
        assert!(!ImageFormat::Gif.is_reencode_supported());
        assert!(!ImageFormat::Svg.is_reencode_supported());
    }
}
