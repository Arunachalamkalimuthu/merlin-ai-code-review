//! Agentic framework — autonomous multi-step AI agent with tool use.
//!
//! Architecture (ZeroClaw-inspired, Merlin-native):
//!   Channel → AgentRuntime → [ReAct loop] → Tools → Platform/Integrations
//!
//! Features:
//!   - Swappable channels: CLI REPL, Slack bot, Discord bot
//!   - Swappable AI providers: all Merlin providers (Anthropic, OpenAI, Gemini, ...)
//!   - Built-in tool registry: all Merlin slash commands + platform actions
//!   - Conversation memory: in-memory (short-term) + JSONL file (long-term)
//!   - ReAct loop: Reason → Act (tool call) → Observe (result) → Repeat
//!
//! Usage:
//!   merlin agent                       — interactive CLI REPL
//!   merlin agent --channel slack       — Slack bot mode
//!   merlin agent --channel discord     — Discord bot mode
//!   merlin agent run "Review PR #42 and link any Jira tickets"

pub mod channels;
pub mod memory;
pub mod runtime;
pub mod tools;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ai::AiProvider;
use crate::config::Config;
use crate::platform::PlatformClient;

// ── Core message types ─────────────────────────────────────────────────────────

/// Role of a message in the conversation history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in the agent's conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: MessageRole,
    pub content: String,
    /// Tool name — only set when role is `Tool`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl AgentMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: MessageRole::User, content: content.into(), tool_name: None }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: MessageRole::Assistant, content: content.into(), tool_name: None }
    }

    pub fn tool_result(tool_name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: output.into(),
            tool_name: Some(tool_name.into()),
        }
    }
}

// ── Tool schema ────────────────────────────────────────────────────────────────

/// A single parameter in an agent tool's schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

/// Definition of a tool exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl AgentToolDef {
    /// Render this tool definition as a prompt-friendly string.
    pub fn to_prompt_line(&self) -> String {
        if self.parameters.is_empty() {
            return format!("- `{}`: {}", self.name, self.description);
        }
        let params: Vec<String> = self
            .parameters
            .iter()
            .map(|p| {
                let req = if p.required { " (required)" } else { " (optional)" };
                format!("  - `{}` [{}]{}: {}", p.name, p.param_type, req, p.description)
            })
            .collect();
        format!("- `{}`: {}\n{}", self.name, self.description, params.join("\n"))
    }
}

// ── Tool call ──────────────────────────────────────────────────────────────────

/// A tool invocation parsed from the LLM's output.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub name: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

/// Result of executing an agent tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}

// ── Context ────────────────────────────────────────────────────────────────────

/// Runtime context available to all agent tools during execution.
#[derive(Clone)]
pub struct AgentContext {
    pub ai: Arc<dyn AiProvider>,
    pub platform: Option<Arc<dyn PlatformClient>>,
    pub config: Config,
}

// ── Core traits ────────────────────────────────────────────────────────────────

/// A tool that can be called by the agent's LLM.
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Tool definition: name, description, parameters.
    fn definition(&self) -> AgentToolDef;

    /// Execute the tool with the given JSON parameters.
    async fn call(&self, params: &serde_json::Value, ctx: &AgentContext) -> ToolResult;
}

/// A channel through which tasks arrive and responses are sent.
#[async_trait]
pub trait AgentChannel: Send + Sync {
    /// Human-readable channel name (e.g. "cli", "slack", "discord").
    fn name(&self) -> &str;

    /// Block until a task arrives, returning `None` on EOF/close.
    async fn recv(&mut self) -> Option<AgentTask>;

    /// Send a response back (default channel output).
    async fn send(&self, response: &str);

    /// Send a response to a specific thread/channel ID (used by Slack/Discord).
    /// Default: falls back to `send()`.
    async fn send_to(&self, response: &str, _thread_id: &str) {
        self.send(response).await;
    }
}

// ── Task ──────────────────────────────────────────────────────────────────────

/// A task received from a channel.
#[derive(Debug, Clone)]
pub struct AgentTask {
    /// The task description / user message.
    pub content: String,
    /// Optional sender identity (username, user ID, etc.).
    pub sender: Option<String>,
    /// Optional thread/conversation/channel ID for routing replies.
    pub thread_id: Option<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_constructors() {
        let u = AgentMessage::user("hello");
        assert_eq!(u.role, MessageRole::User);
        assert_eq!(u.content, "hello");
        assert!(u.tool_name.is_none());

        let a = AgentMessage::assistant("world");
        assert_eq!(a.role, MessageRole::Assistant);

        let t = AgentMessage::tool_result("review_pr", "LGTM");
        assert_eq!(t.role, MessageRole::Tool);
        assert_eq!(t.tool_name.as_deref(), Some("review_pr"));
    }

    #[test]
    fn test_tool_def_prompt_line_no_params() {
        let def = AgentToolDef {
            name: "review_pr".to_string(),
            description: "Review the PR".to_string(),
            parameters: vec![],
        };
        let line = def.to_prompt_line();
        assert!(line.contains("`review_pr`"));
        assert!(line.contains("Review the PR"));
    }

    #[test]
    fn test_tool_def_prompt_line_with_params() {
        let def = AgentToolDef {
            name: "ask".to_string(),
            description: "Ask a question".to_string(),
            parameters: vec![ToolParameter {
                name: "question".to_string(),
                param_type: "string".to_string(),
                description: "The question".to_string(),
                required: true,
            }],
        };
        let line = def.to_prompt_line();
        assert!(line.contains("question"));
        assert!(line.contains("(required)"));
    }
}
