//! Subcommand implementations.

use anyhow::{Context, Result, anyhow};

pub mod apply_shared;
pub mod backup;
pub mod bootstrap;
pub mod commit;
pub mod embed;
pub mod forget_sweep;
pub mod generate_auth_token;
pub mod init;
pub mod install_hooks;
pub mod install_instructions;
pub mod install_mcp;
pub mod lint;
pub mod llm_test;
pub mod render_shared;
pub mod reorg;
pub mod reset;
pub mod restore;
pub mod search;
pub mod serve;
pub mod setup_agent;
pub mod status;
pub mod watch;
pub mod write_page;

/// Resolve the effective project name for a client command.
///
/// Precedence:
/// 1. `explicit` (the user's `--project` flag) when non-empty.
/// 2. Basename of the git repo root walked up from CWD (handles
///    running from any subdir of the project).
/// 3. Basename of the bare CWD (covers non-git directories).
///
/// Mirrors the heuristic the hook router uses in
/// `ai-memory-hooks::router::resolve_project_ids`, so commands
/// auto-target the same project the user's interactive sessions
/// have been writing into. Dot-prefixed dirs are preserved
/// verbatim (`~/.config` → project `.config`).
pub(crate) fn resolve_project_name(explicit: Option<&str>) -> Result<String> {
    if let Some(p) = explicit.filter(|s| !s.is_empty()) {
        return Ok(p.to_string());
    }
    if let Ok(root) = ai_memory_consolidate::discover_repo_root(std::path::Path::new("."))
        && let Some(name) = root
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
    {
        return Ok(name.to_string());
    }
    let cwd = std::env::current_dir().context("getting CWD for project auto-detect")?;
    cwd.file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "could not derive project name from CWD ({}); \
                 pass --project explicitly",
                cwd.display()
            )
        })
}
