//! Page actions — navigate, click, type, wait, extract, screenshot.

use std::time::Duration;

use chromiumoxide::page::Page;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::{BrowserError, Result};

/// Result type cho page action.
pub type PageResult<T> = Result<T>;

/// Action thực hiện trên page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PageAction {
    Navigate { url: String, timeout_ms: u64 },
    Click { selector: String, timeout_ms: u64 },
    Type { selector: String, text: String, delay_ms: u64 },
    Wait { selector: String, timeout_ms: u64 },
    ExtractText,
    Screenshot { full_page: bool },
}

/// Execute action trên page.
pub async fn execute(page: &Page, action: &PageAction) -> PageResult<serde_json::Value> {
    match action {
        PageAction::Navigate { url, timeout_ms } => {
            page.goto(url)
                .await
                .map_err(|e| BrowserError::Navigation(e.to_string()))?;
            page.wait_for_navigation()
                .await
                .map_err(|e| BrowserError::Navigation(e.to_string()))?;
            let _ = timeout_ms; // TODO: real timeout
            Ok(serde_json::json!({"url": url, "status": "ok"}))
        }
        PageAction::Click { selector, timeout_ms } => {
            let _ = timeout_ms;
            page.find_element(selector)
                .await
                .map_err(|e| BrowserError::ElementNotFound(format!("{selector}: {e}")))?
                .click()
                .await
                .map_err(|e| BrowserError::Navigation(format!("click failed: {e}")))?;
            Ok(serde_json::json!({"clicked": selector}))
        }
        PageAction::Type { selector, text, delay_ms } => {
            let el = page
                .find_element(selector)
                .await
                .map_err(|e| BrowserError::ElementNotFound(format!("{selector}: {e}")))?;
            if *delay_ms > 0 {
                el.click().await.ok();
            }
            for ch in text.chars() {
                el.send_keys(&ch.to_string())
                    .await
                    .map_err(|e| BrowserError::Navigation(format!("type failed: {e}")))?;
                if *delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*delay_ms)).await;
                }
            }
            Ok(serde_json::json!({"typed": text.len()}))
        }
        PageAction::Wait { selector, timeout_ms } => {
            let deadline = tokio::time::Instant::now() + Duration::from_millis(*timeout_ms);
            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(BrowserError::Timeout(Duration::from_millis(*timeout_ms)).into());
                }
                if page.find_element(selector).await.is_ok() {
                    return Ok(serde_json::json!({"found": selector}));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        PageAction::ExtractText => {
            let html = page
                .content()
                .await
                .map_err(|e| BrowserError::Navigation(format!("content failed: {e}")))?;
            // Strip HTML tags (naive — cho production nên dùng html2text crate)
            let text = strip_html(&html);
            Ok(serde_json::json!({"text": text, "length": text.len()}))
        }
        PageAction::Screenshot { full_page } => {
            let mut s = page.screenshot();
            if *full_page {
                s = s.fullpage(true);
            }
            let bytes = s
                .capture()
                .await
                .map_err(|e| BrowserError::Navigation(format!("screenshot failed: {e}")))?
                .to_vec();
            // Encode base64
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Ok(serde_json::json!({"screenshot_base64": b64, "bytes": bytes.len()}))
        }
    }
}

/// Strip HTML tags naively — cho production nên dùng dedicated parser.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Collapse whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
