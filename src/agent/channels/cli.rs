//! CLI REPL channel — interactive terminal input/output.
//!
//! Reads task lines from stdin, prints Merlin's response to stdout.
//! Type `exit` or `quit` (or press Ctrl-D) to end the session.

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::agent::{AgentChannel, AgentTask};

/// Interactive terminal REPL channel.
pub struct CliChannel {
    reader: BufReader<tokio::io::Stdin>,
    prompt: &'static str,
}

impl CliChannel {
    /// Create a new CLI channel reading from stdin and writing to stdout.
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            prompt: "merlin> ",
        }
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentChannel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn recv(&mut self) -> Option<AgentTask> {
        use std::io::Write;
        print!("{}", self.prompt);
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        match self.reader.read_line(&mut line).await {
            Ok(0) => None, // EOF (Ctrl-D)
            Ok(_) => {
                let content = line.trim().to_string();
                match content.as_str() {
                    "" | "exit" | "quit" | "q" => None,
                    _ => Some(AgentTask {
                        content,
                        sender: Some("user".to_string()),
                        thread_id: None,
                    }),
                }
            }
            Err(_) => None,
        }
    }

    async fn send(&self, response: &str) {
        println!("\n🦡 Merlin:\n{response}\n");
    }
}
