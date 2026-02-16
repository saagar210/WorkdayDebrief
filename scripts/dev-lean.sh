#!/usr/bin/env bash
set -euo pipefail

TMP_BASE="${TMPDIR:-/tmp}"
LEAN_TMP_ROOT="$(mktemp -d "${TMP_BASE%/}/workdaydebrief-lean.XXXXXX")"

export CARGO_TARGET_DIR="${LEAN_TMP_ROOT}/cargo-target"
export VITE_CACHE_DIR="${LEAN_TMP_ROOT}/vite-cache"

cleanup() {
  rm -rf "${LEAN_TMP_ROOT}"
  npm run -s clean:heavy >/dev/null 2>&1 || true
}

trap cleanup EXIT

npm run tauri dev
