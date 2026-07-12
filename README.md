# TinyDrop

TinyDrop is a local macOS image optimizer built with Tauri, Rust, and Svelte.
Drop an image, choose a preset, and TinyDrop selects the smallest acceptable result.

## Features

- Local processing: images never leave your machine.
- Presets for lossless work, web delivery, email, and social media.
- PNG, JPEG, and WebP optimization with OxiPNG, MozJPEG, and libwebp.
- Adaptive candidate selection with a visual-quality check.
- Queue, history, and local optimization statistics.

Email mode can write a WebP sidecar when it is the best result. TinyDrop never replaces an image with a larger file.

## Development

Requirements:

- macOS 11 or later
- Rust 1.75 or later
- Node.js 20 or later and pnpm 9 or later
- Xcode Command Line Tools
- `cmake` and `pkg-config`

```bash
pnpm install
cargo test --workspace
pnpm --filter @tinydrop/desktop check
pnpm --filter @tinydrop/desktop tauri:dev
```

The first build compiles native image libraries and can take a few minutes.

## TODO

- Add more image formats and encoders.
- Add a command-line interface.
- Add automated release packaging.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
