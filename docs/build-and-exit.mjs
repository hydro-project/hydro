#!/usr/bin/env node
/**
 * Wrapper around docusaurus build that explicitly exits after completion.
 * This works around docusaurus CLI not calling process.exit(0) on success,
 * which can leave the process hanging if any event loop handles remain open.
 */

import {runCLI} from '@docusaurus/core/lib/index.js';

try {
  // Run docusaurus build command
  await runCLI(['node', 'docusaurus', 'build']);
  // Force exit on success - docusaurus doesn't do this
  process.exit(0);
} catch (error) {
  console.error('Build failed:', error);
  process.exit(1);
}
