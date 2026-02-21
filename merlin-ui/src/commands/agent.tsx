/**
 * `merlin-ui agent`
 *
 * Interactive Ink-based REPL that drives `merlin agent --task <input>`
 * in single-shot mode for each user message, displaying responses in a
 * scrollable conversation history.
 */
import React, { useState, useCallback } from 'react';
import { Box, Text, useApp, useInput } from 'ink';
import TextInput from 'ink-text-input';
import Spinner from 'ink-spinner';
import { execa } from 'execa';

import { Header } from '../components/Header.js';
import type { AgentMessage } from '../types.js';

type AgentPhase = 'idle' | 'thinking';

export function AgentCommand() {
  const { exit } = useApp();

  const [messages, setMessages]   = useState<AgentMessage[]>([]);
  const [input, setInput]         = useState('');
  const [phase, setPhase]         = useState<AgentPhase>('idle');
  const [statusLine, setStatus]   = useState('');

  // Ctrl+C / Ctrl+D exits cleanly
  useInput((_input, key) => {
    if (key.ctrl && (_input === 'c' || _input === 'd')) exit();
  });

  const handleSubmit = useCallback(
    async (query: string) => {
      const trimmed = query.trim();
      if (!trimmed) return;
      if (trimmed.toLowerCase() === 'exit' || trimmed.toLowerCase() === 'quit') {
        exit();
        return;
      }

      setInput('');
      setMessages((prev) => [...prev, { role: 'user', content: trimmed }]);
      setPhase('thinking');
      setStatus('Thinking…');

      try {
        const result = await execa('merlin', ['agent', '--task', trimmed], {
          reject: false,
        });
        const reply = result.stdout.trim() || result.stderr.trim() || '(no response)';
        setMessages((prev) => [...prev, { role: 'assistant', content: reply }]);
      } catch (err: any) {
        setMessages((prev) => [
          ...prev,
          { role: 'assistant', content: `Error: ${err.message ?? String(err)}` },
        ]);
      } finally {
        setPhase('idle');
        setStatus('');
      }
    },
    [exit],
  );

  return (
    <Box flexDirection="column">
      <Header subtitle="Autonomous agent — type a task, press Enter. Type 'exit' to quit." />

      {/* Conversation history */}
      <Box flexDirection="column" marginBottom={1}>
        {messages.map((msg, i) => (
          <Box key={i} flexDirection="column" marginBottom={1}>
            {msg.role === 'user' ? (
              <Box gap={1}>
                <Text color="cyan" bold>You</Text>
                <Text>{msg.content}</Text>
              </Box>
            ) : (
              <Box flexDirection="column">
                <Text color="magenta" bold>🧙 Merlin</Text>
                <Box
                  borderStyle="round"
                  borderColor="magenta"
                  paddingX={1}
                  marginLeft={2}
                >
                  <Text wrap="wrap">{msg.content}</Text>
                </Box>
              </Box>
            )}
          </Box>
        ))}
      </Box>

      {/* Thinking indicator */}
      {phase === 'thinking' && (
        <Box gap={1} marginBottom={1}>
          <Text color="magenta"><Spinner type="dots" /></Text>
          <Text color="gray">{statusLine}</Text>
        </Box>
      )}

      {/* Input prompt */}
      {phase === 'idle' && (
        <Box gap={1}>
          <Text color="cyan" bold>{'>'}</Text>
          <TextInput
            value={input}
            onChange={setInput}
            onSubmit={handleSubmit}
            placeholder="Type a task…"
          />
        </Box>
      )}
    </Box>
  );
}
