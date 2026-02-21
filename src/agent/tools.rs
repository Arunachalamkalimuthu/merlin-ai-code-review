//! Built-in agent tools — wrap Ferret's existing slash commands and platform
//! operations as callable `AgentTool` implementations exposed to the LLM.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::{AgentContext, AgentTool, AgentToolDef, ToolParameter, ToolResult};
use crate::tools::{route_command, ToolContext};

// ── Factory ───────────────────────────────────────────────────────────────────

/// Return the full set of built-in agent tools.
pub fn builtin_tools() -> Vec<Arc<dyn AgentTool>> {
    let slash_no_arg: &[(&str, &str)] = &[
        ("review",           "Run a full AI code review on the current PR/MR and post inline comments."),
        ("describe",         "Generate and post a description for the PR/MR based on the diff."),
        ("security",         "Run a dedicated security analysis on the PR changes."),
        ("snyk",             "Scan changed dependencies against the Snyk vulnerability database."),
        ("coverage",         "Analyse test coverage for the files changed in this PR."),
        ("link_jira",        "Find related Jira issues and link them to the PR as a comment."),
        ("link_linear",      "Find related Linear issues and link them to the PR as a comment."),
        ("generate_labels",  "Generate appropriate labels for the PR based on the changes."),
        ("commit_message",   "Generate a conventional commit message for the PR changes."),
        ("update_changelog", "Update CHANGELOG.md with an entry for this PR."),
        ("test",             "Generate unit tests for the changed code."),
        ("docs",             "Generate or update documentation for the changed code."),
        ("approve",          "Approve the PR if no blocking issues are found."),
        ("triage",           "Find similar open issues on CodeTriage for the changed packages."),
        ("similar_issue",    "Find issues in the same repo that are similar to this PR."),
    ];

    let mut tools: Vec<Arc<dyn AgentTool>> = slash_no_arg
        .iter()
        .map(|(cmd, desc)| {
            Arc::new(SlashCommandTool {
                command: format!("/{cmd}"),
                tool_name: cmd.to_string(),
                description: desc.to_string(),
            }) as Arc<dyn AgentTool>
        })
        .collect();

    // Commands that take a string argument
    tools.push(Arc::new(SlashArgTool {
        command: "/ask".to_string(),
        tool_name: "ask".to_string(),
        description: "Ask a specific question about the PR diff.".to_string(),
        param_name: "question",
        param_description: "The question to ask about the code.",
    }));
    tools.push(Arc::new(SlashArgTool {
        command: "/improve".to_string(),
        tool_name: "improve".to_string(),
        description: "Suggest improvements for a specific part of the code.".to_string(),
        param_name: "focus",
        param_description: "Optional: what aspect to improve (e.g. 'error handling', 'performance').",
    }));
    tools.push(Arc::new(SlashArgTool {
        command: "/explain".to_string(),
        tool_name: "explain".to_string(),
        description: "Explain what a part of the diff does in plain English.".to_string(),
        param_name: "target",
        param_description: "Optional: which file or function to explain.",
    }));

    // Platform-level tools
    tools.push(Arc::new(PostCommentTool));
    tools.push(Arc::new(GetPrInfoTool));

    // RAG search
    tools.push(Arc::new(RagSearchTool));

    tools
}

// ── Slash command wrapper (no arg) ────────────────────────────────────────────

struct SlashCommandTool {
    command: String,
    tool_name: String,
    description: String,
}

#[async_trait]
impl AgentTool for SlashCommandTool {
    fn definition(&self) -> AgentToolDef {
        AgentToolDef {
            name: self.tool_name.clone(),
            description: self.description.clone(),
            parameters: vec![],
        }
    }

    async fn call(&self, _params: &Value, ctx: &AgentContext) -> ToolResult {
        run_slash_command(&self.command, None, ctx).await
    }
}

// ── Slash command wrapper (with string arg) ───────────────────────────────────

