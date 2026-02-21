//! AgentRuntime — the core ReAct (Reason → Act → Observe) loop.
//!
//! Each iteration:
//!   1. Build system prompt with tool catalogue
//!   2. Build conversation transcript from memory
//!   3. Call `AiProvider::generate(system, transcript)` → LLM response
//!   4. Parse `<tool_call>{...}</tool_call>` blocks from the response
//!   5. Execute tool calls, collect results
//!   6. Append assistant message + tool results to memory
//!   7. Repeat until: no tool calls (final answer) OR max_iterations reached

use std::sync::Arc;

use tracing::{debug, info, warn};

use super::{
    AgentChannel, AgentContext, AgentMessage, AgentTask, AgentTool, MessageRole, ToolCall,
    ToolResult,
};
use crate::agent::memory::AgentMemory;
use crate::agent::tools::builtin_tools;
use crate::config::AgentConfig;
use crate::error::Result;

/// The autonomous agent runtime.
pub struct AgentRuntime {
    ctx: AgentContext,
    tools: Vec<Arc<dyn AgentTool>>,
    memory: AgentMemory,
    max_iterations: usize,
}

impl AgentRuntime {
    /// Create a new runtime from config and context.
    pub fn new(ctx: AgentContext, config: &AgentConfig) -> Self {
        let memory = match &config.memory_file {
            Some(path) => AgentMemory::with_persistence(config.max_memory_messages, path.clone()),
            None => AgentMemory::new(config.max_memory_messages),
        };

        Self {
            ctx,
            tools: builtin_tools(),
            memory,
            max_iterations: config.max_iterations.unwrap_or(10),
        }
    }

    /// Register an additional custom tool.
    pub fn register_tool(&mut self, tool: Arc<dyn AgentTool>) {
        self.tools.push(tool);
    }

    /// Run the agent on a single task, returning the final Markdown response.
    pub async fn run(&mut self, task: &AgentTask) -> Result<String> {
        info!(
            "Agent task from {:?}: {}",
            task.sender.as_deref().unwrap_or("unknown"),
            task.content
        );

        self.memory.push(AgentMessage::user(task.content.clone()));

        let system = self.build_system_prompt();

        for iteration in 0..self.max_iterations {
            debug!("Agent iteration {}/{}", iteration + 1, self.max_iterations);

            let prompt = self.build_conversation_transcript();
            let response = self.ctx.ai.generate(&system, &prompt).await?;
            debug!("LLM raw response: {response}");

            // Parse tool calls from the LLM response
            let tool_calls = parse_tool_calls(&response);

            if tool_calls.is_empty() {
                // No tool calls → this IS the final answer
                self.memory.push(AgentMessage::assistant(response.clone()));
                return Ok(response);
            }

            // Append the "thinking" part (response minus tool_call blocks)
            let thought = strip_tool_calls(&response);
            if !thought.trim().is_empty() {
                self.memory.push(AgentMessage::assistant(thought));
            }

            // Execute tools one at a time.
            // We clone ctx (Arc fields are cheap) to avoid borrow-across-await.
            for call in &tool_calls {
                info!("Tool call: {} params={}", call.name, call.parameters);

                let ctx_clone = self.ctx.clone();
                let result = self.dispatch_tool(call, ctx_clone).await;

                info!("Tool {} → success={}", result.tool_name, result.success);

                let formatted = if result.success {
                    format!("✅ {}", result.output)
                } else {
                    format!("❌ Error: {}", result.output)
                };

                self.memory.push(AgentMessage::tool_result(call.name.clone(), formatted));
            }
        }

        // Reached max iterations — request a final answer
        warn!(
            "Agent reached max iterations ({}), requesting final answer",
            self.max_iterations
        );
        let system_final = format!(
            "{system}\n\nYou have used the maximum number of tool calls. \
             Provide your final answer now based on all the information gathered."
        );
        let prompt = self.build_conversation_transcript();
        let final_response = self.ctx.ai.generate(&system_final, &prompt).await?;
        self.memory.push(AgentMessage::assistant(final_response.clone()));
        Ok(final_response)
    }

