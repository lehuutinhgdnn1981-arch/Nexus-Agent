//! Config store — load/save `config.toml`.

use std::path::{Path, PathBuf};

use crate::error::{ConfigError, Result};

use super::AppConfig;

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    /// Load config từ file. Nếu file không tồn tại, tạo config mặc định + ghi file.
    pub fn load_or_init(&self) -> Result<AppConfig> {
        if !self.path.exists() {
            let default = AppConfig::defaults();
            self.save(&default)?;
            return Ok(default);
        }
        self.load()
    }

    /// Load config từ file (file phải tồn tại).
    pub fn load(&self) -> Result<AppConfig> {
        let content = std::fs::read_to_string(&self.path)?;
        let cfg: AppConfig = toml::from_str(&content).map_err(crate::error::ConfigError::from)?;
        Ok(cfg)
    }

    /// Save config vào file (atomic: ghi vào temp rồi rename).
    pub fn save(&self, cfg: &AppConfig) -> Result<()> {
        let content = toml::to_string_pretty(cfg).map_err(crate::error::ConfigError::from)?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| ConfigError::NotFound("config parent dir missing".into()))?;
        std::fs::create_dir_all(parent)?;

        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Patch config: load → apply fn → save.
    pub fn patch<F>(&self, f: F) -> Result<AppConfig>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut cfg = self.load_or_init()?;
        f(&mut cfg);
        self.save(&cfg)?;
        Ok(cfg)
    }

    /// Path của config file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_init_creates_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        let store = ConfigStore::new(&path);
        assert!(!path.exists());

        let cfg = store.load_or_init().unwrap();
        assert!(path.exists());
        assert_eq!(cfg.agent.max_iterations, 10);
    }

    #[test]
    fn patch_persists() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        let store = ConfigStore::new(&path);

        let cfg = store
            .patch(|c| {
                c.agent.max_iterations = 20;
            })
            .unwrap();
        assert_eq!(cfg.agent.max_iterations, 20);

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.agent.max_iterations, 20);
    }
}
