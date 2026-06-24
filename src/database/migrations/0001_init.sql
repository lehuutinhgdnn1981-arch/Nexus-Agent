-- 0001_init.sql — Tạo 5 bảng chính của NEXUS
-- Tất cả timestamp lưu dạng INTEGER (Unix seconds).

-- -----------------------------------------------------------------------------
-- sessions: Phiên chat
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT PRIMARY KEY,
    title         TEXT NOT NULL,
    provider      TEXT NOT NULL DEFAULT 'openai',
    model         TEXT NOT NULL DEFAULT 'gpt-4o-mini',
    system_prompt TEXT,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_updated_at
    ON sessions(updated_at DESC);

-- -----------------------------------------------------------------------------
-- messages: Tin nhắn trong session
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS messages (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role         TEXT NOT NULL,                  -- 'user' | 'assistant' | 'system' | 'tool'
    content      TEXT NOT NULL DEFAULT '',
    tool_calls   TEXT,                           -- JSON array of tool calls (nullable)
    tool_results TEXT,                           -- JSON array of tool results (nullable)
    created_at   INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_session_created
    ON messages(session_id, created_at);

-- -----------------------------------------------------------------------------
-- memories: Long-term memory với embeddings
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS memories (
    id           TEXT PRIMARY KEY,
    content      TEXT NOT NULL,
    category     TEXT NOT NULL,                  -- 'fact' | 'preference' | 'task' | 'note'
    tags         TEXT NOT NULL DEFAULT '[]',     -- JSON array
    embedding    BLOB NOT NULL,                  -- f32 little-endian, 1536-dim
    embedding_dim INTEGER NOT NULL DEFAULT 1536,
    session_id   TEXT,                           -- nullable origin session
    created_at   INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    use_count    INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_memories_category
    ON memories(category);

CREATE INDEX IF NOT EXISTS idx_memories_last_used
    ON memories(last_used_at DESC);

-- -----------------------------------------------------------------------------
-- tasks: Scheduled jobs
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tasks (
    id             TEXT PRIMARY KEY,
    kind           TEXT NOT NULL,                -- 'one_time' | 'recurring'
    payload        TEXT NOT NULL,                -- JSON JobSpec
    cron           TEXT,                         -- nullable (one_time = NULL)
    fire_at        INTEGER,                      -- nullable (recurring = NULL)
    enabled        INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL,
    last_fired_at  INTEGER
);

CREATE INDEX IF NOT EXISTS idx_tasks_enabled
    ON tasks(enabled);

CREATE INDEX IF NOT EXISTS idx_tasks_fire_at
    ON tasks(fire_at);

-- -----------------------------------------------------------------------------
-- command_logs: Shell command execution log (audit trail)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS command_logs (
    id           TEXT PRIMARY KEY,
    session_id   TEXT,
    command      TEXT NOT NULL,
    args         TEXT NOT NULL DEFAULT '[]',     -- JSON array
    status       TEXT NOT NULL,                  -- 'approved' | 'rejected' | 'blacklisted' | 'executed' | 'timeout' | 'error'
    exit_code    INTEGER,
    stdout       TEXT,
    stderr       TEXT,
    started_at   INTEGER NOT NULL,
    finished_at  INTEGER
);

CREATE INDEX IF NOT EXISTS idx_command_logs_session_started
    ON command_logs(session_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_command_logs_status
    ON command_logs(status);
