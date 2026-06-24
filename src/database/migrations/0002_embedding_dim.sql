-- 0002_embedding_dim.sql — Hỗ trợ cả OpenAI (1536) và Ollama nomic-embed-text (768)
-- Cột embedding_dim đã có ở 0001_init.sql với default 1536.
-- Migration này chỉ thêm metadata table để track schema version riêng của NEXUS.

CREATE TABLE IF NOT EXISTS schema_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_meta (key, value) VALUES ('embedding_provider', 'openai');
INSERT OR IGNORE INTO schema_meta (key, value) VALUES ('embedding_dim_default', '1536');
INSERT OR IGNORE INTO schema_meta (key, value) VALUES ('app_version', '0.1.0');