    /// Run in channel mode: continuously receive tasks and reply.
    pub async fn run_channel(&mut self, channel: &mut dyn AgentChannel) -> Result<()> {
        info!("Agent running in '{}' channel mode", channel.name());

        while let Some(task) = channel.recv().await {
            let thread_id = task.thread_id.clone();
            let response = match self.run(&task).await {
                Ok(r) => r,
                Err(e) => format!("❌ Agent error: {e}"),
            };

            match thread_id.as_deref() {
                Some(tid) => channel.send_to(&response, tid).await,
                None => channel.send(&response).await,
            }
        }
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn build_system_prompt(&self) -> String {
        let tool_lines: Vec<String> =
            self.tools.iter().map(|t| t.definition().to_prompt_line()).collect();
        let tools_text = tool_lines.join("\n");

        format!(
            "You are Merlin 🦡, an autonomous AI code review agent. You help developers \
             review pull requests, discover security vulnerabilities, analyse code quality, \
             and manage development tasks across GitHub, GitLab, and other platforms.\n\n\
             ## How to call tools\n\n\
             When you need to use a tool, include a JSON block wrapped in `<tool_call>` tags:\n\
             <tool_call>{{\"name\": \"tool_name\", \"parameters\": {{\"key\": \"value\"}}}}</tool_call>\n\n\
             You may call multiple tools in one response. After receiving tool results, reason \
             about them and either call more tools or write your final answer.\n\n\
             ## Available tools\n\n\
             {tools_text}\n\n\
             ## Guidelines\n\n\
             - Always explain your reasoning briefly before calling a tool\n\
             - Use tools proactively to gather information rather than guessing\n\
             - When you have enough information, provide a clear, actionable final answer in Markdown\n\
             - If a tool returns an error, try an alternative approach or explain the limitation\n\
             - Be concise; focus on what matters most to the developer"
        )
    }

    /// Build a plain-text conversation transcript to pass as the `user` message.
    fn build_conversation_transcript(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for msg in self.memory.messages() {
            match msg.role {
                MessageRole::System => {}
                MessageRole::User => {
                    parts.push(format!("**User:** {}", msg.content));
                }
                MessageRole::Assistant => {
                    parts.push(format!("**Merlin:** {}", msg.content));
                }
                MessageRole::Tool => {
                    let name = msg.tool_name.as_deref().unwrap_or("tool");
                    parts.push(format!("**Tool[{name}] result:** {}", msg.content));
                }
            }
        }
        parts.join("\n\n")
    }

    /// Find and execute the tool matching `call.name`.
    async fn dispatch_tool(&self, call: &ToolCall, ctx: AgentContext) -> ToolResult {
        // Find the matching tool (borrows self.tools briefly)
        let tool: Option<Arc<dyn AgentTool>> =
            self.tools.iter().find(|t| t.definition().name == call.name).cloned();

        match tool {
            Some(t) => t.call(&call.parameters, &ctx).await,
            None => {
                let available: Vec<String> =
                    self.tools.iter().map(|t| t.definition().name.clone()).collect();
                ToolResult {
                    tool_name: call.name.clone(),
                    output: format!(
                        "Unknown tool '{}'. Available: {}",
                        call.name,
                        available.join(", ")
                    ),
                    success: false,
                }
            }
        }
    }
}

// ── Tool call parsing ─────────────────────────────────────────────────────────

/// Parse all `<tool_call>{...}</tool_call>` blocks from an LLM response.
pub fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
    let re = regex::Regex::new(r"<tool_call>([\s\S]*?)</tool_call>").unwrap();
    re.captures_iter(text)
        .filter_map(|cap| {
            let json = cap[1].trim();
            serde_json::from_str(json).ok()
        })
        .collect()
}

/// Remove all `<tool_call>...</tool_call>` blocks from text.
pub fn strip_tool_calls(text: &str) -> String {
    let re = regex::Regex::new(r"<tool_call>[\s\S]*?</tool_call>").unwrap();
    re.replace_all(text, "").to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls_empty() {
        assert!(parse_tool_calls("No tool calls here.").is_empty());
    }

    #[test]
    fn test_parse_tool_calls_single() {
        let text = r#"I'll review this PR.
<tool_call>{"name": "review", "parameters": {}}</tool_call>
Let me know."#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "review");
    }

    #[test]
    fn test_parse_tool_calls_multiple() {
        let text = r#"<tool_call>{"name": "review", "parameters": {}}</tool_call>
<tool_call>{"name": "snyk", "parameters": {}}</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "review");
        assert_eq!(calls[1].name, "snyk");
    }

    #[test]
    fn test_parse_tool_call_with_params() {
        let text = r#"<tool_call>{"name": "ask", "parameters": {"question": "Is this safe?"}}</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].parameters["question"], "Is this safe?");
    }

    #[test]
    fn test_strip_tool_calls() {
        let text =
            "Thinking...\n<tool_call>{\"name\":\"x\"}</tool_call>\nDone.";
        let stripped = strip_tool_calls(text);
        assert!(!stripped.contains("<tool_call>"));
        assert!(stripped.contains("Thinking"));
        assert!(stripped.contains("Done"));
    }

    #[test]
    fn test_strip_tool_calls_none() {
        let text = "Just a plain response.";
        assert_eq!(strip_tool_calls(text), text);
    }
}
