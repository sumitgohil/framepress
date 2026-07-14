# MCP agent access

FramePress includes an opt-in local [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server. It lets a trusted MCP client submit image-optimization work while FramePress remains the local authority for file access, processing, and history.

## What it enables

An MCP client can discover presets and safety policy, validate a directory, queue a batch, track or retry work, create WebP copies, and read local results and statistics. Agent-submitted work appears in the desktop Queue, History, and Statistics views alongside work started in the app.

## Safety model

FramePress is intentionally conservative:

- **Disabled by default.** The desktop user enables the server from **Settings → Agent Access (MCP)**.
- **Local transport only.** The service listens on `127.0.0.1`; it is not exposed to the local network or internet.
- **Bearer-token authentication.** Every MCP request requires the token shown in Settings. Rotate it whenever a client should lose access.
- **Approved roots.** An agent can read and submit files only from folders explicitly approved in Settings.
- **User-controlled policy.** The desktop user controls approved roots, port, and batch limit. Agents cannot override those global protections.
- **Shared audit trail.** Agent jobs use FramePress's normal queue and are recorded locally with their agent source.

MCP access gives a client the ability to ask FramePress to process files within approved directories. Only connect clients you trust, and keep the bearer token private.

## Connect a client

1. Open FramePress and go to **Settings → Agent Access (MCP)**.
2. Add the project or asset folders an agent may use under **Approved folders**.
3. Turn on **Agent Access (MCP)**.
4. Choose **OpenCode** in the client selector, then select **Copy OpenCode configuration**.
5. Add the copied entry to your OpenCode configuration and restart OpenCode.

On macOS, closing the FramePress window keeps the app available in the menu bar. Use its **Start MCP** / **Stop MCP** item to control the local service, or **Exit FramePress** to end the app.

For OpenCode, the copied configuration has this shape. The `type: "remote"` field is required for OpenCode to register FramePress as MCP tools rather than treating its URL as an ordinary endpoint.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "framepress": {
      "type": "remote",
      "enabled": true,
      "url": "http://127.0.0.1:39421/mcp",
      "headers": {
        "Authorization": "Bearer <your-token>"
      }
    }
  }
}
```

The **Other compatible client** option copies the common `mcpServers` structure for clients that support it. The exact port and token are generated and shown by your FramePress installation. Do not copy a token from documentation; use the one in Settings.

## Typical agent workflow

```text
Discover capabilities → validate approved inputs → submit optimization
        → inspect job status → retrieve result or statistics
```

Before queuing a folder, an agent should call `validate_inputs`. After `submit_optimization`, it should use the returned batch ID with `get_job_status`. The agent receives output paths; revealing files in Finder remains a desktop-UI action.

## Available tools

| Tool                              | Purpose                                                                           |
| --------------------------------- | --------------------------------------------------------------------------------- |
| `get_agent_capabilities`          | Discover supported behavior, limits, and active policy.                           |
| `get_presets`                     | List the built-in optimization presets.                                           |
| `validate_inputs`                 | Check local files or folders before queueing them.                                |
| `submit_optimization`             | Queue an approved batch and return a job ID.                                      |
| `get_job_status` / `list_jobs`    | Inspect the status of a batch or recent MCP jobs.                                 |
| `cancel_job` / `retry_job`        | Stop active work or retry failed/cancelled files.                                 |
| `create_webp_copy`                | Create a separate WebP copy for PNG or JPEG inputs.                               |
| `get_file_result` / `get_history` | Read completed optimization details and local history.                            |
| `get_statistics`                  | Read 7-day, 30-day, or all-time local analytics.                                  |
| `get_access_policy`               | Read the configured approved roots and limits.                                    |
| `request_directory_access`        | Ask the desktop user to approve a new root; it does not grant access itself.      |
| `set_default_options`             | Submit future per-agent defaults without changing desktop safety settings.        |
| `reveal_output`                   | Returns a safe instruction because revealing a file is handled in the desktop UI. |

## Troubleshooting

- **“MCP server is not running”** — turn on Agent Access in FramePress Settings and copy a fresh configuration.
- **“No approved folders” or an access error** — add the parent project or asset folder in Approved folders, then retry validation.
- **Authentication failure** — refresh the copied configuration or rotate the token and update the client. A `401` response without an `Authorization` header is expected.
- **OpenCode uses shell commands instead of FramePress tools** — recopy the **OpenCode** configuration and confirm it is nested under `mcp.framepress` with `type` set to `remote`, then restart OpenCode.
- **A port cannot be bound** — choose a different local port in Settings, then restart Agent Access.
- **A file is not accepted** — FramePress currently re-encodes PNG, JPEG, and WebP. GIF and SVG are recognized at intake but are not re-encoded.
