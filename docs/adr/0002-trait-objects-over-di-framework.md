# ADR-0002: Trait objects + manual `AppContext` over a DI framework

**Status:** Accepted · **Date:** 2026-07-12

## Context

`tinydrop-core` is built around `CompressionEngine` and `PresetResolver` traits, both used as `dyn Trait` objects (e.g. `Vec<Box<dyn CompressionEngine>>`). The `AdaptiveOptimizer` and `QueueProcessor` need an instance of these. The Tauri command layer needs to share the same instances across command invocations.

Two schools of thought:

1. **DI framework** (e.g. `shaku`) — provides interfaces, modules, and a container builder.
2. **Manual container** — a plain `struct AppContext` with `Arc` fields, hand-wired in `AppContext::build()`.

## Decision

We use the **manual container**. No DI framework.

## Rationale

- The total number of dependencies is small (optimizer, queue, history, settings). A `shaku` module would be longer than the wiring code it replaces.
- A reviewer can grep for `AppContext::build` and see every dependency in one place — no annotations scattered across impl blocks.
- `Arc<dyn CompressionEngine>` is already a runtime abstraction; layering a DI framework on top of it is double indirection.
- A manual `AppContext` clones cleanly and keeps application wiring independent of process model.

## Consequences

- The `AdaptiveOptimizer`, `QueueProcessor`, `SqliteHistory` constructors take their dependencies as parameters (or are wired via `with_*` builder methods).
- Every Tauri command function takes `State<'_, AppContext>` and reaches into the context. This is the explicit, easy-to-grep pattern.
- Adding a new dependency means adding a field to `AppContext` and a line to `AppContext::build`. No compile-time safety net that we wired everything — but the compiler will complain loudly on the first command that needs the new dep.
- We do not use `shaku` or `injector` or any of the modern Rust DI crates.

## Alternatives considered

- **`shaku`** — would give us compile-time wiring checks. Rejected because the runtime cost is the same as our manual approach and the codebase is small enough that the boilerplate is the dominant cost.
- **Pure free functions** — would not let us share the same `AdaptiveOptimizer` between the queue processor and the one-shot optimize path. We need shared state; we want it explicit.
- **Global statics** (`OnceCell<AdaptiveOptimizer>`) — tempting but invisible dependencies and lifetime hacks. Avoided.
