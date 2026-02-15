# WorkdayDebrief

> Turn scattered work data into coherent daily summaries, powered by local AI.

WorkdayDebrief is a desktop app that automatically aggregates your workday activity from Jira, Google Calendar, and Toggl, then uses a local LLM to generate narrative summaries of what you accomplished. Never write another status update from scratch.

## Why Use WorkdayDebrief?

**The Problem**: At the end of each day, you need to remember what you worked on, update stakeholders, or prepare for standups. Your work is scattered across:
- Jira tickets you closed or updated
- Calendar meetings you attended
- Time tracking entries in Toggl
- Mental context that never got written down

**The Solution**: WorkdayDebrief pulls all that data together, feeds it to a local LLM (via Ollama), and generates a coherent narrative summary. You get:
- 📝 **Automated daily summaries** - One-click generation from your actual work data
- 🤖 **AI-powered narratives** - Natural language summaries, not just bullet points
- 🔒 **Privacy-first** - All LLM processing happens locally (no data sent to cloud)
- 📧 **Flexible delivery** - Email yourself, post to Slack, or save as markdown
- 📊 **Historical tracking** - SQLite database keeps all your summaries searchable

## What You'd Use It For

- **Daily standup prep** - Generate a summary of yesterday's work in seconds
- **Status reports** - Email weekly summaries to your manager
- **Personal journaling** - Keep a searchable log of what you actually accomplished
- **Time tracking validation** - Compare Toggl entries against calendar and tickets
- **End-of-sprint reviews** - Look back at what you shipped
- **Performance reviews** - Search months of summaries to remember your wins

## How It Works

1. **Connect your tools** (one-time setup):
   - Jira (base URL + API token)
   - Google Calendar (OAuth2 flow)
   - Toggl (API token)
   - Optional: SMTP for email delivery, Slack webhook

