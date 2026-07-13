//! Golden-file tests using the `SampleImages/` fixtures shipped in the repo.
//!
//! These tests assert that each engine produces a valid output for a known
//! input: the output file exists, has a plausible size, and decodes as the
//! correct format. They are not byte-for-byte deterministic because the
//! engines themselves are deterministic on a given input — but they assert
//! the *contract*, which is what we want from golden tests.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use framepress_core::{CompressionEngine, EngineSettings, MozJpegEngine, OxipngEngine, WebPEngine};
use image::{GenericImageView, ImageReader};

fn samples_dir() -> Option<PathBuf> {
    // tests/ runs with CARGO_MANIFEST_DIR pointing at crates/framepress-core.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest
        .parent()? // crates/
        .parent()? // repo root
        .join("SampleImages");
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// Find the largest PNG fixture in the samples dir, used as the test input.
fn largest_png() -> Option<PathBuf> {
    let dir = samples_dir()?;
    let mut candidates: Vec<_> = fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("png") {
                let size = fs::metadata(&path).ok()?.len();
                Some((path, size))
            } else {
                None
            }
        })
        .collect();
    candidates.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    candidates.into_iter().map(|(p, _)| p).next()
}

#[test]
fn oxipng_golden_compresses_real_png() {
    let Some(input) = largest_png() else {
        eprintln!("skipping: no PNG fixtures in SampleImages/");
        return;
    };

    let original = fs::metadata(&input).unwrap().len();

    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("out.png");

    let result = OxipngEngine::new()
        .optimize(&input, &output, &EngineSettings::balanced())
        .expect("oxipng optimize should succeed on the sample image");

    assert!(result.is_ok());
    assert_eq!(result.engine, "oxipng".to_string());
    assert_eq!(result.original_bytes, original);
    // Output should never be larger than input for a real-world image.
    assert!(result.optimized_bytes <= result.original_bytes);

    // Output must still be a valid PNG with the same dimensions.
    let out_img = ImageReader::open(&output).unwrap().decode().unwrap();
    let in_img = ImageReader::open(&input).unwrap().decode().unwrap();
    assert_eq!(out_img.dimensions(), in_img.dimensions());
}

#[test]
fn webp_golden_round_trips_real_png() {
    let Some(input) = largest_png() else {
        eprintln!("skipping: no PNG fixtures in SampleImages/");
        return;
    };

    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("out.webp");

    let mut settings = EngineSettings::balanced();
    settings.lossless = true;

    let result = WebPEngine::new()
        .optimize(&input, &output, &settings)
        .expect("webp optimize should succeed on the sample image");

    assert!(result.is_ok());
    assert_eq!(result.engine, "webp".to_string());

    // WebP magic: "RIFF" .... "WEBP"
    let mut head = [0u8; 12];
    let mut f = fs::File::open(&output).unwrap();
    f.read_exact(&mut head).unwrap();
    assert_eq!(&head[0..4], b"RIFF");
    assert_eq!(&head[8..12], b"WEBP");
}

#[test]
fn mozjpeg_decodes_input_and_writes_jpeg() {
    // mozjpeg requires a decodable input. Use one of the PNG fixtures —
    // it ships with the `image` crate's PNG decoder.
    let Some(input) = largest_png() else {
        eprintln!("skipping: no PNG fixtures in SampleImages/");
        return;
    };

    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("out.jpg");

    let result = MozJpegEngine::new()
        .optimize(&input, &output, &EngineSettings::balanced())
        .expect("mozjpeg should accept a PNG input and emit JPEG");

    assert!(result.is_ok());
    assert_eq!(result.engine, "mozjpeg".to_string());
    assert_eq!(result.format, framepress_core::ImageFormat::Jpeg);

    let mut head = [0u8; 3];
    let mut f = fs::File::open(&output).unwrap();
    f.read_exact(&mut head).unwrap();
    assert_eq!(head, [0xFF, 0xD8, 0xFF]);
}
