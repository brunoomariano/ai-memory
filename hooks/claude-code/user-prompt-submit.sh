#!/bin/sh
# Claude Code user-prompt hook.
# Forwards the event JSON to the ai-memory server, fire-and-forget.
# Walks up from the payload's cwd for a .ai-memory.toml marker file;
# if found, appends `&workspace=X&project=Y` to the URL so the server
# routes the event to the declared workspace/project pair instead of
# bucketing by basename(cwd) under the default workspace.
# At runtime (after `install-hooks --apply`) `_lib.sh` is staged
# alongside this script. From the source tree it lives one dir up.
_lib_dir="$(dirname "$0")"
[ -f "$_lib_dir/_lib.sh" ] || _lib_dir="$_lib_dir/.."
. "$_lib_dir/_lib.sh"

SERVER="${AI_MEMORY_HOOK_URL:-http://127.0.0.1:49374}"
PAYLOAD=$(cat)
CWD=$(ai_memory_extract_cwd "$PAYLOAD")
QS=$(ai_memory_marker_qs "$CWD")

printf '%s' "$PAYLOAD" \
    | ai_memory_post_hook "$SERVER/hook?event=user-prompt&agent=claude-code${QS}" >/dev/null 2>&1 || true
exit 0
