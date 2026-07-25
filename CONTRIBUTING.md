# Contributing to FramePress

Thanks for helping make local image optimization more useful, trustworthy, and approachable.

## Ways to contribute

- Report reproducible bugs and unexpected optimization results.
- Suggest improvements to presets, workflows, accessibility, or documentation.
- Add focused tests, fixtures, and performance measurements.
- Improve supported-format and encoder behavior without weakening the local-first safety model.
- Help review and refine the MCP experience for trusted local agents.

For substantial features or architectural changes, please open an issue first so the approach can be discussed before implementation begins.

## Setup

```bash
pnpm install
cargo test --workspace
pnpm --filter @framepress/desktop check
```

Run the desktop app with:

```bash
pnpm --filter @framepress/desktop tauri:dev
```

The first build may take a few minutes while native image libraries compile.

## Before opening a pull request

- Keep the change focused and explain the user-visible outcome.
- Include or update tests for behavior changes.
- Run `cargo fmt --check`.
- Run `cargo test --workspace`.
- Run `pnpm --filter @framepress/desktop check` for frontend changes.
- Run `pnpm --filter @framepress/desktop test` when frontend tests apply.
- Run `pnpm format:check` after documentation or frontend formatting changes.
- Update documentation whenever public behavior, MCP tools, or safety boundaries change.

## Project conventions

- Keep image processing, queue behavior, and persistence in `framepress-core`.
- Keep Tauri commands thin; application wiring belongs in `AppContext`.
- Use Svelte runes for component state.
- Prefer small, readable changes over framework-heavy abstractions.
- Preserve the local-first promise: no implicit uploads, telemetry, remote calls, or destructive source-file behavior.
- Treat MCP as a least-privilege integration. New agent capabilities should retain explicit user control, local-only transport, authentication, and approved-root enforcement.

## Issues and pull requests

Use a clear title and include the problem, the proposed behavior, and steps to verify it. Screenshots or short recordings are especially helpful for UI changes. If your change adds or alters an architectural trade-off, add or update an ADR in `docs/adr/`.

## License

By contributing, you agree that your contributions are provided under the project's [GNU General Public License v3.0 or later](LICENSE).
