//! NEXUS — browser automation (Chromium CDP).

pub mod manager;
pub mod page;

pub use manager::BrowserManager;
pub use page::{execute, PageAction, PageResult};
