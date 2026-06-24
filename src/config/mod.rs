//! NEXUS — configuration layer.

pub mod app_config;
pub mod paths;
pub mod provider_config;
pub mod store;

pub use app_config::AppConfig;
pub use paths::{config_path, db_path, ensure_workspace, log_dir, workspace_root};
pub use provider_config::ProviderConfig;
pub use store::ConfigStore;
