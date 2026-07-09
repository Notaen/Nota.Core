use std::fs::create_dir_all;

use anyhow::Result;
use chrono::Local;
use nota_core::{base_dir, config, connect, persona, session};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::Writer, time::FormatTime},
    prelude::*,
};

#[derive(Clone)]
struct ChronoLocalTimer;

impl FormatTime for ChronoLocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    }
}

fn ensure_dir() -> Result<()> {
    create_dir_all(base_dir())?;
    create_dir_all(base_dir().join(".logs"))?;
    create_dir_all(base_dir().join("personas"))?;
    create_dir_all(base_dir().join("sessions"))?;
    create_dir_all(base_dir().join("sessions").join("archive"))?;

    Ok(())
}

fn init_tracing() -> Result<non_blocking::WorkerGuard> {
    let timer = ChronoLocalTimer;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_suffix("log")
        .max_log_files(30)
        .build(base_dir().join(".logs"))?;

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
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    ensure_dir()?;
    let _guard = init_tracing()?;
    config::load()?;

    let cancel_token = CancellationToken::new();

    info!("Nota.Core started");
    persona::init().await?;
    session::manager::load().await?;
    let _ = tokio::spawn(connect::serve(cancel_token.clone()));

    cancel_token.cancelled().await;
    info!("Nota.Core is shutting down");

    Ok(())
}
