use std::fs::create_dir_all;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_log::LogTracer;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::Writer, time::FormatTime},
    prelude::*,
};

use nota_core::persona::PersonaManager;
use nota_core::session::{SessionHandler, SessionManager};
use nota_infra::{
    ConfigStore, FilePersonaStore, StubLlm, SqliteSessionRepository, http_serve,
};

mod config_wizard;

#[derive(Clone)]
struct ChronoLocalTimer;

impl FormatTime for ChronoLocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    }
}

fn ensure_dir(base: &Path) -> Result<()> {
    create_dir_all(base)?;
    create_dir_all(base.join(".logs"))?;
    create_dir_all(base.join("personas"))?;
    create_dir_all(base.join("sessions"))?;
    create_dir_all(base.join("sessions").join("archive"))?;
    Ok(())
}

fn init_tracing(base: &Path) -> Result<non_blocking::WorkerGuard> {
    // Bridge the `log` facade (used by nota-core/nota-infra) into tracing so a
    // single subscriber formats every record.
    LogTracer::init().ok();

    let timer = ChronoLocalTimer;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_suffix("log")
        .max_log_files(30)
        .build(base.join(".logs"))?;

    let (non_blocking_writer, guard) = non_blocking(file_appender);

    // 控制台层：彩色、精简字段、INFO级别
    let console_layer = fmt::layer()
        .with_timer(timer.clone())
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_filter(LevelFilter::INFO);

    // 文件层：关闭颜色、精简字段、DEBUG级别
    let file_layer = fmt::layer()
        .with_writer(non_blocking_writer)
        .with_timer(timer)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(false)
        .with_filter(LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .ok();

    Ok(guard)
}

fn load_config(store: &ConfigStore) -> Result<()> {
    match store.load() {
        Ok(()) => Ok(()),
        Err(e) => {
            tracing::warn!("The config.toml doesn't exist or failed to load: {e}");
            let cfg = config_wizard::interactive_config_init()?;
            store.set(cfg.clone());
            store.save(&cfg)?;
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let base = dirs::home_dir().unwrap().join(".nota");
    ensure_dir(&base)?;
    let _guard = init_tracing(&base)?;

    let config_store = ConfigStore::new(&base);
    load_config(&config_store)?;

    let cancel_token = CancellationToken::new();

    info!("Nota started");

    // --- Dependency wiring -------------------------------------------------
    let repo = Arc::new(SqliteSessionRepository::new(base.clone()));
    let session_manager = SessionManager::new(repo);
    session_manager.load_all().await?;

    let persona_store = Arc::new(FilePersonaStore::new(&base));
    let llm = Arc::new(StubLlm);
    let persona_manager = Arc::new(PersonaManager::new(persona_store, llm));

    // PersonaManager doubles as the default SessionHandler for every session.
    let handler: Arc<dyn SessionHandler> = persona_manager.clone();
    session_manager.register_handler_all(handler).await?;

    let session_manager = Arc::new(session_manager);

    // --- HTTP driving adapter ---------------------------------------------
    let addr: SocketAddr = "127.0.0.1:2349".parse()?;
    let listener = TcpListener::bind(addr).await?;
    tokio::spawn(http_serve(
        listener,
        session_manager.clone(),
        cancel_token.clone(),
    ));

    cancel_token.cancelled().await;
    info!("Nota is shutting down");

    Ok(())
}
