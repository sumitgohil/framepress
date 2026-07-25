# ADR-0004: Opt-in loopback MCP access for trusted local agents

**Status:** Accepted · **Date:** 2026-07-13

## Context

FramePress is useful both as a desktop application and as a local optimization capability in an agent-assisted workflow. A separate agent pipeline would duplicate image-processing behavior, fragment job visibility, and create a second set of safety controls.

MCP offers a common interface for connecting local AI clients, but exposing file-processing capabilities introduces real authority: a connected client can request work on files the user has approved. The integration must preserve FramePress's local-first and user-controlled model.

## Decision

FramePress provides an **opt-in Streamable HTTP MCP server** that is bound to loopback and submits work to the existing application queue.

The desktop user controls whether the server runs, its local port, bearer token, approved directory roots, and batch limit. The MCP server is stateless across client sessions and does not expose a remote network interface.

## Rationale

- Reusing `QueueProcessor` and `SqliteHistory` gives desktop and agent work one optimization pipeline, one history, and one set of analytics.
- Loopback binding (`127.0.0.1`) avoids a network-service deployment model and limits reach to the local machine.
- Bearer-token authentication prevents an arbitrary local process from using the endpoint without the saved connection details.
- Approved roots make the scope of an agent's file access visible and intentionally configurable.
- User-controlled policy keeps an agent from silently broadening its own authority.
- Streamable HTTP is widely supported by MCP clients and lets FramePress present a copyable connection configuration from its Settings UI.

## Consequences

- FramePress owns a small local HTTP server while Agent Access is enabled.
- Users must explicitly approve source directories before an agent can submit work.
- Agent-created batches are visible in Queue, History, and Statistics and are marked with their MCP source.
- Persisted connection details remain useful after an application restart; clients should still handle server availability and token rotation.
- The MCP API remains focused on local file optimization, job control, results, and analytics. It does not expose arbitrary filesystem operations or allow agents to modify global safety configuration.

## Alternatives considered

- **A separate agent-only optimizer** — rejected because duplicate queues and histories would create inconsistent results and poor visibility.
- **A remote or LAN-accessible API** — rejected because it conflicts with the local-first privacy model and expands the security boundary substantially.
- **stdio-only MCP** — rejected for the desktop application because it would require FramePress to manage a separate per-client child-process lifecycle rather than offering a stable local endpoint from Settings.
- **Unauthenticated loopback service** — rejected because any local process could submit work without user-controlled credentials.
