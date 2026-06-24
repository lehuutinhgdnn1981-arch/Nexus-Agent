//! Filesystem paths cho NEXUS runtime data.
//!
//! Tất cả runtime data nằm ở `~/nexus_workspace/`:
//!   - `nexus.db`           — SQLite
//!   - `config.toml`        — AppConfig
//!   - `workspace/`         — sandbox root cho file tools
//!   - `logs/app.log`       — rolling daily log

use std::path::PathBuf;
use directories::ProjectDirs;
use crate::error::{ConfigError, Result};

const APP_QUALIFIER: &str = "ai";
const APP_ORGANIZATION: &str = "nexus";
const APP_APPLICATION: &str = "NEXUS";

/// Lấy base data dir theo platform convention.
/// - Linux: `~/.local/share/NEXUS` (hoặc `$XDG_DATA_HOME/NEXUS`)
/// - macOS: `~/Library/Application Support/ai.nexus.NEXUS`
/// - Windows: `C:\Users\<user>\AppData\Roaming\ai.nexus.NEXUS`
#[must_use]
pub fn data_dir() -> PathBuf {
    if let Some(p) = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_APPLICATION) {
        p.data_dir().to_path_buf()
    } else {
        // Fallback: `~/.nexus` nếu ProjectDirs không lấy được env
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".nexus")
    }
}

/// Workspace root: `<data_dir>/workspace/`. Đây là sandbox cho file tools.
#[must_use]
pub fn workspace_root() -> PathBuf {
    data_dir().join("workspace")
}

/// DB path: `<data_dir>/nexus.db`.
#[must_use]
pub fn db_path() -> PathBuf {
    data_dir().join("nexus.db")
}

/// Config path: `<data_dir>/config.toml`.
#[must_use]
pub fn config_path() -> PathBuf {
    data_dir().join("config.toml")
}

/// Log dir: `<data_dir>/logs/`.
#[must_use]
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Tạo toàn bộ thư mục runtime nếu chưa tồn tại.
/// Gọi 1 lần lúc app start.
pub fn ensure_workspace() -> Result<()> {
    let dirs = [
        data_dir(),
        workspace_root(),
        log_dir(),
    ];
    for d in &dirs {
        std::fs::create_dir_all(d)
            .map_err(|e| ConfigError::Io(e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_subdirs_of_data_dir() {
        let base = data_dir();
        assert!(workspace_root().starts_with(&base));
        assert!(db_path().starts_with(&base));
        assert!(config_path().starts_with(&base));
        assert!(log_dir().starts_with(&base));
    }
}
