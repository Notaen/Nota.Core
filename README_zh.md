# Nota

AI Agent 会话框架，SQLite 存储消息。采用六边形架构（端口与适配器）。

## 构建

```sh
cargo build
```

## 运行

```sh
cargo run -p nota-cli
```

## 架构

项目为 Cargo 工作区，含三个 crate：

| Crate | 职责 |
|-------|------|
| `nota-core` | 领域实体、聚合与**端口 trait**。不含持久层/HTTP/IO 依赖——仅用 `log`（门面）、`serde`、`async-trait`、`chrono`、`anyhow`、`tokio`（sync）。 |
| `nota-infra` | **适配器**：SQLite 仓储（sqlx + crudly）、axum HTTP、文件 persona store、桩 LLM、TOML 配置。实现 `nota-core` 的端口。 |
| `nota-cli` | 二进制入口。初始化日志（`tracing` + `tracing-log` 桥接）、运行配置向导、把适配器注入 core、启动 HTTP 服务。 |

依赖方向严格单向：`nota-cli → nota-infra → nota-core`。core 不引用 sqlx、axum、crudly 或 `tracing`。

## 技术栈

Rust 2024 · Axum · SQLite (sqlx 0.9) · crudly 0.6 · Tokio

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
crates/
├── nota-core/    # 领域 + 端口
├── nota-infra/   # 适配器（sqlite / http / persona_store / llm / config）
└── nota-cli/     # 二进制：接线 + 日志
```

运行时数据位于用户主目录：

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
