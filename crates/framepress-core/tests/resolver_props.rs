//! Property tests for the preset resolver.
//!
//! The contract: every (preset, format) pair must produce settings that pass
//! `sanitized()`. No edge case should be able to leak invalid values into an
//! engine call.

use framepress_core::{BuiltinPresetResolver, CompressionPreset, ImageFormat, PresetResolver};
use proptest::prelude::*;

fn arb_preset() -> impl Strategy<Value = CompressionPreset> {
    prop_oneof![
        Just(CompressionPreset::Lossless),
        Just(CompressionPreset::MaximumCompression),
        Just(CompressionPreset::DeveloperAssets),
        Just(CompressionPreset::Website),
        Just(CompressionPreset::Email),
        Just(CompressionPreset::SocialMedia),
    ]
}

fn arb_format() -> impl Strategy<Value = ImageFormat> {
    prop_oneof![
        Just(ImageFormat::Png),
        Just(ImageFormat::Jpeg),
        Just(ImageFormat::WebP),
    ]
}

proptest! {
    #[test]
    fn resolver_always_returns_sanitized_settings(
        preset in arb_preset(),
        format in arb_format(),
    ) {
        let resolver = BuiltinPresetResolver::new();
        let settings = resolver.resolve(preset, format);
        let sanitized = settings.sanitized();
        prop_assert_eq!(settings.quality, sanitized.quality);
        prop_assert_eq!(settings.effort, sanitized.effort);
        prop_assert!(settings.quality >= 1 && settings.quality <= 100);
        prop_assert!(settings.effort >= 1 && settings.effort <= 10);
    }

    #[test]
    fn lossless_preset_is_always_lossless(format in arb_format()) {
        let resolver = BuiltinPresetResolver::new();
        let s = resolver.resolve(CompressionPreset::Lossless, format);
        prop_assert!(s.lossless);
        prop_assert_eq!(s.quality, 100);
    }

    #[test]
    fn email_preset_is_more_aggressive_than_website(
        format in arb_format(),
    ) {
        let resolver = BuiltinPresetResolver::new();
        let website = resolver.resolve(CompressionPreset::Website, format);
        let email = resolver.resolve(CompressionPreset::Email, format);
        prop_assert!(email.quality <= website.quality);
    }
}
