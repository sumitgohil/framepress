# ADR-0003: YCbCr visual-distance check

**Status:** Accepted · **Date:** 2026-07-12

## Context

The adaptive optimizer needs a visual-distance check to reject candidates whose output looks visibly different from the original. Candidate approaches include:

- **DSSIM** — a structural-similarity-derived measure.
- **Butteraugli** — Google's perceptual metric, designed for modern image codecs and more sensitive to subtle artifacts at a higher integration and runtime cost.
- **A compact luminance-weighted YCbCr metric** — a predictable score that emphasizes luminance while still accounting for chroma changes.

## Decision

We use a **luminance-weighted YCbCr normalized mean squared error** in `[0.0, 1.0]`.

## Rationale

- The metric is inexpensive enough to run for every candidate.
- It prioritizes luminance while still accounting for chroma changes.
- It keeps the first-version pipeline compact and avoids another native dependency.
- Preset-specific thresholds make the quality/size trade-off explicit and testable.

## Consequences

- `crates/framepress-core/src/optimizer/scoring.rs` exports `perceptual_distance(original, candidate) -> CoreResult<f64>`.
- Presets define the accepted distance for their use case.

## Alternatives considered

- **Butteraugli** — rejected because it adds substantial native build complexity.
- **Full DSSIM** — rejected for the current version because the encoder pipeline benefits more from a compact, predictable metric.
