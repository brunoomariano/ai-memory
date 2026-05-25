#!/bin/sh
# Claude Code SessionStart hook.
# 1. Forwards the event JSON to the ai-memory server (fire-and-forget).
# 2. Synchronously fetches the pending cross-agent handoff and prints
#    it to stdout — Claude Code prepends `session-start` stdout to the
#    next session, so the resuming agent sees prior context with no
#    human in the loop.
#
# Walks up from the payload's cwd for a .ai-memory.toml marker file
# and appends `&workspace=X&project=Y` to both URLs when found — so a
# session resuming under a marker-declared workspace doesn't query the
# `default` bucket and miss its own handoff.
. "$(dirname "$0")/_lib.sh"

SERVER="${AI_MEMORY_HOOK_URL:-http://127.0.0.1:49374}"
PAYLOAD=$(cat)
CWD=$(ai_memory_extract_cwd "$PAYLOAD")
QS=$(ai_memory_marker_qs "$CWD")

printf '%s' "$PAYLOAD" \
    | ai_memory_post_hook "$SERVER/hook?event=session-start&agent=claude-code${QS}" >/dev/null 2>&1 || true

ai_memory_get_handoff "$SERVER/handoff?agent=claude-code${QS}" 2>/dev/null || true
exit 0
