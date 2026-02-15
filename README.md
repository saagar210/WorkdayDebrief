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

## Build

```bash
npm run build
npm run tauri build
```

## Useful scripts

From `package.json`:

- `npm run dev`: start Vite frontend
- `npm run build`: type-check and build frontend assets
- `npm run preview`: preview built frontend
- `npm run tauri`: run Tauri CLI
- `npm run clean`: remove generated local artifacts (`dist`, `node_modules`, `src-tauri/target`, and cache files)

## Project layout

- `src/`: React UI
- `src-tauri/src/`: Rust backend commands and services
- `src-tauri/migrations/`: SQLite migrations
- `src-tauri/icons/`: app icons used for bundles

## Notes

- Secrets and tokens should be handled through the app settings, not hardcoded in files.
- LLM generation depends on a locally available Ollama model.
