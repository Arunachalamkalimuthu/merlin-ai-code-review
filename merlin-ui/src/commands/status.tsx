/**
 * `merlin-ui status`
 *
 * Shows the installed Merlin binary version and checks GitHub for updates.
 */
import React, { useEffect, useState } from 'react';
import { Box, Text, useApp } from 'ink';
import Spinner from 'ink-spinner';
import { execa } from 'execa';

import { Header } from '../components/Header.js';

type Phase = 'checking' | 'done' | 'error';

export function StatusCommand() {
  const { exit } = useApp();

  const [phase, setPhase]       = useState<Phase>('checking');
  const [version, setVersion]   = useState('');
  const [latest, setLatest]     = useState('');
  const [updateAvail, setUpdateAvail] = useState(false);
  const [binaryPath, setBinaryPath]   = useState('');

  useEffect(() => {
    let cancelled = false;

    async function run() {
      try {
        // Get current version
        const verResult = await execa('merlin', ['--version'], { reject: false });
        const verLine = (verResult.stdout || verResult.stderr).trim();
        const verMatch = verLine.match(/(\d+\.\d+\.\d+)/);
        const currentVer = verMatch ? verMatch[1] : verLine;
        if (!cancelled) setVersion(currentVer ?? 'unknown');

        // Get binary path
        const whichResult = await execa('which', ['merlin'], { reject: false });
        if (!cancelled) setBinaryPath(whichResult.stdout.trim());

        // Check for update via the --check flag
        const checkResult = await execa(
          'merlin',
          ['self-update', '--check'],
          { reject: false },
        );
        const checkOutput = (checkResult.stdout + checkResult.stderr).trim();
        if (!cancelled) {
          // Look for "v0.1.3" in the output
          const latestMatch = checkOutput.match(/v(\d+\.\d+\.\d+)/g);
          if (latestMatch && latestMatch.length >= 1) {
            const latestVer = latestMatch[latestMatch.length - 1]!.replace('v', '');
            setLatest(latestVer);
            setUpdateAvail(latestVer !== currentVer);
          }
          setPhase('done');
          setTimeout(exit, 100);
        }
      } catch (err: any) {
        if (!cancelled) {
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
      <Header />

      {phase === 'checking' && (
        <Box gap={1}>
          <Text color="green"><Spinner type="dots" /></Text>
          <Text color="gray">Checking version…</Text>
        </Box>
      )}

      {phase === 'done' && (
        <Box flexDirection="column" borderStyle="round" borderColor="cyan" paddingX={2} paddingY={1}>
          <Box gap={2}>
            <Text color="gray">Installed version</Text>
            <Text color="cyan" bold>v{version}</Text>
          </Box>

          {binaryPath && (
            <Box gap={2}>
              <Text color="gray">Binary path      </Text>
              <Text>{binaryPath}</Text>
            </Box>
          )}

          {latest && (
            <Box gap={2}>
              <Text color="gray">Latest release   </Text>
              <Text color={updateAvail ? 'yellow' : 'green'} bold>
                v{latest}
              </Text>
            </Box>
          )}

          {updateAvail ? (
            <Box marginTop={1}>
              <Text color="yellow" bold>
                ⬆  Update available! Run: <Text color="white">merlin-ui update</Text>
              </Text>
            </Box>
          ) : (
            <Box marginTop={1}>
              <Text color="green">✔ You are on the latest version.</Text>
            </Box>
          )}
        </Box>
      )}

      {phase === 'error' && (
        <Text color="red">Could not determine Merlin version. Is it installed?</Text>
      )}
    </Box>
  );
}
