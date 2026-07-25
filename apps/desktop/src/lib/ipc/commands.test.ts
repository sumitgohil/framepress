import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import {
  cancelJob,
  mcpConfig,
  mcpStatus,
  optimizePaths,
  pauseQueue,
  queueSnapshot,
  resumeQueue,
  rotateMcpToken,
  setMcpEnabled,
  updateMcpConfig,
} from "./commands";

describe("Tauri IPC command wrappers", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  it("uses the backend queue command names and payload shapes", async () => {
    await optimizePaths({ paths: ["/tmp/a.png"], preset: "website" });
    await cancelJob("job-1");
    await pauseQueue();
    await resumeQueue();
    await queueSnapshot();

    expect(invoke.mock.calls).toEqual([
      [
        "optimize_paths",
        { args: { paths: ["/tmp/a.png"], preset: "website" } },
      ],
      ["cancel_job", { jobId: "job-1" }],
      ["pause_queue"],
      ["resume_queue"],
      ["queue_snapshot"],
    ]);
  });

  it("uses the MCP command names and preserves their payload boundaries", async () => {
    const config = {
      enabled: true,
      port: 4444,
      token: "token",
      approved_roots: ["/tmp"],
      preserve_format: true,
      max_batch_size: 10,
    };

    await mcpConfig();
    await mcpStatus();
    await setMcpEnabled(true);
    await updateMcpConfig(config);
    await rotateMcpToken();

    expect(invoke.mock.calls).toEqual([
      ["mcp_config"],
      ["mcp_status"],
      ["set_mcp_enabled", { enabled: true }],
      ["update_mcp_config", { config }],
      ["rotate_mcp_token"],
    ]);
  });
});
