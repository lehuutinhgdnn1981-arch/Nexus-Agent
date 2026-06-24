use std::time::Duration;
use chromiumoxide::page::Page;
use serde::{Deserialize, Serialize};
use crate::error::{BrowserError, Result};

pub type PageResult<T> = Result<T>;

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

pub async fn execute(page: &Page, action: &PageAction) -> PageResult<serde_json::Value> {
    match action {
        PageAction::Navigate { url, .. } => {
            page.goto(url).await.map_err(|e| BrowserError::Navigation(e.to_string()))?;
            Ok(serde_json::json!({"url": url, "status": "ok"}))
        }
        PageAction::Click { selector, .. } => {
            page.find_element(selector).await.map_err(|e| BrowserError::ElementNotFound(format!("{selector}: {e}")))?.click().await.map_err(|e| BrowserError::Navigation(format!("click: {e}")))?;
            Ok(serde_json::json!({"clicked": selector}))
        }
        PageAction::Type { selector, text, delay_ms } => {
            // Escape special chars in selector and text for safe JS injection
            let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
            let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n").replace('\r', "\\r");
            let js = format!("(function(){{var el=document.querySelector('{}');if(el){{el.value+='{}';el.dispatchEvent(new Event('input',{{bubbles:true}}));}}}})()", escaped_selector, escaped_text);
            let _ = page.evaluate(js.as_str()).await;
            if *delay_ms > 0 { tokio::time::sleep(Duration::from_millis(*delay_ms)).await; }
            Ok(serde_json::json!({"typed": text.len()}))
        }
        PageAction::Wait { selector, timeout_ms } => {
            let deadline = tokio::time::Instant::now() + Duration::from_millis(*timeout_ms);
            loop {
                if tokio::time::Instant::now() >= deadline { return Err(BrowserError::Timeout(Duration::from_millis(*timeout_ms)).into()); }
                if page.find_element(selector).await.is_ok() { return Ok(serde_json::json!({"found": selector})); }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        PageAction::ExtractText => {
            let html = page.content().await.map_err(|e| BrowserError::Navigation(format!("content: {e}")))?;
            let text: String = html.split_whitespace().collect::<Vec<_>>().join(" ");
            Ok(serde_json::json!({"text": text, "length": text.len()}))
        }
        PageAction::Screenshot { full_page } => {
            use chromiumoxide::page::ScreenshotParams;
            let params = ScreenshotParams::builder().full_page(*full_page).build();
            let bytes = page.screenshot(params).await.map_err(|e| BrowserError::Navigation(format!("screenshot: {e}")))?;
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Ok(serde_json::json!({"screenshot_base64": b64, "bytes": bytes.len()}))
        }
    }
}
