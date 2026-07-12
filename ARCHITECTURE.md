# TinyDrop Architecture

TinyDrop keeps the UI, application wiring, and image-processing code separate so compression logic stays testable and portable.

## Layout

```text
apps/desktop/          Tauri shell and Svelte user interface
crates/tinydrop-core/  optimizer, queue, history, presets, and engines
docs/adr/              concise records of key technical decisions
SampleImages/          image fixtures used by integration tests
```

## Runtime flow

```text
Svelte UI → Tauri command → QueueProcessor → AdaptiveOptimizer → image engine
```

The queue owns background work and exposes snapshots for the UI. CPU-heavy encoding runs outside the async command path. The optimizer runs compatible encoders, checks visual distance, and keeps the smallest result that meets the selected preset's budget.

## Components

- `AdaptiveOptimizer` resolves a preset and scores candidate outputs.
- `CompressionEngine` is the common interface for OxiPNG, MozJPEG, and WebP.
- `QueueProcessor` tracks pending, running, completed, failed, and cancelled jobs.
- `SqliteHistory` records completed work and aggregates local statistics.
- `AppContext` wires shared application services into Tauri commands.

## File handling

TinyDrop receives paths only from explicit file selection or a native drag-and-drop action. Outputs are written beside the source with a `-tinydrop` suffix. A generated sidecar cannot be queued as a new source image.

## Quality and size policy

Each preset defines encoder quality, effort, and a visual-distance budget. Lossy results must save at least 5%; otherwise TinyDrop keeps the original. This prevents marginal or negative savings.

## Decisions

- [Tauri over Electron](docs/adr/0001-tauri-over-electron.md)
- [Manual application context](docs/adr/0002-trait-objects-over-di-framework.md)
- [YCbCr visual-distance check](docs/adr/0003-dssim-over-butteraugli-for-v1.md)
