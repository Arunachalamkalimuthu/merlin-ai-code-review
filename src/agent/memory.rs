//! Agent conversation memory — short-term (in-memory ring buffer) + optional
//! long-term persistence (JSONL file, one message per line).

use std::collections::VecDeque;
use std::io::Write;

use super::AgentMessage;

/// Conversation memory with optional JSONL file persistence.
pub struct AgentMemory {
    messages: VecDeque<AgentMessage>,
    max_messages: usize,
    persist_path: Option<String>,
}

impl AgentMemory {
    /// Create an in-memory-only memory store.
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_messages,
            persist_path: None,
        }
    }

    /// Create a memory store that also appends messages to a JSONL file.
    /// Existing messages are loaded from the file on startup.
    pub fn with_persistence(max_messages: usize, path: String) -> Self {
        let messages = VecDeque::from(Self::load_from_file(&path));
        Self {
            messages,
            max_messages,
            persist_path: Some(path),
        }
    }

    /// Add a message to memory. Evicts oldest message when at capacity.
    pub fn push(&mut self, message: AgentMessage) {
        if let Some(path) = &self.persist_path {
            append_to_file(path, &message);
        }
        self.messages.push_back(message);
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    /// Immutable view of the current conversation history.
    pub fn messages(&self) -> &VecDeque<AgentMessage> {
        &self.messages
    }

    /// Number of messages currently stored.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// True if no messages are stored.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all in-memory messages (does NOT truncate the persistence file).
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    fn load_from_file(path: &str) -> Vec<AgentMessage> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

fn append_to_file(path: &str, message: &AgentMessage) {
    if let Ok(line) = serde_json::to_string(message) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::MessageRole;

    #[test]
    fn test_memory_push_and_evict() {
        let mut mem = AgentMemory::new(3);
        mem.push(AgentMessage::user("a"));
        mem.push(AgentMessage::user("b"));
        mem.push(AgentMessage::user("c"));
        assert_eq!(mem.len(), 3);

        mem.push(AgentMessage::user("d"));
        assert_eq!(mem.len(), 3);
        // Oldest ("a") should be evicted
        assert_eq!(mem.messages().front().unwrap().content, "b");
        assert_eq!(mem.messages().back().unwrap().content, "d");
    }

    #[test]
    fn test_memory_clear() {
        let mut mem = AgentMemory::new(10);
        mem.push(AgentMessage::user("hello"));
        assert!(!mem.is_empty());
        mem.clear();
        assert!(mem.is_empty());
    }

    #[test]
    fn test_memory_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("memory.jsonl")
            .to_string_lossy()
            .to_string();

        {
            let mut mem = AgentMemory::with_persistence(100, path.clone());
            mem.push(AgentMessage::user("hello"));
            mem.push(AgentMessage::assistant("world"));
        }

        // Reload from file
        let mem = AgentMemory::with_persistence(100, path);
        assert_eq!(mem.len(), 2);
        assert_eq!(mem.messages().front().unwrap().role, MessageRole::User);
        assert_eq!(mem.messages().back().unwrap().role, MessageRole::Assistant);
    }
}
