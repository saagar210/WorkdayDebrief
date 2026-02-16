#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

const mode = process.argv[2];

const HEAVY_TARGETS = [
  'dist',
  'node_modules/.vite',
  'src-tauri/target',
  'src-tauri/gen',
  '.vite',
  'tsconfig.tsbuildinfo',
];

const FULL_ONLY_TARGETS = [
  'node_modules',
  '.codex_audit',
];

if (mode !== 'heavy' && mode !== 'full') {
  console.error('Usage: node ./scripts/cleanup.mjs <heavy|full>');
  process.exit(1);
}

const targets = mode === 'full'
  ? [...HEAVY_TARGETS, ...FULL_ONLY_TARGETS]
  : HEAVY_TARGETS;

for (const target of targets) {
  try {
    fs.rmSync(target, { recursive: true, force: true });
    console.log(`removed ${target}`);
  } catch {
    // Best-effort cleanup.
  }
}

function removeOsNoise(root) {
  let entries = [];
  try {
    entries = fs.readdirSync(root, { withFileTypes: true });
  } catch {
    return;
  }

  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      removeOsNoise(fullPath);
      continue;
    }
    if (entry.name === '.DS_Store') {
      try {
        fs.rmSync(fullPath, { force: true });
      } catch {
        // Best-effort cleanup.
      }
    }
  }
}

removeOsNoise('.');
