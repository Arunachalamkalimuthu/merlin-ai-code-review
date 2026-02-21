import React from 'react';
import { Box, Text } from 'ink';

interface HeaderProps {
  subtitle?: string;
}

/**
 * Merlin brand header shown at the top of every command.
 */
export function Header({ subtitle }: HeaderProps) {
  return (
    <Box flexDirection="column" marginBottom={1}>
      <Box>
        <Text bold color="magenta">
          {'🧙 Merlin'}
        </Text>
        <Text color="gray"> — AI code review</Text>
      </Box>
      {subtitle && (
        <Text color="gray" dimColor>
          {subtitle}
        </Text>
      )}
    </Box>
  );
}
