# Nota.Core

AI Agent 会话框架，SQLite 存储消息。

## 构建

```sh
cargo build
```

## 运行

```sh
cargo run
```

## 技术栈

Rust 2024 · Axum · SQLite (sqlx 0.9) · Tokio

## 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/session` | 会话列表 |
| POST | `/session` | 创建会话 |
| GET | `/session/{sid}/archive_at` | 获取归档时间 |
| POST | `/session/{sid}/archive_at` | 设置归档时间 |
| POST | `/admin/stop` | 优雅停机 |

## 目录

```
~/.nota/
├── personas/          # persona 工作区
├── sessions/          # 活跃会话数据库
│   └── archive/       # 已归档会话
├── .logs/             # 日志（30天轮转）
└── config.toml
```

## 文档

开发规范与 crudly 用法见 [`.agent/guide.md`](.agent/guide.md)。