2. **Configure your LLM**:
   - Install [Ollama](https://ollama.ai)
   - Pull a model (recommended: `qwen3:14b` for quality, `llama3.2` for speed)
   - Set model name, temperature, timeout in settings

3. **Generate summaries**:
   - Manual: Click "Generate Summary" for today
   - Scheduled: Configure automatic generation (e.g., 6pm daily)
   - On-demand: Regenerate narratives for past days

4. **Review and deliver**:
   - Edit narratives before sending (AI is a starting point)
   - Add tomorrow's priorities
   - Send via email, post to Slack, or export as markdown

## Features

### Data Aggregation
- ✅ **Jira**: Tickets closed, tickets in progress, blockers
- ✅ **Google Calendar**: Meetings attended (with OAuth2 PKCE flow)
- ✅ **Toggl**: Time entries grouped by project/task

### LLM Integration
- ✅ **Local Ollama support** - No cloud API keys required
- ✅ **Customizable prompts** - Adjust tone, detail level, formatting
- ✅ **Model selection** - Use any Ollama-compatible model
- ✅ **Regeneration** - Tweak settings and regenerate narratives

### Security
- ✅ **Age encryption** - Secrets encrypted with AES-256
- ✅ **OAuth2 PKCE** - Secure Google Calendar authorization
- ✅ **Local-first** - All data stored on your machine

### Delivery
- ✅ **Email** - SMTP delivery with customizable subject/format
- ✅ **Slack** - Webhook integration for team channels
- ✅ **File export** - Markdown format for archival

### Automation
- ✅ **Scheduled generation** - Set daily/weekly schedule
- ✅ **Auto-delivery** - Automatically email/Slack when summaries generate
- ✅ **Missed summary detection** - Catch up if app wasn't running

## Installation

### Prerequisites
- **macOS** (Intel or Apple Silicon) - _Windows/Linux support planned_
- **Ollama** installed and running (`brew install ollama`)
- **LLM model** downloaded (`ollama pull qwen3:14b`)

### Download & Run

#### Option 1: Download Release (Recommended)
1. Go to [Releases](https://github.com/samueladad75-byte/WorkdayDebrief/releases)
2. Download the latest `.dmg` file
3. Drag to Applications folder
4. Launch WorkdayDebrief

#### Option 2: Build from Source
```bash
# Clone the repo
git clone https://github.com/samueladad75-byte/WorkdayDebrief.git
cd WorkdayDebrief

# Install dependencies
npm install

# Set up environment variables (for Google OAuth)
cp .env.example .env
# Edit .env and add your Google OAuth credentials

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
# Find the .dmg in src-tauri/target/release/bundle/dmg/
```

### Repo Hygiene

Use these commands to remove local build/dependency bloat and rebuild from a clean state:

```bash
# Remove generated artifacts and local cache folders
npm run clean

# Reinstall dependencies exactly from lockfile
npm ci

# Verify TypeScript + production frontend build
npm run build

# (Optional) Run full desktop app in dev mode
npm run tauri dev
```

## Configuration

### First-Time Setup

1. **Launch the app** - Opens to Settings panel

2. **Connect Jira** (optional):
   - Base URL: `https://your-company.atlassian.net`
   - Email: Your Jira account email
   - API Token: [Create one here](https://id.atlassian.com/manage-profile/security/api-tokens)
   - Click "Test Connection" to verify

3. **Connect Google Calendar** (optional):
   - Click "Connect Google Calendar"
   - Authorize in browser (OAuth2 flow)
   - App stores encrypted refresh token

4. **Connect Toggl** (optional):
   - API Token: [Get from Toggl settings](https://track.toggl.com/profile)
   - Click "Test Connection" to verify

5. **Configure LLM**:
   - Model: `qwen3:14b` (or any Ollama model)
   - Temperature: `0.7` (lower = more factual, higher = more creative)
   - Timeout: `30` seconds

6. **Set up delivery** (optional):
   - **Email**: SMTP server, port, username, password
   - **Slack**: Webhook URL from Slack app settings
   - Toggle "Auto-deliver after generation"

7. **Schedule automation** (optional):
   - Enable scheduled generation
   - Set time (e.g., `18:00` for 6pm daily)
   - App generates summary automatically

### Google OAuth Setup (for Calendar)

If building from source, you need Google OAuth credentials:

1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Create a new project (or use existing)
3. Enable "Google Calendar API"
4. Create OAuth 2.0 credentials:
   - Application type: Desktop app
   - Authorized redirect URI: `http://localhost:8765/callback`
5. Copy Client ID and Client Secret to `.env`:
   ```
   GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
   GOOGLE_CLIENT_SECRET=your-client-secret
   ```

_Note: Releases include pre-configured OAuth credentials for convenience._

## Usage Guide

### Generate Your First Summary

1. **Go to "Today" panel** (opens by default)
2. Click **"Generate Summary"**
3. Wait 10-30 seconds (app fetches data + runs LLM)
4. Review the generated narrative
5. Click **"Edit"** to refine if needed
6. Add **"Tomorrow's Priorities"** (optional)
7. Click **"Send Summary"** to deliver

### Regenerate a Narrative

If the AI-generated summary isn't quite right:
1. Click **"Regenerate Narrative"**
2. App re-runs LLM with same data
3. New narrative appears (non-destructive - can regenerate again)

### View Past Summaries

1. Click **"History"** panel
2. Select date range or specific date
3. Click on a summary to view details
4. Regenerate, edit, or re-send from history

### Schedule Automatic Summaries

1. Go to **Settings → Automation**
2. Enable **"Scheduled Generation"**
3. Set time (e.g., `18:00` for 6pm)
4. Enable **"Auto-deliver after generation"** (optional)
5. App runs daily at scheduled time

### Export Summaries

From any summary view:
- Click **"Export"** → Choose format (Markdown, JSON)
- File saved to `~/Documents/WorkdayDebrief/exports/`

## Tech Stack

### Frontend
- **React 19** - UI framework
- **TypeScript** (strict mode) - Type safety
- **Vite** - Build tool
- **TanStack Query** - Data fetching/caching
- **Tailwind CSS 4** - Styling

### Backend
- **Tauri 2** - Desktop app framework
- **Rust** - Backend logic
- **SQLite** (via `sqlx`) - Local database
- **Age** - Secret encryption (AES-256)
- **OAuth2** (with PKCE) - Google Calendar auth
- **Lettre** - Email delivery
- **Reqwest** - HTTP client for APIs

### LLM
- **Ollama** - Local LLM runtime
- **Recommended models**:
  - `qwen3:14b` - Best quality (requires 16GB+ RAM)
  - `llama3.2` - Good balance (8GB RAM)
  - `phi4` - Fastest (4GB RAM)

## Troubleshooting

### "Failed to connect to Ollama"
- Make sure Ollama is running: `ollama serve`
- Check model is pulled: `ollama list`
- Verify port 11434 is accessible: `curl http://localhost:11434`

### "Google Calendar authorization failed"
- Check redirect URI is `http://localhost:8765/callback`
- Ensure port 8765 is not blocked by firewall
- Try re-authenticating (Settings → Disconnect → Reconnect)

### "Jira API token invalid"
- Verify token hasn't expired (check Atlassian settings)
- Ensure email matches Jira account email
- Check base URL format: `https://company.atlassian.net` (no trailing slash)

### "Summary generation takes too long"
- Try a smaller model (`llama3.2` instead of `qwen3:14b`)
- Increase timeout in Settings → LLM (e.g., 60 seconds)
- Check Ollama logs: `journalctl -u ollama` (Linux) or Console.app (macOS)

### "Email delivery failed"
- Test SMTP settings with "Test Connection" button
- Common ports: Gmail (587), Outlook (587), custom SMTP (check docs)
- Enable "Less secure app access" or use app-specific passwords

## Roadmap

- [ ] **Windows/Linux support** - Cross-platform builds
- [ ] **More integrations** - GitHub, GitLab, Linear, Asana
- [ ] **Team features** - Shared summaries, manager roll-ups
- [ ] **Advanced LLM options** - Multi-model comparison, custom prompts
- [ ] **Rich export formats** - PDF, HTML, Notion integration
- [ ] **Mobile companion** - View summaries on iOS/Android
- [ ] **Plugins** - Extensibility API for custom data sources

## Contributing

Contributions welcome! This is an early-stage project.

**Areas that need help**:
- Windows/Linux testing and builds
- Additional data source integrations
- UI/UX improvements
- Documentation and tutorials

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Privacy & Security

- **No telemetry** - App doesn't phone home
- **Local LLM** - All AI processing happens on your machine
- **Encrypted secrets** - API tokens stored with Age encryption
- **Open source** - Audit the code yourself

Your work data never leaves your machine unless you explicitly send summaries via email/Slack.

## Support

- **Issues**: [GitHub Issues](https://github.com/samueladad75-byte/WorkdayDebrief/issues)
- **Discussions**: [GitHub Discussions](https://github.com/samueladad75-byte/WorkdayDebrief/discussions)

---

Built with ❤️ for people who want to remember what they actually accomplished.
