# ADR-0003: YCbCr visual-distance check

**Status:** Accepted · **Date:** 2026-07-12

## Context

The adaptive optimizer needs a visual-distance check to reject candidates whose output looks visibly different from the original. Options include:

- **DSSIM** — Structural Similarity Index, a common reference for image similarity.
- **Butteraugli** — Google's perceptual metric, designed for the JPEG/WebP/AVIF codec family. Catches more subtle artifacts than DSSIM at the cost of being noticeably slower and harder to integrate as a Rust crate.

A simple MSE / RMSE fallback exists but is technically a step backwards — it correlates with but does not measure perceptual distance.

## Decision

We use a **luminance-weighted YCbCr normalized mean squared error** in `[0.0, 1.0]`.

## Rationale

- The metric is inexpensive enough to run for every candidate.
- It prioritizes luminance while still accounting for chroma changes.
- It avoids another native dependency in the encoder pipeline.

## Consequences

- `crates/tinydrop-core/src/optimizer/scoring.rs` exports `perceptual_distance(original, candidate) -> CoreResult<f64>`.
- Presets define the accepted distance for their use case.

## Alternatives considered

- **Butteraugli** — rejected because it adds substantial native build complexity.
- **Full DSSIM** — rejected because the current encoder pipeline benefits more from a compact, predictable metric.