struct SlashArgTool {
    command: String,
    tool_name: String,
    description: String,
    param_name: &'static str,
    param_description: &'static str,
}

#[async_trait]
impl AgentTool for SlashArgTool {
    fn definition(&self) -> AgentToolDef {
        AgentToolDef {
            name: self.tool_name.clone(),
            description: self.description.clone(),
            parameters: vec![ToolParameter {
                name: self.param_name.to_string(),
                param_type: "string".to_string(),
                description: self.param_description.to_string(),
                required: false,
            }],
        }
    }

    async fn call(&self, params: &Value, ctx: &AgentContext) -> ToolResult {
        let arg = params[self.param_name].as_str().map(str::to_string);
        run_slash_command(&self.command, arg, ctx).await
    }
}

// ── Common slash command dispatch ─────────────────────────────────────────────

async fn run_slash_command(
    command: &str,
    arg: Option<String>,
    ctx: &AgentContext,
) -> ToolResult {
    let tool = match route_command(command) {
        Ok(t) => t,
        Err(e) => {
            return ToolResult {
                tool_name: command.to_string(),
                output: format!("Command routing error: {e}"),
                success: false,
            }
        }
    };

    let platform = match &ctx.platform {
        Some(p) => p.clone(),
        None => {
            return ToolResult {
                tool_name: command.to_string(),
                output: "No VCS platform configured. Set GITHUB_TOKEN / GITLAB_TOKEN.".to_string(),
                success: false,
            }
        }
    };

    let tool_ctx = ToolContext { ai: ctx.ai.clone(), platform, arg };

    match tool.run(&tool_ctx).await {
        Ok(output) => ToolResult { tool_name: command.to_string(), output, success: true },
        Err(e) => ToolResult { tool_name: command.to_string(), output: e.to_string(), success: false },
    }
}

// ── post_comment ──────────────────────────────────────────────────────────────

struct PostCommentTool;

#[async_trait]
impl AgentTool for PostCommentTool {
    fn definition(&self) -> AgentToolDef {
        AgentToolDef {
            name: "post_comment".to_string(),
            description: "Post a custom Markdown comment to the PR/MR.".to_string(),
            parameters: vec![ToolParameter {
                name: "body".to_string(),
                param_type: "string".to_string(),
                description: "The Markdown body of the comment.".to_string(),
                required: true,
            }],
        }
    }

    async fn call(&self, params: &Value, ctx: &AgentContext) -> ToolResult {
        let body = match params["body"].as_str() {
            Some(b) if !b.is_empty() => b.to_string(),
            _ => {
                return ToolResult {
                    tool_name: "post_comment".to_string(),
                    output: "Missing required parameter 'body'.".to_string(),
                    success: false,
                }
            }
        };

        match &ctx.platform {
            Some(platform) => match platform.post_summary(&body).await {
                Ok(_) => ToolResult {
                    tool_name: "post_comment".to_string(),
                    output: "Comment posted successfully.".to_string(),
                    success: true,
                },
                Err(e) => ToolResult {
                    tool_name: "post_comment".to_string(),
                    output: e.to_string(),
                    success: false,
                },
            },
            None => ToolResult {
                tool_name: "post_comment".to_string(),
                output: "No platform configured.".to_string(),
                success: false,
            },
        }
    }
}

// ── get_pr_info ───────────────────────────────────────────────────────────────

struct GetPrInfoTool;

#[async_trait]
impl AgentTool for GetPrInfoTool {
    fn definition(&self) -> AgentToolDef {
        AgentToolDef {
            name: "get_pr_info".to_string(),
            description:
                "Get metadata about the current PR/MR: title, author, branches, stats, labels."
                    .to_string(),
            parameters: vec![],
        }
    }

