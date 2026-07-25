//! Compression presets. Six built-ins ship in Phase 1. The enum is closed
//! (no custom presets in v1) — adding a custom preset editor is a Phase 2
//! concern.
//!
//! See `ARCHITECTURE.md` for the rationale.

use serde::{Deserialize, Serialize};

/// The six built-in compression presets. Ordered by visual quality (highest
/// to lowest) for the UI rendering. The [`PRESETS`] constant below mirrors
/// this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionPreset {
    /// No quality loss. Optimized but bit-identical (or perceptually identical)
    /// to the original. PNG/oxipng, JPEG can only approximate this; we use a
    /// mozjpeg quality of 95 + aggressive chroma subsampling.
    Lossless,
    /// Smallest file possible while staying below the strict DSSIM gate.
    /// Slowest preset — runs the highest effort levels.
    MaximumCompression,
    /// For app icons, screenshots, UI assets. Lossless, max effort.
    DeveloperAssets,
    /// For inline web use. Quality tuned for LCP/image-heavy pages.
    Website,
    /// Aggressive size reduction for email attachments (Gmail's 25 MB cap).
    Email,
    /// Tuned for Instagram/Twitter — visually punchy, small file.
    SocialMedia,
}

impl CompressionPreset {
    /// All built-in presets in display order.
    pub const ALL: &'static [CompressionPreset] = &[
        Self::Lossless,
        Self::MaximumCompression,
        Self::DeveloperAssets,
        Self::Website,
        Self::Email,
        Self::SocialMedia,
    ];

    /// User-visible label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lossless => "Lossless",
            Self::MaximumCompression => "Maximum Compression",
            Self::DeveloperAssets => "Developer Assets",
            Self::Website => "Website",
            Self::Email => "Email",
            Self::SocialMedia => "Social Media",
        }
    }

    /// One-line description shown beneath the label in the UI.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Lossless => "No quality loss. Pixel-perfect output.",
            Self::MaximumCompression => "Smallest file possible. Slow.",
            Self::DeveloperAssets => "Icons, screenshots, UI assets. Lossless.",
            Self::Website => "Tuned for fast-loading web pages.",
            Self::Email => "Stays under common attachment size limits.",
            Self::SocialMedia => "Punchy visuals for social platforms.",
        }
    }
}

impl std::fmt::Display for CompressionPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::str::FromStr for CompressionPreset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lossless" => Ok(Self::Lossless),
            "maximum_compression" | "maximum-compression" => Ok(Self::MaximumCompression),
            "developer_assets" | "developer-assets" => Ok(Self::DeveloperAssets),
            "website" => Ok(Self::Website),
            "email" => Ok(Self::Email),
            "social_media" | "social-media" => Ok(Self::SocialMedia),
            _ => Err(()),
        }
    }
}

/// Per-preset tunables: quality, lossless flag, encoder effort, DSSIM gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresetSpec {
    /// The preset these settings belong to.
    pub preset: CompressionPreset,
    /// Target quality (1–100).
    pub quality: u8,
    /// Whether the preset should produce lossless output.
    pub lossless: bool,
    /// Encoder effort (1–10).
    pub effort: u8,
    /// DSSIM gate threshold.
    pub max_dssim: f64,
}

/// Static spec table for the six built-in presets. The optimizer and
/// resolver consult this when scoring candidates.
pub const PRESETS: &[PresetSpec] = &[
    // Lossless — quality 100, no perceptual loss acceptable.
    PresetSpec {
        preset: CompressionPreset::Lossless,
        quality: 100,
        lossless: true,
        effort: 6,
        max_dssim: 0.0,
    },
    // Maximum Compression — push every dial to the limit, accept some loss.
    PresetSpec {
        preset: CompressionPreset::MaximumCompression,
        quality: 60,
        lossless: false,
        effort: 10,
        max_dssim: 0.13,
    },
    // Developer Assets — lossless PNG, max effort (icons, screenshots).
    PresetSpec {
        preset: CompressionPreset::DeveloperAssets,
        quality: 100,
        lossless: true,
        effort: 10,
        max_dssim: 0.0,
    },
    // Website — sweet spot for hero images / inline photos.
    PresetSpec {
        preset: CompressionPreset::Website,
        quality: 78,
        lossless: false,
        effort: 8,
        max_dssim: 0.06,
    },
    // Email — push quality down, file size is the constraint.
    PresetSpec {
        preset: CompressionPreset::Email,
        quality: 55,
        lossless: false,
        effort: 10,
        max_dssim: 0.12,
    },
    // Social Media — punchy, slightly higher saturation tolerance.
    PresetSpec {
        preset: CompressionPreset::SocialMedia,
        quality: 72,
        lossless: false,
        effort: 9,
        max_dssim: 0.08,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_presets_have_specs() {
        assert_eq!(PRESETS.len(), 6);
        for spec in PRESETS {
            assert!(spec.quality >= 1 && spec.quality <= 100);
            assert!(spec.effort >= 1 && spec.effort <= 10);
            assert!(spec.max_dssim >= 0.0);
        }
    }

    #[test]
    fn label_is_human_readable() {
        assert_eq!(CompressionPreset::Lossless.label(), "Lossless");
        assert_eq!(
            CompressionPreset::MaximumCompression.label(),
            "Maximum Compression"
        );
    }

    #[test]
    fn from_str_round_trips_label_style() {
        assert_eq!(
            "lossless".parse::<CompressionPreset>().unwrap(),
            CompressionPreset::Lossless
        );
        assert_eq!(
            "maximum_compression".parse::<CompressionPreset>().unwrap(),
            CompressionPreset::MaximumCompression
        );
        assert!("garbage".parse::<CompressionPreset>().is_err());
    }
}
