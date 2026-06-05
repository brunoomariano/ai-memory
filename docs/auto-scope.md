# `[auto_scope]` isolation modes

`ai-memory serve` publishes a process-shared "currently active project"
pointer that MCP read tools consult when the caller omits `workspace` /
`project`. The pointer is fed by the lifecycle hooks: every `/hook`
event that resolves a `cwd` to a real project updates the pointer so
read tools answer for the project the agent is actually in, not the
server's static `--project` default.

By default that pointer is a single process-wide slot — right for one
operator running one project at a time, but it collapses parallel
sessions on shared installs: a hook firing from `~/repo-A` overwrites
the slot that a concurrent `memory_query` (with no explicit project)
in `~/repo-B` was about to read.

The `[auto_scope]` config block selects opt-in isolation modes that
key the pointer by request identity so concurrent callers stay
separated.

## Modes

| `mode`        | Key                    | When to use                                                                                              |
|---------------|------------------------|----------------------------------------------------------------------------------------------------------|
| `single`      | (none — global slot)   | **Default.** Single operator, one project at a time. Backward-compatible with every existing install.    |
| `per_session` | `session_id`           | Session-aware clients/bridges that forward the hook session id on every MCP request. |
| `per_actor`   | `(user, session_id)`, with a user-only fallback | Shared engine fielding multiple authenticated users (multi-user mode, rung 2). Isolates across operators even when the MCP client cannot forward a session id. |

Both opt-in modes still publish to the single slot in parallel, so a
caller with no actor identity (anonymous probe, legacy code path) sees
the most recent project rather than an empty pointer. That preserves
legacy behavior, but it is not per-session isolation; use explicit
`workspace` + `project` arguments when a client cannot send actor
identity and concurrent runs matter.

## Configuration

```toml
[auto_scope]
mode = "single"           # "single" (default) | "per_session" | "per_actor"
session_ttl_secs = 3600   # TTL for per-key entries (default 1 h)
max_entries = 4096        # hard cap; oldest insertions evicted first
```

Environment-variable overrides follow the standard
`AI_MEMORY_<SECTION>__<KEY>` shape:

```bash
AI_MEMORY_AUTO_SCOPE__MODE=per_actor
AI_MEMORY_AUTO_SCOPE__SESSION_TTL_SECS=7200
AI_MEMORY_AUTO_SCOPE__MAX_ENTRIES=8192
```

## Where the actor identity comes from

| Source                                             | Populates                  |
|----------------------------------------------------|----------------------------|
| Hook payload (`/hook?event=…&agent=…`)             | `session_id`, `agent`      |
| Auth middleware (rung 1 root with `root_username`) | `user` ← root_username     |
| Auth middleware (rung 2 DB user)                   | `user` ← `users.username`  |
| MCP request header `X-Memory-Actor-Session-Id`     | `session_id` for tool calls |
| MCP request header `Mcp-Session-Id`                | fallback `session_id` for tool calls |
| Anonymous / no token                               | empty actor → single slot  |

`per_session` reads from `session_id`; `per_actor` reads from both
`user` and `session_id`. In `per_actor`, a request that has `user` but
no matching `session_id` falls back to that user's latest slot before
falling back to the process-wide single slot. That keeps Alice's MCP
request from inheriting Bob's project on authenticated shared servers,
but it does not isolate two simultaneous sessions for the same user
unless the MCP client forwards a matching session id.

## Client requirements

Lifecycle hooks already include the agent-run session id in their
payloads. MCP tool calls are separate HTTP requests, and most built-in
MCP client config files can only declare static URL/auth headers. Static
configs cannot inject the current agent-run session id into every tool
call.

Use `per_session` only when your client or bridge can send the same
opaque session id from the hook payload on each MCP request as
`X-Memory-Actor-Session-Id` (preferred) or `Mcp-Session-Id`. Otherwise
`per_session` degrades to the legacy single slot, so concurrent sessions
can still overwrite each other's current-project pointer.

For built-in installs that use static MCP config, prefer:

- `single` for one operator / one active project at a time.
- `per_actor` with multi-user bearer auth when several humans share one
  server. It isolates users via the authenticated `user` fallback even
  when their clients cannot forward session ids, but same-user
  concurrent sessions still need explicit `workspace` + `project` args
  or a session-aware bridge.

## Pairing with multi-user mode

`per_actor` is most useful when the engine is in multi-user mode (see
[`docs/users.md`](users.md)) — each authenticated user has their own
`users.token_hash` row, so the auth middleware tags every request with
the right `user`. With `[auto_scope] mode = "per_actor"`, two
authenticated users running concurrent agent sessions through the same
engine no longer overwrite each other's "current project" pointer for
MCP calls; if their clients also forward session ids, concurrent
sessions by the same user are isolated too.

Single-user installs can use `per_session` alone (no `token_pepper`,
no `users` row) only when the client/bridge forwards the session id on
MCP calls. With the stock static MCP configs, use explicit
`workspace` + `project` arguments for concurrent windows.

## Memory footprint

Per-key entries are tiny: two `Uuid`-sized ids + an `Instant`. With
the default `max_entries = 4096`, the map worst-cases at ~tens of KB
even on a corporate engine fielding hundreds of concurrent sessions.
The TTL ensures stale entries (closed Claude Code windows, dropped
hook clients) age out within an hour; the cap drops the oldest
insertions first if the TTL window is somehow exceeded.
