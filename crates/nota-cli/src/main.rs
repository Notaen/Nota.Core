use std::fs::create_dir_all;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
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
    ConfigStore, FilePersonaStore, OpenAiLlm, SqliteSessionRepository, http_serve,
};

mod config_wizard;

#[derive(Parser)]
#[command(name = "nota", about = "AI agent session framework")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the interactive configuration wizard to set up or modify API settings.
    Onboard,
}

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
    LogTracer::init().ok();

    let timer = ChronoLocalTimer;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_suffix("log")
        .max_log_files(30)
        .build(base.join(".logs"))?;

    let (non_blocking_writer, guard) = non_blocking(file_appender);

    let console_layer = fmt::layer()
        .with_timer(timer.clone())
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_filter(LevelFilter::INFO);

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

fn load_or_create_config(store: &ConfigStore) -> Result<nota_infra::Config> {
    match store.load() {
        Ok(()) => Ok(store.get().unwrap()),
        Err(e) => {
            tracing::warn!("The config.toml doesn't exist or failed to load: {e}");
            let cfg = config_wizard::run_wizard(None)?;
            store.set(cfg.clone());
            store.save(&cfg)?;
            info!("Config saved");
            Ok(cfg)
        }
    }
}

async fn run_server(base: &Path, config: nota_infra::Config, cancel_token: CancellationToken) -> Result<()> {
    let repo = Arc::new(SqliteSessionRepository::new(base.to_path_buf()));
    let session_manager = Arc::new(SessionManager::new(repo));
    session_manager.load_all().await?;

    let persona_store = Arc::new(FilePersonaStore::new(base));
    let llm = Arc::new(OpenAiLlm::new(
        &config.api_url,
        &config.api_key,
        &config.model,
    ));
    let persona_manager = Arc::new(PersonaManager::new(
        persona_store,
        llm,
        session_manager.clone(),
    ));

    let handler: Arc<dyn SessionHandler> = persona_manager.clone();
    session_manager.register_handler_all(handler).await?;

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let base = dirs::home_dir().unwrap().join(".nota");
    ensure_dir(&base)?;
    let _guard = init_tracing(&base)?;

    let config_store = ConfigStore::new(&base);

    match cli.command {
        Some(Command::Onboard) => {
            let existing = config_store.load().ok().and_then(|_| config_store.get());
            let cfg = config_wizard::run_wizard(existing.as_ref())?;
            config_store.save(&cfg)?;
            info!("Configuration updated");
        }
        None => {
            info!("Nota started");
            let cancel_token = CancellationToken::new();
            let config = load_or_create_config(&config_store)?;
            run_server(&base, config, cancel_token).await?;
        }
    }

    Ok(())
}
