import React from 'react';
import { Box, Text } from 'ink';
import type { ReviewComment } from '../types.js';
import { SeverityBadge } from './SeverityBadge.js';

interface CommentCardProps {
  comment: ReviewComment;
  index: number;
}

const CATEGORY_COLOR: Record<string, string> = {
  bug:         'red',
  security:    'magenta',
  style:       'cyan',
  performance: 'yellow',
};

/**
 * Renders a single review comment as a bordered card.
 */
export function CommentCard({ comment, index }: CommentCardProps) {
  const catColor = CATEGORY_COLOR[comment.category] ?? 'white';

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={comment.severity === 'critical' || comment.severity === 'high' ? 'red' : 'gray'}
      paddingX={1}
      marginBottom={1}
    >
      {/* Header row */}
      <Box justifyContent="space-between" marginBottom={1}>
        <Box gap={1}>
          <Text dimColor>#{index + 1}</Text>
          <SeverityBadge severity={comment.severity} />
          <Text color={catColor as any}>[{comment.category}]</Text>
        </Box>
        <Text color="cyan">
          {comment.file}:{comment.line}
        </Text>
      </Box>

      {/* Title */}
      <Text bold>{comment.title}</Text>

      {/* Body */}
      <Box marginTop={1}>
        <Text wrap="wrap">{comment.body}</Text>
      </Box>

      {/* Suggestion block */}
      {comment.suggestion && (
        <Box
          flexDirection="column"
          marginTop={1}
          paddingX={1}
          borderStyle="single"
          borderColor="green"
        >
          <Text color="green" bold>
            Suggestion
          </Text>
          <Text color="greenBright">{comment.suggestion}</Text>
        </Box>
      )}
    </Box>
  );
}
