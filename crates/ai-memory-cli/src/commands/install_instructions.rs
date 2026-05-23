//! `ai-memory install-instructions` — drop the proactive-use snippet
//! into a project's `CLAUDE.md` / `AGENTS.md` / other rules file.
//!
//! ## Why this exists
//!
//! Lifecycle hooks handle *capture* and *handoff surfacing*
//! automatically. What they can't do is make the agent *proactively
//! call* `memory_query` / `memory_recent` when it should — that
//! decision lives in the model's system prompt, fed turn-by-turn by
//! the project's CLAUDE.md / AGENTS.md.
//!
//! This subcommand drops a small, opinionated snippet into that
//! file. Idempotent via HTML-comment markers so re-running picks up
//! whatever the snippet evolves into without duplicating the block.

use anyhow::Result;

use crate::cli::InstallInstructionsArgs;
use crate::commands::apply_shared::{ApplyOutcome, apply_atomic};
use crate::config::Config;

/// Marker that opens our managed section. Don't change the wording —
/// the closing marker, the install-instructions tooling, and any
/// future "ai-memory uninstall-instructions" all key off this exact
/// string.
const MARKER_START: &str = "<!-- ai-memory:start -->";
const MARKER_END: &str = "<!-- ai-memory:end -->";

/// The canonical snippet body. Lives in code so `install-instructions`
/// always writes the current recommended copy.
const SNIPPET_BODY: &str = r#"
## Long-term memory (ai-memory)

