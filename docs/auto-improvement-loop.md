# Optional Auto-Improvement Loop Research

> Status: research plus phased implementation notes. The dry-run reviewer is
> implemented for CLI/admin/MCP use and is safe to expose by default because it
> does not mutate wiki files or pending proposals. Pending proposal storage,
> approval, session-end triggers, and auto-apply remain proposed future work.

## Executive Summary

An ai-memory equivalent of Hermes Agent's self-improvement loop is worth
building as a default-available dry-run review path, with staged review required
before any write behavior. The current wiki already
captures useful durable knowledge: decisions, gotchas, concepts, rules, notes,
and session summaries. The missing piece is not more capture. It is a careful
post-session reviewer that can identify durable lessons and propose small wiki
patches without mutating the active agent context or silently promoting weak
session residue into rules.

The safe product shape is:

1. Keep automatic observation capture and session consolidation as they are.
2. Add an optional background review pass that creates pending wiki edits.
3. Require approval by default before applying those edits.
4. Keep a separate slow maintenance pass for deduplication, stale-page review,
   and lifecycle cleanup.

Do not copy Hermes' agent-local skill system directly. ai-memory's durable unit
is the project wiki page, not a `SKILL.md` package. The analogous targets are
`gotchas/`, `decisions/`, `concepts/`, `procedures/`, `_rules/`, small
`_slots/` state pages, and pending review pages under `_pending/`.

## Hermes Findings

Hermes has two distinct learning loops.

### Post-Turn Review

The immediate loop runs after a successful turn when cadence counters trip.
Memory review is based on user turns. Skill review is based on tool-call
iterations. The observed defaults are 10 user turns for memory review and 10
tool iterations for skill review.

Important implementation properties:

| Property | Hermes behavior | Lesson for ai-memory |
|---|---|---|
| Active context | The review runs after the response is delivered. | Never compete with the user's active task. |
| Prompt mutation | Mid-session writes update disk but do not mutate the cached active prompt. | Background learning must not rewrite the current agent context. |
| Runtime inheritance | The fork inherits provider, model, auth, cached system prompt, session id, and toolset config. | Avoid model/cache drift when spawning auxiliary review work. |
| Tool restriction | The fork keeps the parent tool schema for prefix-cache parity, then enforces a runtime whitelist for memory and skill tools. | Safety should be enforced mechanically at dispatch/write time, not only by prompt text. |
| External memory plugins | The fork is created with `skip_memory=True` so it does not prefetch/sync external providers. | Do not let the review harness pollute independent memory systems. |
| Dangerous approvals | Background review installs an auto-deny approval callback. | A daemon review must never block on an interactive prompt. |
| Compression | Review compression is disabled. | The review should not race the parent session's lifecycle. |
| Visibility | Successful review actions are summarized to the user as self-improvement review output. | Autonomous memory changes need explicit provenance. |

The post-turn prompts are intentionally aggressive about capturing reusable
procedures, user corrections, and non-trivial techniques. They also include
negative filters: do not encode transient setup failures, negative claims that a
tool is broken, one-off task narratives, or failures that resolved before the
conversation ended.

### Write Approval

Hermes has an optional write-approval gate for persistent memory and skills.
The default is off, preserving existing behavior. When enabled:

| Case | Behavior |
|---|---|
| Foreground memory, interactive CLI | Prompt inline when possible. |
| Foreground memory, no prompt channel | Stage to pending storage. |
| Background memory | Stage to pending storage. |
| Skill writes | Always stage, because skill files can be large. |
| User denies inline memory write | Block, do not stage. |
| Prompt machinery fails | Stage rather than silently dropping the write. |

Validation runs before staging, so invalid writes are rejected immediately
instead of being queued for approval and failing later.

This maps strongly to ai-memory. Wiki edits are closer to Hermes skills than to
small memory entries: they can be large, durable, and project-shaping. Staging
should be the default for autonomous ai-memory learning writes.

### Curator

Hermes' slower curator is a maintenance loop for agent-created skills. It is
triggered by inactivity rather than a cron daemon. Defaults observed in code and
docs:

