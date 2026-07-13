# ADR-0001: Tauri v2 over Electron

**Status:** Accepted · **Date:** 2026-07-12

## Context

FramePress is a local-first macOS image optimizer with a SvelteKit frontend, a Rust optimization core, and optional loopback MCP access. It needs a desktop shell that keeps the app compact, integrates naturally with Rust services, and supports a clear local-only privacy boundary. The two primary options are Tauri v2 (Rust core + system webview) and Electron (Node.js + Chromium).

## Decision

We use **Tauri v2**.

## Rationale

| Concern             | Tauri v2                            | Electron                           |
| ------------------- | ----------------------------------- | ---------------------------------- |
| Runtime model       | System webview + Rust               | Bundled Chromium + Node.js         |
| Native feel         | System webview (WKWebView on macOS) | Chromium — close but not identical |
| Backend language    | Rust, same as `framepress-core`     | Need a Rust↔Node bridge            |
| Permissions model   | Capability files (scoped, explicit) | nodeIntegration flags (legacy)     |
| macOS-specific APIs | Easy via Cargo crates               | Need NAPI or context bridge        |
| Auto-update         | Built-in plugin                     | electron-updater (separate)        |

For FramePress's profile — a macOS-first app with a Rust core, local-only processing, and an optional local MCP service — Tauri's system webview and Rust-native backend are decisive wins. Exact binary size, memory use, and startup time are build- and machine-dependent, so they are measured rather than assumed when performance targets are introduced.

## Consequences

- We depend on `tauri = "2"` and the `tauri-plugin-*` ecosystem (dialog, opener, fs).
- The SvelteKit build outputs to `apps/desktop/build/` and is loaded as static assets by the Tauri shell.
- We must keep our capability file at `apps/desktop/src-tauri/capabilities/default.json` minimal (see ARCHITECTURE.md).
- We accept that Tauri is a younger ecosystem than Electron; some "obvious" plugin may not exist and we'll write a small Rust shim instead.

## Alternatives considered

- **Electron** — rejected per the table above. Would have made the Node.js dependency story easier but at significant binary and memory cost.
- **Native macOS app (SwiftUI)** — would double the frontend effort and separate the UI from the Rust workspace.
- **Slint / egui** — considered for a Rust-native UI. Rejected in favor of the Svelte-based queue and history views.
