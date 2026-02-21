import React from 'react';
import { Text } from 'ink';
import type { Severity } from '../types.js';

interface SeverityBadgeProps {
  severity: Severity;
}

const SEVERITY_CONFIG: Record<
  Severity,
  { label: string; color: string; emoji: string }
> = {
  critical: { label: 'CRITICAL', color: 'red',     emoji: '🔴' },
  high:     { label: 'HIGH',     color: 'redBright', emoji: '🟠' },
  medium:   { label: 'MEDIUM',   color: 'yellow',   emoji: '🟡' },
  low:      { label: 'LOW',      color: 'blue',     emoji: '🔵' },
  info:     { label: 'INFO',     color: 'gray',     emoji: '⚪' },
};

/**
 * Coloured severity badge: `🔴 CRITICAL`, `🟡 MEDIUM`, etc.
 */
export function SeverityBadge({ severity }: SeverityBadgeProps) {
  const { label, color, emoji } = SEVERITY_CONFIG[severity] ?? SEVERITY_CONFIG.info;
  return (
    <Text color={color as any} bold>
      {emoji} {label}
    </Text>
  );
}