| Setting | Default |
|---|---:|
| `interval_hours` | 168 hours, 7 days |
| `min_idle_hours` | 2 hours |
| `stale_after_days` | 30 days |
| `archive_after_days` | 90 days |

First-run behavior is deliberately conservative. A fresh install seeds
`last_run_at` and defers the first real pass by a full interval. Users can run a
manual dry-run first.

Important curator properties:

| Property | Hermes behavior | Lesson for ai-memory |
|---|---|---|
| Managed scope | Primarily agent-created skills, tracked in `.usage.json`. | Separate user-authored pages from autonomous pages. |
| Destructive limit | Archive is the maximum automatic destructive action. No auto-delete. | Prefer supersession or soft deletion. |
| Pinned objects | Pinned skills bypass automatic transitions. | Pinned pages and invariant slots must be protected. |
| Reports | Writes machine-readable `run.json` and human `REPORT.md`. | Every maintenance run should leave an audit artifact. |
| Backups | Takes snapshots before mutating runs and supports rollback. | Wiki git commits help, but approval reports should still be explicit. |
| Dry run | Produces report-only preview. | Dry-run should be first-class. |
| Consolidation | Merges narrow skills into umbrellas with structured summary. | ai-memory should consolidate duplicate/narrow pages separately from fresh lesson capture. |

## Live ai-memory Wiki Findings

The deployed homelab wiki was sampled on 2026-06-15. At the time of sampling it
contained 1 workspace, 38 projects, and 204 latest pages.

Page distribution by path prefix:

| Prefix | Count | Assessment |
|---|---:|---|
| `sessions/` | 57 | Useful episodic history, but also the main noise source. |
| `gotchas/` | 40 | High signal; concrete failure modes and fixes. |
| `decisions/` | 37 | High signal; durable rationale. |
| `concepts/` | 36 | High signal; architecture and domain knowledge. |
| `notes/` | 17 | Mixed; includes useful facts and smoke markers. |
| `_rules/` | 8 | High signal when concise and current. |
| `bootstrap.md` | 7 | Useful seed summary. |
| `_slots/` | 2 | Useful for current state, but stale risk is real. |

Representative high-signal pages:

| Page | Why it is useful |
|---|---|
| `ai-memory/gotchas/cli-is-always-http-client.md` | Captures a durable architectural rule, why it exists, exceptions, and prior-art failure modes. |
| `ai-memory/concepts/karpathy-wiki-pattern.md` | Explains the conceptual model behind the product. |
| `.config/notes/marvin-server-nfs-drop-rootcause.md` | Concrete root cause and fix with enough detail to prevent rediscovery. |
| `nes-to-sms/gotchas/vram-budget.md` | Domain-specific constraint that will matter across future work. |
| `akitaonrails-hugo/decisions/blog-content-sourcing.md` | Short decision with rationale and implementation guidance. |
| `llm-coding-benchmark/gotchas/hallucinated-apis.md` | Detailed, verified gotcha with examples and corrections. |

Representative low-signal pages:

| Page | Why it should not be promoted |
|---|---|
| `.config/sessions/0b9f6071-...md` | Three-second no-activity session. |
| `.config/sessions/1f8ffad8-...md` | Single `echo claude-bash-ok` test with no captured output. |
| `.config/sessions/cf81e9c3-...md` | Repeated bash smoke attempts; useful as diagnostics history only. |
| `.config/sessions/914f9f80-...md` | User prompt was only `config`; no substantive work. |
| `sabadell/sessions/8feda9e6-...md` | Heuristic session-end page with one observation. |
| `ai-memory/notes/livetest-v011-release.md` | Valid release smoke marker, but not a general lesson. |

The current system is already creating the right durable page families. The
auto-improvement opportunity is therefore selective promotion and cleanup, not a
new memory substrate.

## Recommendation

Build auto-improvement as a default-available review path, but keep the default
mode read-only. Do not let it write directly to the wiki without review at
first.

The initial feature should be a stage-only learning reviewer:

1. Dry-run review is enabled by default when an LLM provider is configured;
   CLI/admin/MCP dry-runs still require that provider and can be invoked without
   enabling any session-end trigger.