    async fn call(&self, _params: &Value, ctx: &AgentContext) -> ToolResult {
        match &ctx.platform {
            Some(platform) => match platform.get_pr_info().await {
                Ok(info) => {
                    let labels = if info.labels.is_empty() {
                        "none".to_string()
                    } else {
                        info.labels.join(", ")
                    };
                    let output = format!(
                        "**PR #{number}:** {title}\n\
                         **Author:** {author}\n\
                         **Branches:** `{base}` ← `{head}`\n\
                         **Draft:** {draft}\n\
                         **Labels:** {labels}\n\
                         **Changes:** +{add} / -{del} across {files} file(s)\n\
                         **Description:**\n{body}",
                        number = info.number,
                        title = info.title,
                        author = info.author,
                        base = info.base_branch,
                        head = info.head_branch,
                        draft = info.is_draft,
                        labels = labels,
                        add = info.additions,
                        del = info.deletions,
                        files = info.files_changed,
                        body = if info.body.is_empty() { "(no description)" } else { &info.body },
                    );
                    ToolResult { tool_name: "get_pr_info".to_string(), output, success: true }
                }
                Err(e) => ToolResult {
                    tool_name: "get_pr_info".to_string(),
                    output: e.to_string(),
                    success: false,
                },
            },
            None => ToolResult {
                tool_name: "get_pr_info".to_string(),
                output: "No platform configured.".to_string(),
                success: false,
            },
        }
    }
}

// ── rag_search ────────────────────────────────────────────────────────────────

struct RagSearchTool;

#[async_trait]
impl AgentTool for RagSearchTool {
    fn definition(&self) -> AgentToolDef {
        AgentToolDef {
            name: "rag_search".to_string(),
            description: "Search the RAG (vector) index for code, past review comments, or \
                          documentation relevant to a query. Returns the top matching snippets."
                .to_string(),
            parameters: vec![
                ToolParameter {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "Natural-language or code search query.".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "limit".to_string(),
                    param_type: "number".to_string(),
                    description: "Maximum number of results to return (default: 5).".to_string(),
                    required: false,
                },
            ],
        }
    }

    async fn call(&self, params: &Value, ctx: &AgentContext) -> ToolResult {
        let query = match params["query"].as_str() {
            Some(q) if !q.is_empty() => q.to_string(),
            _ => {
                return ToolResult {
                    tool_name: "rag_search".to_string(),
                    output: "Missing required parameter 'query'.".to_string(),
                    success: false,
                }
            }
        };
        let limit = params["limit"].as_u64().unwrap_or(5) as usize;

        if !ctx.config.rag.enabled {
            return ToolResult {
                tool_name: "rag_search".to_string(),
                output: "RAG is not enabled. Set `[rag] enabled = true` in merlin.toml.".to_string(),
                success: false,
            };
        }

        let pipeline = crate::rag::build_pipeline(&ctx.config.rag);
        match pipeline.retrieve(&query, limit).await {
            Ok(docs) if docs.is_empty() => ToolResult {
                tool_name: "rag_search".to_string(),
                output: format!("No relevant results found for: {query}"),
                success: true,
            },
            Ok(docs) => {
                let output = crate::rag::retriever::format_rag_context(&docs);
                ToolResult { tool_name: "rag_search".to_string(), output, success: true }
            }
            Err(e) => ToolResult {
                tool_name: "rag_search".to_string(),
                output: format!("RAG search failed: {e}"),
                success: false,
            },
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_tools_have_unique_names() {
        let tools = builtin_tools();
        let mut names = std::collections::HashSet::new();
        for tool in &tools {
            let name = tool.definition().name;
            assert!(names.insert(name.clone()), "Duplicate tool name: {name}");
        }
    }

    #[test]
    fn test_all_tools_have_non_empty_description() {
        for tool in builtin_tools() {
            let def = tool.definition();
            assert!(!def.description.is_empty(), "Tool '{}' has empty description", def.name);
        }
    }

    #[test]
    fn test_post_comment_tool_requires_body() {
        // This just tests the definition metadata; actual call requires async.
        let tool = PostCommentTool;
        let def = tool.definition();
        assert_eq!(def.name, "post_comment");
        assert!(def.parameters.iter().any(|p| p.name == "body" && p.required));
    }
}
