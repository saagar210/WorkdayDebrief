-- Initial schema for WorkdayDebrief

CREATE TABLE IF NOT EXISTS daily_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    summary_date TEXT NOT NULL UNIQUE,  -- YYYY-MM-DD

    -- Aggregated data (JSON blobs)
    tickets_closed TEXT DEFAULT '[]',
    tickets_in_progress TEXT DEFAULT '[]',
    meetings TEXT DEFAULT '[]',
    focus_hours REAL DEFAULT 0.0,

    -- User-edited fields
    blockers TEXT DEFAULT '',
    tomorrow_priorities TEXT DEFAULT '',
    manual_notes TEXT DEFAULT '',

    -- Generated narrative
    narrative TEXT DEFAULT '',
    tone TEXT DEFAULT 'professional',

    -- Delivery tracking
    delivered_to TEXT DEFAULT '[]',

    -- Sources status snapshot
    sources_status TEXT DEFAULT '{}',

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Trigger for updated_at
CREATE TRIGGER IF NOT EXISTS update_daily_summaries_timestamp
    AFTER UPDATE ON daily_summaries
    FOR EACH ROW
BEGIN
    UPDATE daily_summaries SET updated_at = datetime('now') WHERE id = OLD.id;
END;

CREATE TABLE IF NOT EXISTS delivery_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    delivery_type TEXT NOT NULL UNIQUE,
    config TEXT NOT NULL DEFAULT '{}',
    is_enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    scheduled_time TEXT DEFAULT '17:00',
    default_tone TEXT DEFAULT 'professional',
    enable_llm INTEGER DEFAULT 1,
    llm_model TEXT DEFAULT 'qwen3:14b',
    llm_temperature REAL DEFAULT 0.7,
    llm_timeout_secs INTEGER DEFAULT 15,
    calendar_source TEXT DEFAULT 'none',
    retention_days INTEGER DEFAULT 90,
    jira_base_url TEXT,
    jira_project_key TEXT,
    toggl_workspace_id TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed default settings row
INSERT OR IGNORE INTO settings (id) VALUES (1);

CREATE INDEX IF NOT EXISTS idx_summaries_date ON daily_summaries(summary_date DESC);