2. Reads a completed session, recent pages, and relevant existing wiki pages.
3. Produces a structured proposal containing small page creates or updates.
4. Stores the proposal in a pending-review queue with evidence and diffs.
5. Applies approved proposals through `Wiki::apply_batch`, admission webhooks,
   auth capabilities, audit logging, and the single writer actor.

Auto-apply can be considered later for narrow, high-confidence updates, but the
first version should earn trust through dry-runs and pending approvals.

## Proposed Page Targets

| Target | Use for | Notes |
|---|---|---|
| `gotchas/<topic>.md` | Reproducible pitfalls, root causes, tool quirks, failed approaches with a durable fix. | Require evidence and a correction or mitigation. |
| `decisions/<topic>.md` | Choices that changed architecture, workflow, dependencies, deployment, or policy. | Include decision, rationale, consequences. |
| `concepts/<topic>.md` | Stable domain or project architecture knowledge. | Prefer synthesis over task chronology. |
| `procedures/<topic>.md` | Reusable workflows, operating procedures, and repeated multi-step patterns. | Use when the value is the sequence, not only the root cause or rationale. |
| `_rules/<topic>.md` | Explicit always/never instructions for future agents. | Should also trigger the existing lint hint to update `AGENTS.md` or `CLAUDE.md`. |
| `_slots/current-focus.md` | Mutable short-term project state. | Treat as state, not durable truth; overwrite rather than append. |
| `notes/<topic>.md` | Useful facts that do not fit the above. | Avoid using notes as a dumping ground. |
| `_pending/auto-improve/<id>.md` | Human-reviewable staged proposals and diffs. | This is proposal storage, not approved durable knowledge. |

Do not create new session pages from the auto-improvement loop. Session pages
already come from session-end consolidation.

## Negative Filters

The review prompt should explicitly reject these as durable learning:

| Filter | Why |
|---|---|
| No-activity sessions | They add retrieval noise. |
| Single-command smoke tests | Usually operational evidence, not reusable knowledge. |
| Release markers | Keep as notes if needed, but do not promote to rules/gotchas. |
| Transient missing binaries, credentials, or setup state | These become stale false constraints. |
| Broad negative tool claims | `tool X is broken` hardens into future refusals after the tool is fixed. |
| One-off task narratives | Session pages already preserve chronology. |
| Resolved transient failures | Capture the retry or fix pattern, not the temporary failure. |
| User-visible status only | Use handoff or `_slots/current-focus.md`, not durable semantic pages. |

## Safety Invariants

Any implementation should preserve these invariants:

1. Off by default.
2. Stage by default.
3. Never mutate the active session prompt or already-prepended handoff context.
4. Never run inside hook latency. Hooks remain fire-and-forget and bounded.
5. Never bypass workspace/project isolation. Use `ScopeResolver` or its explicit
   helpers for every read and write path.
6. Never bypass auth. Use `AuthLevel::authorize(Capability::...)` for all admin
   and write surfaces.
7. Never write wiki files directly from a handler or background worker. Use
   `Wiki::write_page`, `Wiki::apply_batch`, or existing destructive helpers.
8. Never auto-delete semantic pages. Use supersession, pending proposals, or the
   existing retention sweep for episodic pages.
9. Never rewrite pinned pages or invariant slots unless the proposal cites a
   direct contradiction and is explicitly approved.
10. Include source evidence for every proposed edit.
11. Attribute autonomous proposals to a distinct `auto_improve` actor so audit
    logs, admission webhooks, and review screens can distinguish machine-suggested
    changes from user/root writes.
12. Bound model cost, input size, output size, and number of proposed page
    mutations per run.
13. Write a human-readable and machine-readable report for each run.

## Existing User Upgrade Contract

Default-available auto-improvement must not surprise existing installs:

1. Dry-run review may be exposed by default, but it must not write wiki files,
   pending proposal pages, handoffs, or audit rows.
2. Session-end triggering stays off until a bounded background scheduler and
   proposal storage exist; hooks must not gain LLM latency as a side effect of
   upgrade.
3. No database migration is required for the dry-run-only phase. Future pending
   proposal storage must use additive, idempotent migrations that preserve all
   existing wiki files and session/observation rows.
