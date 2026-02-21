/**
 * `merlin-ui review [--diff <file>] [--output json]`
 *
 * Spawns the `merlin` binary with JSON output, streams stderr as a live
 * status line, then renders all comments as coloured cards once done.
 */
import React, { useEffect, useState } from 'react';
import { Box, Text, useApp } from 'ink';
import Spinner from 'ink-spinner';
import { execa } from 'execa';

import { Header } from '../components/Header.js';
import { CommentCard } from '../components/CommentCard.js';
import { Summary } from '../components/Summary.js';
import type { ReviewComment } from '../types.js';

interface ReviewProps {
  diff?: string;
}

type Phase = 'running' | 'done' | 'error';

export function ReviewCommand({ diff }: ReviewProps) {
  const { exit } = useApp();

  const [phase, setPhase]       = useState<Phase>('running');
  const [status, setStatus]     = useState('Fetching diff…');
  const [comments, setComments] = useState<ReviewComment[]>([]);
  const [errorMsg, setErrorMsg] = useState('');
  const [startMs]               = useState(() => Date.now());
  const [elapsedMs, setElapsed] = useState(0);

  useEffect(() => {
    let cancelled = false;

    async function run() {
      try {
        const args = ['review', '--output', 'json'];
        if (diff) args.push('--diff', diff);

        const proc = execa('merlin', args, {
          reject: false,
          all: true,
        });

        // Stream stderr lines as a live status message
        proc.stderr?.on('data', (chunk: Buffer) => {
          const line = chunk.toString().trim();
          if (line && !cancelled) setStatus(line.replace(/^\[merlin\]\s*/, ''));
        });

        const result = await proc;
        if (cancelled) return;

        const elapsed = Date.now() - startMs;
        setElapsed(elapsed);

        if (result.exitCode !== 0) {
          setErrorMsg(result.stderr || result.stdout || 'merlin exited with an error');
          setPhase('error');
          setTimeout(exit, 100);
          return;
        }

        // Parse JSON from stdout
        let parsed: ReviewComment[] = [];
        try {
          const raw = result.stdout.trim();
          if (raw.startsWith('[')) {
            parsed = JSON.parse(raw) as ReviewComment[];
          }
        } catch {
          // Non-fatal: render empty list
        }

        setComments(parsed);
        setPhase('done');
        setTimeout(exit, 100);
      } catch (err: any) {
        if (!cancelled) {
          setErrorMsg(err.message ?? String(err));
          setPhase('error');
          setTimeout(exit, 100);
        }
      }
    }

    run();
    return () => { cancelled = true; };
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <Box flexDirection="column">
      <Header subtitle={diff ? `Local diff: ${diff}` : 'CI / PR review'} />

      {phase === 'running' && (
        <Box gap={1}>
          <Text color="green"><Spinner type="dots" /></Text>
          <Text color="gray">{status}</Text>
        </Box>
      )}

      {phase === 'error' && (
        <Box flexDirection="column">
          <Text color="red" bold>✖ Review failed</Text>
          <Text color="red">{errorMsg}</Text>
        </Box>
      )}

      {phase === 'done' && (
        <Box flexDirection="column">
          {comments.map((c, i) => (
            <CommentCard key={`${c.file}:${c.line}:${i}`} comment={c} index={i} />
          ))}
          <Summary comments={comments} elapsedMs={elapsedMs} />
        </Box>
      )}
    </Box>
  );
}
