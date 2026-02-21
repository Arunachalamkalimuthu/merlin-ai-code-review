#!/usr/bin/env node
// Entry point — delegates to the compiled ESM bundle.
import('../dist/index.js').catch((err) => {
  console.error('[merlin-ui] Failed to start:', err.message);
  process.exit(1);
});
