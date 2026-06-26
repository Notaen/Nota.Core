use std::{path::PathBuf, sync::LazyLock};

pub mod config;

static BASE_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::home_dir().unwrap().join(".nota"));

pub fn base_dir() -> &'static PathBuf {
    &*BASE_DIR
}
