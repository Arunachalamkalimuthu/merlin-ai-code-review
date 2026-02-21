#!/usr/bin/env node
/**
 * merlin-ui — Ink-powered terminal UI for the Merlin AI code review tool.
 *
 * Usage:
 *   merlin-ui                         # same as merlin-ui review
 *   merlin-ui review                  # run a PR review (CI mode)
 *   merlin-ui review --diff <file>    # review a local diff file
 *   merlin-ui agent                   # interactive ReAct agent REPL
 *   merlin-ui update                  # update Merlin to the latest release
 *   merlin-ui update --check          # check for updates without downloading
 *   merlin-ui status                  # show installed version + update check
 */

import React from 'react';
import { render } from 'ink';
import meow from 'meow';

import { ReviewCommand } from './commands/review.js';
import { AgentCommand }  from './commands/agent.js';
import { UpdateCommand } from './commands/update.js';
import { StatusCommand } from './commands/status.js';

// ── CLI definition ─────────────────────────────────────────────────────────────

const cli = meow(
  `
  Usage
    $ merlin-ui [command] [options]

  Commands
    review          Run a full PR/MR code review              (default)
    agent           Interactive autonomous agent REPL
    update          Update Merlin to the latest release
    status          Show installed version and check for updates

  Options (review)
    --diff <file>   Review a local diff file instead of live CI

  Options (update)
    --check         Check for updates without downloading
    --force         Reinstall even if already on the latest version

  Examples
    $ merlin-ui
    $ merlin-ui review --diff changes.diff
    $ merlin-ui agent
    $ merlin-ui update
    $ merlin-ui update --check
    $ merlin-ui status
`,
  {
    importMeta: import.meta,
    flags: {
      diff:  { type: 'string'  },
      check: { type: 'boolean', default: false },
      force: { type: 'boolean', default: false },
    },
  },
);

// ── Command dispatch ───────────────────────────────────────────────────────────

const command = cli.input[0] ?? 'review';

switch (command) {
  case 'review':
    render(<ReviewCommand diff={cli.flags.diff} />);
    break;

  case 'agent':
    render(<AgentCommand />);
    break;

  case 'update':
  case 'self-update':
    render(<UpdateCommand checkOnly={cli.flags.check} force={cli.flags.force} />);
    break;

  case 'status':
    render(<StatusCommand />);
    break;

  default:
    console.error(`Unknown command: ${command}`);
    console.error('Run `merlin-ui --help` for usage.');
    process.exit(1);
}
