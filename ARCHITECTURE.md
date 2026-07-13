# FramePress architecture

FramePress is a local-first desktop image optimizer. Its architecture keeps the Svelte user experience, Tauri integration, MCP access, and Rust optimization engine separate, so the performance-sensitive work stays testable and reusable.

## Goals

- Keep image data on the user's machine.
- Make safe optimization the default: preserve originals and avoid marginal savings.
- Give desktop users and trusted MCP clients one consistent queue and audit trail.
- Keep application wiring explicit and the optimization core portable.

## Workspace layout

```text
apps/desktop/          SvelteKit interface and Tauri v2 desktop shell
crates/framepress-core/  domain model, presets, optimizer, queue, engines, history
crates/framepress-cli/   reserved workspace crate for future command-line use
crates/framepress-api/   reserved workspace crate for future programmatic use
docs/adr/              architectural decision records
docs/mcp.md            MCP setup, safety model, and tool reference
```

## System overview

```text
Svelte desktop UI ─┐
                   ├── Tauri commands / local MCP server ── QueueProcessor
MCP client ────────┘                                      │
                                                          ▼
                                                  AdaptiveOptimizer
                                                          │
                                                          ▼
                                        OxiPNG · MozJPEG · WebP encoder
                                                          │
                                                          ▼
                                            sibling output + SQLite history
```

The desktop UI and the MCP server are two entry points to the same local application services. This prevents separate optimization behavior, avoids parallel histories, and makes agent-submitted work visible in Queue, History, and Statistics.

## Components

| Component           | Responsibility                                                                                 |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| SvelteKit UI        | File and folder drop, preset selection, queue controls, history, statistics, and MCP settings. |
| Tauri commands      | Thin typed bridge between the UI and application services.                                     |
| `AppContext`        | Explicitly wires shared optimizer, queue, history, settings, and MCP services.                 |
| `QueueProcessor`    | Owns pending and active work, cancellation, pause/resume state, and queue snapshots.           |
| `AdaptiveOptimizer` | Resolves the preset, runs compatible encoders, evaluates candidates, and selects a result.     |
| `CompressionEngine` | Common engine contract implemented by OxiPNG, MozJPEG, and WebP paths.                         |
| `SqliteHistory`     | Persists completed work and provides local statistics and analytics.                           |
| MCP server          | Opt-in, authenticated loopback endpoint for trusted agents.                                    |

## Optimization policy

1. FramePress detects the source format and resolves the selected preset.
2. The optimizer runs compatible encoding candidates.
3. Lossy candidates are compared with the original using a luminance-weighted YCbCr visual-distance score.
4. FramePress chooses the smallest candidate that remains within the preset's visual-distance budget.
5. A lossy result must save at least 5%; otherwise the original is retained.

Supported re-encoding formats are PNG, JPEG, and WebP. GIF and SVG are recognized by the intake layer but are not re-encoded in the current implementation.

## File and privacy boundaries

- Desktop paths come only from explicit file selection or native drag-and-drop.
- Folder traversal is performed in the desktop process; symbolic links are skipped to avoid cycles.
- Outputs are sibling files with a `-framepress` suffix, so originals remain intact.
- Generated outputs are excluded from subsequent queue intake.
- All history and analytics are stored locally.

## MCP boundary

MCP is deliberately an opt-in local integration, not a remote service:

- The server listens only on `127.0.0.1`.
- Every request must present the configured bearer token.
- An agent can submit images only beneath user-approved directory roots.
- Batch size is configured by the desktop user; standard optimization preserves the source format.
- Agents cannot alter global safety settings or approve a directory themselves.
- MCP work enters the same queue and is recorded with an agent source in local history.

See [docs/mcp.md](docs/mcp.md) for the connection flow and tool reference.

## Key decisions

- [ADR-0001: Tauri v2 over Electron](docs/adr/0001-tauri-over-electron.md)
- [ADR-0002: Trait objects and manual `AppContext`](docs/adr/0002-trait-objects-over-di-framework.md)
- [ADR-0003: YCbCr visual-distance check](docs/adr/0003-dssim-over-butteraugli-for-v1.md)
- [ADR-0004: Opt-in loopback MCP access](docs/adr/0004-local-mcp-agent-access.md)
