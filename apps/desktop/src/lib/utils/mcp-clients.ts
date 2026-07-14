/**
 * MCP client configuration recipes.
 *
 * FramePress only knows how to listen for MCP requests — it does not know
 * which client is talking to it. This module maps each client onto the
 * snippet a user must paste into that client's config file. Adding a new
 * client is one entry in `MCP_CLIENTS` and nothing else; the Settings page
 * reads `MCP_CLIENTS` directly.
 *
 * Wire-format note: every JSON snippet eventually emits the same JSON-ish
 * shape (`{ url, headers: { Authorization: "Bearer …" } }`). The differences
 * between clients live in:
 *   - the top-level key (`mcp` vs `mcpServers` vs `servers` vs `mcp_servers`)
 *   - whether `type: "http"` / `type: "remote"` is required
 *   - the per-server `type` discriminator
 * TOML and YAML clients encode the bearer token as a top-level field rather
 * than an `Authorization` header.
 */

export type McpConnection = {
  url: string;
  token: string;
};

export type McpClientFormat = "json" | "toml" | "yaml";

export type McpClient = {
  /** Stable slug used as the `<option>` value. */
  id: string;
  /** Display label shown in the dropdown. */
  label: string;
  /** Hint shown in help text — usually the target config file. */
  location: string;
  /** Snippet format. Affects only the post-copy tooltip wording. */
  format: McpClientFormat;
  /** Build the snippet text for this client. */
  snippet: (connection: McpConnection) => string;
};

/** The connection payload every JSON client ends up with, pre-wrapped in headers. */
function auth_headers(token: string): { Authorization: string } {
  return { Authorization: `Bearer ${token}` };
}

export const MCP_CLIENTS: readonly McpClient[] = [
  {
    id: "opencode",
    label: "OpenCode",
    location: "OpenCode config (any key with the opencode schema)",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        {
          $schema: "https://opencode.ai/config.json",
          mcp: {
            framepress: {
              type: "remote",
              enabled: true,
              url,
              headers: auth_headers(token),
            },
          },
        },
        null,
        2,
      ),
  },
  {
    id: "claude-desktop",
    label: "Claude Desktop",
    location: "claude_desktop_config.json",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        { mcpServers: { framepress: { url, headers: auth_headers(token) } } },
        null,
        2,
      ),
  },
  {
    id: "claude-code",
    label: "Claude Code",
    location: "project-root .mcp.json or ~/.claude.json",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        {
          mcpServers: {
            framepress: { type: "http", url, headers: auth_headers(token) },
          },
        },
        null,
        2,
      ),
  },
  {
    id: "cursor",
    label: "Cursor",
    location: "~/.cursor/mcp.json",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        { mcpServers: { framepress: { url, headers: auth_headers(token) } } },
        null,
        2,
      ),
  },
  {
    id: "vscode",
    label: "VS Code (GitHub Copilot)",
    location: ".vscode/mcp.json or user-scoped mcp.json",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        {
          servers: {
            framepress: { type: "http", url, headers: auth_headers(token) },
          },
        },
        null,
        2,
      ),
  },
  {
    id: "zed",
    label: "Zed",
    location: "~/.config/zed/settings.json",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        { mcp_servers: { framepress: { url, headers: auth_headers(token) } } },
        null,
        2,
      ),
  },
  {
    id: "codex",
    label: "Codex CLI (OpenAI)",
    location: "~/.codex/config.toml under [mcp_servers.framepress]",
    format: "toml",
    snippet: ({ url, token }) =>
      `[mcp_servers.framepress]\nurl = "${url}"\nbearer_token = "${token}"\n`,
  },
  {
    id: "goose",
    label: "Goose (Block)",
    location: "~/.config/goose/config.yaml under extensions:",
    format: "yaml",
    snippet: ({ url, token }) =>
      `extensions:\n  framepress:\n    name: framepress\n    type: streamable_http\n    url: ${url}\n    bearer_token: ${token}\n`,
  },
  {
    id: "compatible",
    label: "Other mcpServers-style client",
    location: "any client that reads { mcpServers: { … } }",
    format: "json",
    snippet: ({ url, token }) =>
      JSON.stringify(
        { mcpServers: { framepress: { url, headers: auth_headers(token) } } },
        null,
        2,
      ),
  },
];

/**
 * Visual grouping used by the `<select>` element. Order matters: clients
 * appear in the order listed here.
 */
export const MCP_CLIENT_GROUPS: readonly {
  group: string;
  ids: readonly string[];
}[] = [
  { group: "First-party", ids: ["opencode"] },
  {
    group: "Standards-based JSON (mcpServers / servers / mcp_servers)",
    ids: ["claude-desktop", "claude-code", "cursor", "vscode", "zed"],
  },
  { group: "Other formats", ids: ["codex", "goose"] },
  { group: "Fallback", ids: ["compatible"] },
];

/** Default selected id — kept here so the Settings page does not need to know order. */
export const DEFAULT_MCP_CLIENT_ID = MCP_CLIENTS[0]!.id;

/** All valid client ids as a union — typed against `MCP_CLIENTS` so they stay in sync. */
export type McpClientId = (typeof MCP_CLIENTS)[number]["id"];

export function get_mcp_client(id: string): McpClient | undefined {
  return MCP_CLIENTS.find((client) => client.id === id);
}
