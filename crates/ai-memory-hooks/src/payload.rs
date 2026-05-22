//! Wire envelope received on `POST /hook`.

use ai_memory_core::{AgentKind, ObservationKind};
use serde::{Deserialize, Serialize};

/// Query-string parameters on `POST /hook`.
#[derive(Debug, Clone, Deserialize)]
pub struct HookQuery {
    /// Lifecycle event identifier (kebab-case or snake_case).
    pub event: String,
    /// Agent CLI identifier (`claude-code`, `codex`, `open-code`).
    pub agent: Option<String>,
}

/// Coalesced view of an incoming hook event after light parsing of the
/// body. We keep the original raw JSON around so consumers can extract
/// agent-specific fields they care about.
#[derive(Debug, Clone, Serialize)]
pub struct HookEnvelope {
    /// Mapped lifecycle event.
    pub event: HookEvent,
    /// Agent CLI identifier.
    pub agent: AgentKind,
    /// Session identifier, if found in the body. Required for everything
    /// except the initial `SessionStart`.
    pub session_id: Option<String>,
    /// Current working directory at the time of the event.
    pub cwd: Option<String>,
    /// Optional title hint extracted from the body.
    pub title_hint: Option<String>,
    /// Optional body excerpt extracted from the agent's raw payload.
    pub body_excerpt: Option<String>,
    /// The agent's raw JSON, kept for forensics.
    pub raw: serde_json::Value,
}

/// Discriminator for the lifecycle event that triggered the hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookEvent {
    /// New session started (capture cwd + model).
    SessionStart,
    /// User submitted a prompt.
    UserPrompt,
    /// Agent is about to call a tool.
    PreToolUse,
    /// Agent finished a tool call.
    PostToolUse,
    /// Compaction event (context window pressure).
    PreCompact,
    /// Agent emitted a notification.
    Notification,
    /// Agent finished its turn (interactive `/stop` or natural end).
    Stop,
    /// Session ended (final).
    SessionEnd,
    /// Anything else.
    Other,
}

impl HookEvent {
    /// Parse a kebab- or snake-case event identifier into [`HookEvent`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "session-start" | "session_start" | "SessionStart" => Self::SessionStart,
            "user-prompt" | "user_prompt" | "UserPromptSubmit" => Self::UserPrompt,
            "pre-tool-use" | "pre_tool_use" | "PreToolUse" => Self::PreToolUse,
            "post-tool-use" | "post_tool_use" | "PostToolUse" => Self::PostToolUse,
            "pre-compact" | "pre_compact" | "PreCompact" => Self::PreCompact,
            "notification" | "Notification" => Self::Notification,
            "stop" | "Stop" => Self::Stop,
            "session-end" | "session_end" | "SessionEnd" => Self::SessionEnd,
            _ => Self::Other,
        }
    }

    /// Map to the storage-level [`ObservationKind`].
    #[must_use]
    pub const fn to_observation_kind(self) -> ObservationKind {
        match self {
            Self::SessionStart => ObservationKind::SessionStart,
            Self::UserPrompt => ObservationKind::UserPrompt,
            Self::PreToolUse => ObservationKind::PreToolUse,
            Self::PostToolUse => ObservationKind::PostToolUse,
            Self::PreCompact => ObservationKind::PreCompact,
            Self::Notification => ObservationKind::Notification,
            Self::Stop => ObservationKind::Stop,
            Self::SessionEnd => ObservationKind::SessionEnd,
            Self::Other => ObservationKind::Other,
        }
    }
}

/// Parse an agent identifier into [`AgentKind`]. Unknown values map to
/// [`AgentKind::Other`].
#[must_use]
pub fn parse_agent(s: &str) -> AgentKind {
    match s {
        "claude-code" | "claude_code" | "claude" => AgentKind::ClaudeCode,
        "codex" => AgentKind::Codex,
        "open-code" | "opencode" => AgentKind::OpenCode,
        _ => AgentKind::Other,
    }
}