4. Existing installed `CLAUDE.md`/`AGENTS.md` blocks remain valid. Operators pick
   up newer proactive retrieval guidance by running `ai-memory install-instructions`
   or asking an agent to refresh the ai-memory routing block. The marker-based
   replacement must remain idempotent.
5. Future staged/apply modes must default to reviewable staging before any
   mutation and must keep approval attribution separate from the autonomous
   `auto_improve` proposal actor.

## Configuration Sketch

The exact names can change, but the shape should be explicit and conservative:

```toml
[auto_improve]
enabled = true
mode = "dry_run"            # dry_run | stage | auto_apply, auto_apply future only
on_session_end = false       # opt in separately from manual runs
min_observations = 8
min_session_duration_secs = 120
min_confidence = 0.75         # tune from dry-runs before any auto_apply mode
max_input_tokens = 24000
max_proposals_per_run = 5
include_raw_fallback = false
proposal_actor = "auto_improve"
pending_path = "_pending/auto-improve"

[auto_improve.maintenance]
enabled = false
interval_hours = 168
min_idle_hours = 2
dry_run_first = true
```

`mode = "auto_apply"` should not ship until staged proposals have enough real
usage to calibrate prompt quality and false-positive rates. The confidence
threshold should remain configurable because real projects vary; initial values
should be chosen from dry-runs against existing deployed wikis, not from prompt
intuition alone.

## Proposal Format

The LLM output should be structured JSON, validated before anything is staged:

```json
{
  "summary": "short human summary",
  "proposals": [
    {
      "operation": "create_or_update",
      "path": "gotchas/example.md",
      "title": "Example gotcha",
      "kind": "gotcha",
      "confidence": 0.82,
      "rationale": "why this is durable",
      "evidence": [
        {"page": "sessions/abc.md", "quote": "bounded quote"}
      ],
      "body_markdown": "# Example gotcha\n\n..."
    }
  ],
  "rejected_candidates": [
    {
      "reason": "single-command smoke test",
      "evidence": "sessions/xyz.md"
    }
  ]
}
```

Validation should reject proposals with missing evidence, wrong path prefix,
oversized bodies, attempts to mutate protected pages, unsupported operations,
or confidence below the configured threshold.

## Pending Review UX

The first production UX is explicit and review-gated. The CLI/admin dry-run
path previews proposals, `--stage` stores validated pending proposals, and
`pending-writes` applies or rejects them later. The MCP tool remains dry-run
only so agents can ask for a learning review without gaining an approval/apply
surface.

| Command or route | Purpose |
|---|---|
| `ai-memory auto-improve --dry-run` | Preview proposals for a project/session. |
| `memory_auto_improve` | MCP dry-run preview for the latest completed session or a named session. |
| `ai-memory auto-improve --stage` | Store pending proposals without applying. |
| `ai-memory curator` | Rule-based, report-only maintenance review. Defaults to dry-run. |
| `ai-memory curator --stage` | Stage exactly one curator report page for pending-writes approval. |
| `ai-memory pending-writes list` | Show staged wiki changes. |
| `ai-memory pending-writes diff <id>` | Show markdown diff. |
| `ai-memory pending-writes approve <id>` | Apply through the normal wiki mutation path. |
| `ai-memory pending-writes reject <id>` | Discard proposal with audit trail. |

Pending proposals should be visible as markdown under `_pending/auto-improve/`
so humans can review them in the wiki/Obsidian workflow. SQLite can still hold
proposal state, approval status, evidence metadata, and audit rows, but the
review artifact itself should be inspectable and versioned like the rest of the
wiki.

Because this is now an MCP tool surface, the standard prompt snippets and
regression tests assert `memory_auto_improve` appears in both prompt surfaces.
Existing installed `CLAUDE.md`/`AGENTS.md` snippets update idempotently when the
operator runs `ai-memory install-instructions` or asks an agent to refresh the
ai-memory routing block.

## Maintenance Loop Shape

Keep the curator analogue separate from the post-session learning reviewer.

The maintenance loop should handle:

