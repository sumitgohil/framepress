# Contributing to TinyDrop

Thanks for contributing.

## Setup

```bash
pnpm install
cargo test --workspace
pnpm --filter @tinydrop/desktop check
```

Run the desktop app with:

```bash
pnpm --filter @tinydrop/desktop tauri:dev
```

## Before opening a pull request

- Keep changes focused and include tests for behavior changes.
- Run `cargo fmt --check`.
- Run `cargo test --workspace`.
- Run `pnpm --filter @tinydrop/desktop check` for frontend changes.
- Update documentation when public behavior changes.

## Style

- Keep compression and queue logic in `tinydrop-core`.
- Keep Tauri commands thin.
- Use Svelte runes for component state.
- Prefer small, readable changes over framework-heavy abstractions.

## License

By contributing, you agree that contributions are available under the project's MIT or Apache-2.0 license.