impl HookEnvelope {
    /// Build an envelope from the parsed query + the body JSON. Performs
    /// best-effort extraction of `session_id` / `cwd` / a body excerpt
    /// from common shapes used by Claude Code, Codex, and OpenCode hook
    /// payloads.
    #[must_use]
    pub fn from_query_and_body(query: HookQuery, raw: serde_json::Value) -> Self {
        let event = HookEvent::parse(&query.event);
        let agent = query.agent.as_deref().map_or(AgentKind::Other, parse_agent);
        let session_id = extract_string(&raw, &["session_id", "sessionId", "session"]);
        let cwd = extract_string(&raw, &["cwd", "current_dir", "working_dir"]);
        let title_hint = best_title_hint(event, &raw);
        let body_excerpt = best_body_excerpt(event, &raw);
        Self {
            event,
            agent,
            session_id,
            cwd,
            title_hint,
            body_excerpt,
            raw,
        }
    }
}

fn extract_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = value.get(*key).and_then(serde_json::Value::as_str)
            && !s.is_empty()
        {
            return Some(s.to_string());
        }
    }
    None
}

fn best_title_hint(event: HookEvent, raw: &serde_json::Value) -> Option<String> {
    match event {
        HookEvent::SessionStart => extract_string(raw, &["model", "title"]),
        HookEvent::UserPrompt => {
            extract_string(raw, &["prompt", "message", "text"]).map(|s| truncate_for_title(&s))
        }
        HookEvent::PreToolUse | HookEvent::PostToolUse => {
            extract_string(raw, &["tool", "tool_name", "name"])
        }
        HookEvent::Notification => extract_string(raw, &["message", "text"]),
        _ => None,
    }
}

fn best_body_excerpt(event: HookEvent, raw: &serde_json::Value) -> Option<String> {
    match event {
        HookEvent::UserPrompt => extract_string(raw, &["prompt", "message", "text"]),
        HookEvent::PostToolUse => {
            let tool = extract_string(raw, &["tool", "tool_name", "name"])?;
            let result = extract_string(raw, &["tool_response", "tool_output", "output", "result"])
                .unwrap_or_else(|| "(no output captured)".into());
            Some(format!("tool: {tool}\n---\n{}", truncate_excerpt(&result)))
        }
        HookEvent::Notification => extract_string(raw, &["message", "text"]),
        _ => None,
    }
}

fn truncate_for_title(s: &str) -> String {
    const MAX: usize = 80;
    let one_line: String = s.chars().take_while(|c| *c != '\n').collect();
    if one_line.chars().count() <= MAX {
        one_line
    } else {
        let mut buf: String = one_line.chars().take(MAX - 1).collect();
        buf.push('…');
        buf
    }
}

fn truncate_excerpt(s: &str) -> String {
    const MAX: usize = 2_000;
    if s.len() <= MAX {
        s.to_string()
    } else {
        let mut buf = String::with_capacity(MAX + 1);
        buf.push_str(&s[..MAX]);
        buf.push('…');
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_events() {
        assert_eq!(HookEvent::parse("session-start"), HookEvent::SessionStart);
        assert_eq!(HookEvent::parse("PreToolUse"), HookEvent::PreToolUse);
        assert_eq!(HookEvent::parse("user_prompt"), HookEvent::UserPrompt);
        assert_eq!(HookEvent::parse("bogus"), HookEvent::Other);
    }

    #[test]
    fn maps_to_observation_kind() {
        assert_eq!(
            HookEvent::SessionEnd.to_observation_kind(),
            ObservationKind::SessionEnd
        );
    }

    #[test]
    fn envelope_extracts_session_and_cwd() {
        let q = HookQuery {
            event: "session-start".into(),
            agent: Some("claude-code".into()),
        };
        let raw = serde_json::json!({
            "session_id": "abc-123",
            "cwd": "/tmp/x",
            "model": "claude-sonnet-4-6"
        });
        let env = HookEnvelope::from_query_and_body(q, raw);
        assert_eq!(env.event, HookEvent::SessionStart);
        assert_eq!(env.session_id.as_deref(), Some("abc-123"));
        assert_eq!(env.cwd.as_deref(), Some("/tmp/x"));
        assert_eq!(env.title_hint.as_deref(), Some("claude-sonnet-4-6"));
    }
}
