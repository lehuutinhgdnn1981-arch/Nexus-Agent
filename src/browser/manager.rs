use std::sync::Arc;
use chromiumoxide::browser::{Browser as ChromeBrowser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use parking_lot::Mutex;
use tokio::sync::OnceCell;
use tracing::info;
use crate::error::{BrowserError, Result};

#[allow(dead_code)]
pub struct BrowserManager { inner: OnceCell<Arc<BrowserInner>>, headless: bool, port: u16 }
struct BrowserInner { browser: ChromeBrowser, page: Mutex<Option<Page>> }

impl BrowserManager {
    pub fn new(headless: bool, port: u16) -> Self { Self { inner: OnceCell::new(), headless, port } }
    pub async fn ensure_started(&self) -> Result<()> {
        self.inner.get_or_try_init(|| async {
            let mut cfg = BrowserConfig::builder().arg("--disable-gpu").arg("--disable-dev-shm-usage").arg("--no-sandbox").arg("--disable-extensions");
            if self.headless { cfg = cfg.arg("--headless=new"); }
            let cfg = cfg.window_size(1280, 800).build().map_err(|e| BrowserError::CdpConnection(format!("config: {e}")))?;
            let (browser, mut handler) = ChromeBrowser::launch(cfg).await.map_err(|e| {
                if e.to_string().contains("not found") || e.to_string().contains("spawn") { BrowserError::ChromiumNotFound } else { BrowserError::CdpConnection(e.to_string()) }
            })?;
            tokio::spawn(async move { while handler.next().await.is_some() {} });
            info!("Chromium started");
            Ok(Arc::new(BrowserInner { browser, page: Mutex::new(None) }))
        }).await.map(|_| ())
    }
    pub async fn page(&self) -> Result<Page> {
        self.ensure_started().await?;
        let inner = self.inner.get().ok_or_else(|| BrowserError::CdpConnection("not init".into()))?;
        { let g = inner.page.lock(); if let Some(p) = &*g { return Ok(p.clone()); } }
        let page = inner.browser.new_page("about:blank").await.map_err(|e| BrowserError::CdpConnection(e.to_string()))?;
        *inner.page.lock() = Some(page.clone());
        Ok(page)
    }
    pub async fn shutdown(&self) -> Result<()> { if let Some(inner) = self.inner.get() { *inner.page.lock() = None; } Ok(()) }
}
