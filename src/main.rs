use std::fs::create_dir_all;

use anyhow::Result;
use nota_core::{base_dir, config};
use time::macros::format_description;
use tracing::{debug, info};
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, time::LocalTime},
    prelude::*,
};

fn ensure_dir() -> Result<()> {
    create_dir_all(base_dir())?;
    create_dir_all(base_dir().join("persona"))?;
    create_dir_all(base_dir().join("sessions"))?;
    Ok(())
}

fn init_tracing() -> Result<non_blocking::WorkerGuard> {
    // 本地时区：年-月-日 时:分:秒
    let time_fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let timer = LocalTime::new(time_fmt);

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
        .with_ansi(false) // 文件不要彩色转义码
        .with_filter(LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}

fn main() -> Result<()> {
    let _guard = init_tracing()?;
    ensure_dir()?;
    debug!("Dir ensured");
    config::load()?;
    info!("Config loaded");
    Ok(())
}
