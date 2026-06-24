-- 0003_indexes.sql — Index phụ trợ cho performance

-- Full-text search (SQLite FTS5) cho memories.content
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    content,
    content='memories',
    content_rowid='rowid'
);

-- Trigger đồng bộ memories → memories_fts
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- Index cho tasks fire_at (cho query one_time jobs sắp tới)
CREATE INDEX IF NOT EXISTS idx_tasks_fire_at_enabled
    ON tasks(fire_at) WHERE enabled = 1 AND kind = 'one_time';
