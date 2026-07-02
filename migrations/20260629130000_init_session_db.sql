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

-- schedule 表：支持单次和循环任务
CREATE TABLE IF NOT EXISTS schedules (
    id INTEGER PRIMARY KEY,                     -- 自增主键
    message TEXT NOT NULL,                      -- 要推送的消息内容
    next_run_at INTEGER NOT NULL,               -- 下次执行的时间戳（Unix 秒）
    interval_seconds INTEGER,                   -- 循环间隔（秒），NULL 表示单次任务
    status TEXT NOT NULL DEFAULT 'active',      -- active, paused, completed
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- 关键索引：只查询 active 且到期的任务（利用部分索引）
CREATE INDEX IF NOT EXISTS idx_schedule_due 
ON schedule(next_run_at, status) 
WHERE status = 'active';

-- 给 messages 的 timestamp 创建索引
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);