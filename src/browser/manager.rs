//! Browser manager — singleton Chromium instance.
//!
//! Lazy start: browser chỉ spawn khi tool đầu tiên gọi `ensure_started()`.
//! Toàn bộ app có 1 browser duy nhất (singleton qua `OnceCell`).

use std::sync::Arc;

use chromiumoxide::browser::{Browser as ChromeBrowser, BrowserConfig};
use chromiumoxide::page::Page;
use parking_lot::Mutex;
use tokio::sync::OnceCell;
use tracing::info;

use crate::error::{BrowserError, Result};

/// Singleton browser manager.
pub struct BrowserManager {
    inner: OnceCell<Arc<BrowserInner>>,
    headless: bool,
    port: u16,
}

struct BrowserInner {
    browser: ChromeBrowser,
    page: Mutex<Option<Page>>,
}

impl BrowserManager {
    #[must_use]
    pub fn new(headless: bool, port: u16) -> Self {
        Self {
            inner: OnceCell::new(),
            headless,
            port,
        }
    }

    /// Ensure browser đã start. Idempotent.
    pub async fn ensure_started(&self) -> Result<()> {
        self.inner.get_or_try_init(|| async {
            let mut cfg = BrowserConfig::builder()
                .arg("--disable-gpu")
                .arg("--disable-dev-shm-usage")
                .arg("--no-sandbox")
                .arg("--disable-extensions");

            if self.headless {
                cfg = cfg.arg("--headless=new");
            }

            let cfg = match cfg.window_size(1280, 800).build() {
                Ok(c) => c,
                Err(e) => {
                    return Err(BrowserError::CdpConnection(format!("browser config: {e}")).into());
                }
            };

            let (browser, mut handler) = match ChromeBrowser::launch(cfg).await {
                Ok(b) => b,
                Err(e) => {
                    if e.to_string().contains("not found") || e.to_string().contains("spawn") {
                        return Err(BrowserError::ChromiumNotFound.into());
                    }
                    return Err(BrowserError::CdpConnection(e.to_string()).into());
                }
            };

            // Spawn handler task để drive CDP event loop
            tokio::spawn(async move {
                while let Some(_) = handler.next().await {}
            });

            info!("Chromium started (headless={})", self.headless);
            Ok(Arc::new(BrowserInner {
                browser,
                page: Mutex::new(None),
            }))
        })
        .await
        .map(|_| ())
    }

    /// Lấy page hiện tại, hoặc tạo mới nếu chưa có.
    pub async fn page(&self) -> Result<Page> {
        self.ensure_started().await?;
        let inner = self
            .inner
            .get()
            .ok_or_else(|| BrowserError::CdpConnection("browser not initialized".into()))?;

        // Try fast path: clone existing page handle
        {
            let guard = inner.page.lock();
            if let Some(p) = &*guard {
                return Ok(p.clone());
            }
        }

        // Slow path: create new page
        let page = inner
            .browser
            .new_page("about:blank")
            .await
            .map_err(|e| BrowserError::CdpConnection(e.to_string()))?;

        let mut guard = inner.page.lock();
        *guard = Some(page.clone());
        Ok(page)
    }

    /// Shutdown browser (đóng process Chromium).
    pub async fn shutdown(&self) -> Result<()> {
        if let Some(inner) = self.inner.get() {
            let _ = inner.browser.close().await;
            let mut guard = inner.page.lock();
            *guard = None;
        }
        Ok(())
    }
}
