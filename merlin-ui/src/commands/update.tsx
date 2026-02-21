/**
 * `merlin-ui update [--check]`
 *
 * Streams live output from `merlin self-update` and renders each line
 * until the process exits.
 */
import React, { useEffect, useState } from 'react';
import { Box, Text, useApp } from 'ink';
import Spinner from 'ink-spinner';
import { execa } from 'execa';

import { Header } from '../components/Header.js';

interface UpdateProps {
  checkOnly?: boolean;
  force?: boolean;
}

type Phase = 'running' | 'done' | 'error';

export function UpdateCommand({ checkOnly = false, force = false }: UpdateProps) {
  const { exit } = useApp();

  const [phase, setPhase]   = useState<Phase>('running');
  const [lines, setLines]   = useState<string[]>([]);
  const [error, setError]   = useState('');

  useEffect(() => {
    let cancelled = false;

    async function run() {
      const args = ['self-update'];
      if (checkOnly) args.push('--check');
      if (force)     args.push('--force');

      try {
        const proc = execa('merlin', args, { reject: false, all: true });

        proc.stdout?.on('data', (chunk: Buffer) => {
          if (cancelled) return;
          const newLines = chunk.toString().split('\n').filter(Boolean);
          setLines((prev) => [...prev, ...newLines]);
        });

        proc.stderr?.on('data', (chunk: Buffer) => {
          if (cancelled) return;
          const newLines = chunk.toString().split('\n').filter(Boolean);
          setLines((prev) => [...prev, ...newLines]);
        });

        const result = await proc;
        if (cancelled) return;

        if (result.exitCode !== 0) {
          setError(`merlin exited with code ${result.exitCode}`);
          setPhase('error');
        } else {
          setPhase('done');
        }
        setTimeout(exit, 200);
      } catch (err: any) {
        if (!cancelled) {
          setError(err.message ?? String(err));
          setPhase('error');
          setTimeout(exit, 200);
        }
      }
    }

    run();
    return () => { cancelled = true; };
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <Box flexDirection="column">
      <Header subtitle={checkOnly ? 'Checking for updates…' : 'Self-update'} />

      {/* Live output lines */}
      <Box flexDirection="column" marginBottom={1}>
        {lines.map((line, i) => (
          <Text key={i} color={line.includes('error') || line.includes('Error') ? 'red' : 'white'}>
            {line}
          </Text>
        ))}
      </Box>

      {phase === 'running' && (
        <Box gap={1}>
          <Text color="green"><Spinner type="dots" /></Text>
          <Text color="gray">{checkOnly ? 'Querying GitHub releases…' : 'Updating…'}</Text>
        </Box>
      )}

      {phase === 'done' && (
        <Text color="green" bold>✔ Done</Text>
      )}

      {phase === 'error' && (
        <Text color="red" bold>✖ {error}</Text>
      )}
    </Box>
  );
}
