#!/bin/sh
# Claude Code SessionEnd hook (final).
# We give the session-end hook a longer timeout because the server
# synthesises a wiki page from the observation log here.
SERVER="${AI_MEMORY_HOOK_URL:-http://127.0.0.1:49374}"
curl -s --max-time 2.0 \
    -X POST "$SERVER/hook?event=session-end&agent=claude-code" \
    -H "Content-Type: application/json" \
    --data-binary @- >/dev/null 2>&1 || true
exit 0
