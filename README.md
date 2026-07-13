# FramePress

**A local-first image optimizer for macOS — with a desktop workflow for people and an MCP interface for trusted AI agents.**

FramePress makes images smaller without turning optimization into guesswork. Drop files or folders into the app, select an intent-based preset, and FramePress evaluates compatible encoders to keep the smallest result that meets the preset's visual-quality budget.

Everything happens on your Mac. There is no upload step, cloud processing, account, or telemetry requirement.

> FramePress is currently macOS-first and under active development. Feedback, issues, and contributions are welcome.

## Why FramePress?

Image optimization should fit the way you work: a quick drag-and-drop task when you are at your desk, or a controlled capability an agent can call while helping with a project. FramePress provides both paths over the same local queue, so every job remains visible and measurable.

## Features

### Built for everyday image work

- **Local-first by default.** Images are processed on-device and never leave your machine.
- **Drag in files or folders.** FramePress recursively finds supported images, removes duplicates, and skips previously generated `-framepress` outputs.
- **PNG, JPEG, and WebP optimization.** It uses OxiPNG, MozJPEG, and libwebp-based encoding paths to find a strong result for each image.
- **Adaptive candidate selection.** Compatible candidates are measured against the original; FramePress retains the smallest one that clears the selected visual-distance budget.
- **No larger replacements.** Lossy results must save at least 5%; otherwise FramePress keeps the original.
- **Safe sibling outputs.** Optimized files are written beside the source using a `-framepress` suffix, preserving the original.
- **Optional WebP copies.** Create a separate WebP output for PNG and JPEG files when a WebP deliverable is useful.

### Presets with clear intent

Choose the trade-off that matches the job instead of tuning encoder flags:

| Preset              | Designed for                                             |
| ------------------- | -------------------------------------------------------- |
| Lossless            | Pixel-perfect output with optimized encoding             |
| Maximum Compression | The smallest practical file when speed is less important |
| Developer Assets    | Icons, screenshots, and UI assets                        |
| Website             | Fast-loading pages and image-heavy experiences           |
| Email               | Smaller attachments and tighter size limits              |
| Social Media        | Compact, visually punchy social assets                   |

### A workflow you can see

- **Queue controls** for pending, running, completed, failed, and cancelled work.
- **Background processing** so desktop work stays responsive.
- **Local history** of completed optimizations, including source, output, encoder, and savings.
- **Statistics and trends** for bytes saved, reductions by format and preset, biggest wins, and work submitted by people versus agents.

### MCP agent access — local and controlled

FramePress can expose an opt-in [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) endpoint for trusted local clients such as Codex, Claude Code, or Cursor. Agents can validate inputs, submit and monitor batches, retry or cancel jobs, create WebP copies, and read local history and statistics.

The MCP service is designed around deliberate user control:

- It binds to `127.0.0.1` only; it is not reachable from your network.
- It requires a bearer token, which can be rotated from Settings.
- Agents can access only folders explicitly approved in FramePress.
- Agent jobs use the same queue, history, and analytics as desktop jobs.
- The desktop user retains control of global safety settings and directory approval.

See [MCP agent access](docs/mcp.md) for setup, connection details, and the available tools.

## How it works

```text
Desktop UI or MCP client
          ↓
       Local queue
          ↓
 Adaptive optimizer
          ↓
 OxiPNG · MozJPEG · WebP
          ↓
 Smallest candidate within the quality budget
```

For the design and component boundaries, read [ARCHITECTURE.md](ARCHITECTURE.md). The trade-offs behind key decisions live in [the ADRs](docs/adr/).

## Development

### Requirements

- macOS 11 or later
- Rust 1.75 or later
- Node.js 20 or later
- pnpm 9 or later
- Xcode Command Line Tools
- `cmake` and `pkg-config`

### Run locally

```bash
pnpm install
cargo test --workspace
pnpm --filter @framepress/desktop check
pnpm --filter @framepress/desktop tauri:dev
```

The first build compiles native image libraries and may take a few minutes.

### Useful checks

```bash
cargo fmt --check
cargo test --workspace
pnpm --filter @framepress/desktop check
pnpm --filter @framepress/desktop test
pnpm format:check
```

## Project status and roadmap

FramePress's current focus is a dependable macOS desktop workflow and safe local MCP access. Contributions that improve reliability, accessibility, test coverage, documentation, supported formats, encoders, packaging, and developer ergonomics are especially useful.

Potential future directions include additional image formats and encoders, a standalone CLI, and automated release packaging. These are not commitments; please check or open an issue before investing significant implementation time.

## Contributing

We welcome bug reports, feature ideas, documentation improvements, and code contributions. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

FramePress is licensed under the [GNU General Public License v3.0 or later](LICENSE). Third-party libraries retain their own licenses.
