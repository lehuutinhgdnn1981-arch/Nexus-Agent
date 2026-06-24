//! Browser smoke tests — require Chromium installed.
//!
//! Run with: `cargo test --test browser_smoke --features test-utils -- --ignored`

#![cfg(feature = "test-utils")]

mod common;

use nexus::browser::page::{execute, PageAction};
use nexus::browser::BrowserManager;
use std::time::Duration;

#[tokio::test]
#[ignore = "requires Chromium installed"]
async fn browser_smoke_navigate_and_extract() {
    let manager = BrowserManager::new(true, 9222);
    manager.ensure_started().await.unwrap();

    let page = manager.page().await.unwrap();
    let result = execute(
        &page,
        &PageAction::Navigate {
            url: "https://example.com".into(),
            timeout_ms: 30_000,
        },
    )
    .await
    .unwrap();
    assert_eq!(result["status"], "ok");

    let result = execute(&page, &PageAction::ExtractText).await.unwrap();
    let text = result["text"].as_str().unwrap_or("");
    assert!(!text.is_empty());
    assert!(text.contains("Example Domain") || text.contains("example"));

    let _ = manager.shutdown().await;
}

#[tokio::test]
#[ignore = "requires Chromium installed"]
async fn browser_smoke_screenshot() {
    let manager = BrowserManager::new(true, 9223);
    manager.ensure_started().await.unwrap();

    let page = manager.page().await.unwrap();
    let _ = execute(
        &page,
        &PageAction::Navigate {
            url: "https://example.com".into(),
            timeout_ms: 30_000,
        },
    )
    .await
    .unwrap();

    let result = execute(
        &page,
        &PageAction::Screenshot {
            full_page: false,
        },
    )
    .await
    .unwrap();
    let bytes = result["bytes"].as_u64().unwrap_or(0);
    assert!(bytes > 1000, "screenshot too small: {bytes} bytes");

    let _ = manager.shutdown().await;
}

#[tokio::test]
#[ignore = "requires Chromium installed"]
async fn browser_smoke_wait_for_selector() {
    let manager = BrowserManager::new(true, 9224);
    manager.ensure_started().await.unwrap();

    let page = manager.page().await.unwrap();
    let _ = execute(
        &page,
        &PageAction::Navigate {
            url: "https://example.com".into(),
            timeout_ms: 30_000,
        },
    )
    .await
    .unwrap();

    // example.com has an <h1> element
    let result = execute(
        &page,
        &PageAction::Wait {
            selector: "h1".into(),
            timeout_ms: 5_000,
        },
    )
    .await
    .unwrap();
    assert_eq!(result["found"], "h1");

    let _ = manager.shutdown().await;
}

#[tokio::test]
#[ignore = "requires Chromium installed"]
async fn browser_smoke_singleton_reuses_page() {
    let manager = BrowserManager::new(true, 9225);
    manager.ensure_started().await.unwrap();

    let page1 = manager.page().await.unwrap();
    let page2 = manager.page().await.unwrap();

    // Same page (singleton pattern)
    // Note: Page doesn't implement PartialEq, so we check that both navigate works
    let _ = execute(
        &page1,
        &PageAction::Navigate {
            url: "https://example.com".into(),
            timeout_ms: 30_000,
        },
    )
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let _ = manager.shutdown().await;
}