This project uses [ai-memory](https://github.com/akitaonrails/ai-memory)
for cross-session continuity. **Lifecycle hooks already capture every
prompt + tool call automatically.** You never need to manually write
notes; the SessionStart hook auto-fetches pending handoffs and the
SessionEnd hook auto-consolidates. Just *use* the read tools.

### When to reach for each tool

The user can express any of the intents below in plain English —
match the intent to the tool. They do not need to name the tool.

| User says / situation | Tool |
|---|---|
| "have we discussed X?" / "search memory for Y" / before proposing architecture | `memory_query` |
| "what's been going on" / "show recent activity" (light) | `memory_recent` |
| "is ai-memory healthy?" / "how big is the wiki?" | `memory_status` |
| "give me the stats" / structured snapshot for the agent to consume | `memory_briefing` |
| "catch me up" / "I've been away" / "what's important right now?" / open-ended exploration | `memory_explore` |
| "where did we leave off?" / first turn of a new session | `memory_handoff_accept` |
| "save context for the next session" / wrapping up | `memory_handoff_begin` |
| "consolidate this session" / "compile what we learned" (usually automatic) | `memory_consolidate` |
| "audit the wiki" / "find contradictions" / "what rules should we add?" | `memory_lint` |
| "prune old pages" / "memory cleanup" | `memory_forget_sweep` |

`memory_explore` is the right default for the "I want to know what's
going on" use case — it returns a prose digest whose verbosity
scales automatically to how long it's been since the last activity
(< 1 h → one line; > 30 days → full catchup).

### When you write a project rule, write it here

If you're about to write a durable project rule ("always X", "never
Y", "all PRs must …"), this file (CLAUDE.md / AGENTS.md) is where
the rule belongs. ai-memory's lint pass surfaces the same hint
automatically when a `kind: rule` page lands in `_rules/`.

### Refreshing this snippet

This block is maintained by ai-memory. To re-sync it with the
latest binary's recommended copy (e.g. when new tools ship), run:

```
ai-memory install-instructions
```

It's idempotent: re-runs replace the block bracketed by
`<!-- ai-memory:start -->` / `<!-- ai-memory:end -->` markers
without disturbing the rest of the file.
"#;

/// Run the `install-instructions` subcommand.
///
/// # Errors
/// Returns an error if the target path can't be written or if the
/// existing file isn't valid UTF-8.
pub fn run(_config: &Config, args: InstallInstructionsArgs) -> Result<()> {
    let block = format!("{MARKER_START}\n{}\n{MARKER_END}\n", SNIPPET_BODY.trim());

    if args.print {
        // Print mode: show the block + the target path, no mutation.
        println!("# Would write into: {}\n", args.target.display());
        println!("{block}");
        return Ok(());
    }

    let outcome = apply_atomic(&args.target, |existing| {
        Ok(merge_instructions_block(existing, &block))
    })?;
    println!(
        "✓ {} {} ({})",
        outcome.verb(),
        args.target.display(),
        match outcome {
            ApplyOutcome::Created => "new file",
            ApplyOutcome::Updated => "backup written next to it",
            ApplyOutcome::NoOp => "already up to date",
        }
    );
    Ok(())
}

/// Idempotent merge: when the markers exist, replace everything
/// between them (inclusive) with `block`. When they don't, append
/// `block` to the end of the file with a single blank-line
/// separator. The user's other content is never touched.
fn merge_instructions_block(existing: &str, block: &str) -> String {
    if let Some(start_idx) = existing.find(MARKER_START)
        && let Some(end_idx_rel) = existing[start_idx..].find(MARKER_END)
    {
        let end_idx = start_idx + end_idx_rel + MARKER_END.len();
        // Consume a trailing newline after the end marker if present
        // so we don't accumulate blank lines on every re-run.
        let after_end = if existing.as_bytes().get(end_idx).copied() == Some(b'\n') {
            end_idx + 1
        } else {
            end_idx
        };
        let mut out = String::with_capacity(existing.len() + block.len());
        out.push_str(&existing[..start_idx]);
        out.push_str(block);
        out.push_str(&existing[after_end..]);
        return out;
    }
    // No prior block — append. If the file already ends with a
    // newline, separate with one blank line; otherwise add the
    // newline + a blank line.
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(block);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_appends_to_empty_file() {
        let out = merge_instructions_block("", "BLOCK\n");
        assert_eq!(out, "BLOCK\n");
    }

    #[test]
    fn merge_appends_when_no_markers_present() {
        let original = "# My project\n\nSome notes.\n";
        let out = merge_instructions_block(original, "BLOCK\n");
        assert!(out.starts_with("# My project"));
        assert!(out.ends_with("BLOCK\n"));
        // One blank line between user content and our block.
        assert!(out.contains("Some notes.\n\nBLOCK\n"));
    }

    /// Real-world contract: the caller passes a marker-wrapped
    /// block (that's what `run()` builds). The merge replaces the
    /// prior bracketed section in place.
    #[test]
    fn merge_replaces_existing_block() {
        let original =
            format!("# My project\n\n{MARKER_START}\nOLD\n{MARKER_END}\n\nMore notes.\n");
        let new_block = format!("{MARKER_START}\nNEW BLOCK\n{MARKER_END}\n");
        let out = merge_instructions_block(&original, &new_block);
        assert!(out.contains("# My project"));
        assert!(out.contains("NEW BLOCK"));
        // Old content gone.
        assert!(!out.contains("OLD"));
        // User content after the block is preserved.
        assert!(out.contains("More notes."));
        // No duplicate markers.
        assert_eq!(out.matches(MARKER_START).count(), 1);
        assert_eq!(out.matches(MARKER_END).count(), 1);
    }

    #[test]
    fn merge_idempotent_double_run() {
        let block = format!("{MARKER_START}\nBLOCK\n{MARKER_END}\n");
        let first = merge_instructions_block("# Title\n", &block);
        let second = merge_instructions_block(&first, &block);
        assert_eq!(first, second, "second merge must be a no-op");
    }

    /// Defensive: existing file ends without trailing newline. We
    /// should still produce well-formed output.
    #[test]
    fn merge_tolerates_missing_trailing_newline() {
        let out = merge_instructions_block("# Title", "BLOCK\n");
        assert!(out.starts_with("# Title\n"));
        assert!(out.ends_with("BLOCK\n"));
    }
}
