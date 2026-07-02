-- session_meta 表
CREATE TABLE IF NOT EXISTS session_meta (
    session_id TEXT NOT NULL,
    creator TEXT NOT NULL,
    create_time INTEGER NOT NULL,
    archive_at INTEGER
);

-- messages 表
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    content TEXT NOT NULL,
    role TEXT NOT NULL,
    tag TEXT
);

-- 给 messages 的 timestamp 创建索引
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);