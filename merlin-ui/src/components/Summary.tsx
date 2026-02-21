import React from 'react';
import { Box, Text } from 'ink';
import type { ReviewComment, Severity } from '../types.js';

interface SummaryProps {
  comments: ReviewComment[];
  elapsedMs: number;
}

function count(comments: ReviewComment[], sev: Severity) {
  return comments.filter((c) => c.severity === sev).length;
}

/**
 * Summary bar shown after all comments, e.g.:
 *   3 issues  🔴 1 critical  🟠 1 high  🟡 1 medium  — 4.2s
 */
export function Summary({ comments, elapsedMs }: SummaryProps) {
  if (comments.length === 0) {
    return (
      <Box marginTop={1}>
        <Text color="green" bold>
          ✅ No issues found. Great work!
        </Text>
        <Text color="gray">  ({(elapsedMs / 1000).toFixed(1)}s)</Text>
      </Box>
    );
  }

  const critical = count(comments, 'critical');
  const high     = count(comments, 'high');
  const medium   = count(comments, 'medium');
  const low      = count(comments, 'low');
  const info     = count(comments, 'info');

  return (
    <Box flexDirection="column" marginTop={1} borderStyle="round" borderColor="yellow" paddingX={1}>
      <Box gap={2}>
        <Text bold>{comments.length} issue{comments.length !== 1 ? 's' : ''}</Text>
        {critical > 0 && <Text color="red"   bold>🔴 {critical} critical</Text>}
        {high     > 0 && <Text color="redBright">🟠 {high} high</Text>}
        {medium   > 0 && <Text color="yellow">🟡 {medium} medium</Text>}
        {low      > 0 && <Text color="blue">🔵 {low} low</Text>}
        {info     > 0 && <Text color="gray">⚪ {info} info</Text>}
        <Text color="gray">— {(elapsedMs / 1000).toFixed(1)}s</Text>
      </Box>
    </Box>
  );
}