1. Duplicate or near-duplicate titles.
2. Narrow pages that should be merged into a broader concept/gotcha.
3. Stale `_slots/current-focus.md` state.
4. Episodic pages that the retention formula marks cold.
5. Broken cross-references and contradiction candidates already surfaced by
   `memory_lint`.

The maintenance loop starts as report-only. `ai-memory curator --stage` stages
one normal report page under `notes/curator-<date>.md`; approving it records the
report only and does not perform the recommended maintenance actions. Later it
can stage merge or supersession proposals. It should not auto-delete semantic
pages.

## Implementation Phases

### Phase 1: Dry-Run Reviewer

Status: implemented for explicit CLI/admin/MCP dry-runs.

Add a library-level reviewer that consumes one completed session plus existing
wiki context and returns validated proposals. No writes except an optional run
report. The initial shipped surface requires `--dry-run` and an explicit
`--session-id`; it refuses staging/apply until pending proposal storage exists.

The implemented reviewer is designed for existing projects with large histories:
it treats the consolidated `sessions/<id>.md` page as the primary source when it
exists, then adds a bounded deterministic sample of raw observations selected
from start/end context, user prompts, high-importance events, error/fix/decision
keywords, and evenly spaced checkpoints. Validation rejects missing evidence,
unsupported paths, low confidence, oversized bodies, duplicate existing paths or
titles, and normalizes a missing H1 by prepending the proposal title before final
validation.

Tests:

1. Empty/no-activity sessions produce no proposals.
2. Single-command smoke sessions produce no proposals.
3. A session with a durable root cause produces one gotcha proposal.
4. A session with an explicit user rule produces one `_rules/` proposal.
5. Missing evidence rejects the proposal.

### Phase 2: Pending Wiki Writes

Add durable pending proposal storage under `_pending/auto-improve/`,
list/diff/approve/reject commands, and audit rows. Approval applies through
`Wiki::apply_batch` and existing admission hooks with the `auto_improve` actor
preserved in proposal provenance.

Tests:

1. Pending proposals survive restart.
2. Approval writes files and index rows atomically.
3. Rejection never writes a wiki file.
4. Protected page proposals are rejected before staging.
5. Cross-workspace partial scopes fail closed.
6. Pending proposal markdown is created under `_pending/auto-improve/` and never
   indexed as approved durable knowledge.
7. Approval/audit metadata preserves `auto_improve` proposal attribution and the
   approving actor separately.

### Phase 3: Optional Session-End Trigger

Add a background trigger behind config. It should run after the normal session
page is written and never block the hook response.

Tests:

1. Disabled config does nothing.
2. Enabled config stages proposals after session-end.
3. Saturated or failed review leaves normal session-end behavior intact.
4. Reports include model/provider, scope, proposals, rejections, and errors.

### Phase 4: Maintenance Curator

Add a separate scheduled report that uses existing lint, access counters,
retention scoring, links, and page metadata to propose merges/supersessions.

Tests:

1. First run defers or dry-runs before mutating behavior is possible.
2. Pinned pages and invariant slots are skipped.
3. Semantic pages are never hard-deleted.
4. Proposed merges identify source and destination pages with evidence.

## Resolved Design Choices

1. Pending proposals should be a first-class wiki structure under `_pending/`,
   with SQLite retaining state and audit metadata.
2. Procedural lessons should have a `procedures/` page family instead of being
   forced into `gotchas/` or `concepts/`.
3. Autonomous proposals should be attributed to a distinct `auto_improve` actor,
   with approval attribution tracked separately.
4. The minimum confidence threshold should be configurable and calibrated with
   dry-runs on real projects before any future auto-apply mode is considered.

## Remaining Open Question

Should approval be CLI/admin-only at first, or exposed to agents through MCP?

## Current Conclusion

Hermes validates the idea, but also shows why the boundaries matter. The useful
part is not that the agent can write memory by itself. The useful part is a
bounded, observable, reviewable loop that turns repeated work into durable
knowledge while keeping active task execution isolated.

For ai-memory, the next correct step after the dry-run reviewer is pending
proposal storage under `_pending/auto-improve/` plus approval/rejection commands.
Direct autonomous wiki mutation should remain out of scope until staged proposals
prove high signal in real deployments.
