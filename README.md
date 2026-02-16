# WorkdayDebrief

WorkdayDebrief is a Tauri desktop app that collects daily work activity and generates end-of-day summaries using a local LLM (Ollama).

## What it does

- Aggregates activity from Jira, Google Calendar, and Toggl
- Generates narrative summaries with a local model through Ollama
- Lets you review, edit, export, and deliver summaries (email/Slack/file)
- Stores summary history locally in SQLite

## Stack

- Frontend: React 19, TypeScript, Vite, Tailwind CSS
- Desktop/runtime: Tauri 2, Rust
- Data + services: SQLite (`sqlx`), OAuth2 (Google), reqwest, lettre

## Prerequisites

- macOS
- Node.js and npm
- Rust toolchain
- Ollama running locally

## Quick start

```bash
npm ci
npm run tauri dev
```

## Dev modes

### Normal dev (fastest rebuilds, more local disk usage)

```bash
npm run tauri dev
```

This keeps Rust and Vite build caches in the repo (`src-tauri/target`, `node_modules/.vite`) for faster restart and incremental compile times.

### Lean dev (lower disk usage, slower cold starts)

```bash
npm run dev:lean
```

`dev:lean` runs the same Tauri dev flow, but redirects heavy build caches to a temporary directory and removes heavy generated artifacts when the app exits.

## Build

```bash
npm run build
npm run tauri build
```

## Useful scripts

From `package.json`:

- `npm run dev`: start Vite frontend
- `npm run dev:lean`: run Tauri dev with temporary cache locations and auto-clean heavy artifacts on exit
- `npm run build`: type-check and build frontend assets
- `npm run preview`: preview built frontend
- `npm run tauri`: run Tauri CLI
- `npm run clean:heavy`: remove heavy build artifacts only (`dist`, `src-tauri/target`, `src-tauri/gen`, Vite caches, and TS build info)
- `npm run clean:full`: remove all reproducible local caches (includes `node_modules` plus heavy artifacts)
- `npm run clean`: alias for `npm run clean:full`

## Project layout

- `src/`: React UI
- `src-tauri/src/`: Rust backend commands and services
- `src-tauri/migrations/`: SQLite migrations
- `src-tauri/icons/`: app icons used for bundles

## Notes

- Secrets and tokens should be handled through the app settings, not hardcoded in files.
- LLM generation depends on a locally available Ollama model.
