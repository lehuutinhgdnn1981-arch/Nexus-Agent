//! DuckDuckGo search — HTML scrape (no API key required).

use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use tracing::warn;

use crate::error::{LlmError, Result};
use crate::tools::search::provider::{SearchProvider, SearchResult};

const ENDPOINT: &str = "https://html.duckduckgo.com/html/";

pub struct DuckDuckGoSearch {
    client: Client,
}

impl DuckDuckGoSearch {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (compatible; NEXUS/0.1)")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

impl Default for DuckDuckGoSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchProvider for DuckDuckGoSearch {
    fn id(&self) -> &'static str { "duckduckgo" }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
        let resp = self
            .client
            .post(ENDPOINT)
            .form(&[("q", query)])
            .send()
            .await
            .map_err(LlmError::Http)?;

        let html = resp.text().await.map_err(LlmError::Http)?;

        // Parse result blocks. DuckDuckGo HTML có dạng:
        // <a class="result__a" href="...">Title</a>
        // <a class="result__snippet" href="...">Snippet</a>
        let title_re = Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();
        let snippet_re = Regex::new(r#"<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#).unwrap();
        let tag_re = Regex::new(r"<[^>]+>").unwrap();
        let entity_re = Regex::new(r"&[a-z]+;").unwrap();

        let mut results = Vec::new();
        for cap in title_re.captures_iter(&html) {
            let url_raw = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let title_raw = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let title = clean_text(&tag_re.replace_all(title_raw, ""), &entity_re);
            let url = decode_ddg_url(url_raw);

            // Find snippet gần nhất
            let snippet = snippet_re
                .captures_iter(&html)
                .next()
                .and_then(|sc| sc.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_default();
            let snippet = clean_text(&tag_re.replace_all(&snippet, ""), &entity_re);

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResult { title, url, snippet });
                if results.len() >= limit as usize {
                    break;
                }
            }
        }

        if results.is_empty() {
            warn!(query, "duckduckgo returned no results (HTML structure may have changed)");
        }
        Ok(results)
    }
}

fn clean_text(s: &str, entity_re: &Regex) -> String {
    let s = s.trim();
    let s = entity_re.replace_all(s, "");
    s.to_string()
}

/// DuckDuckGo redirect URL: `//duckduckgo.com/l/?uddg=<encoded>&rut=...`
fn decode_ddg_url(raw: &str) -> String {
    if let Some(idx) = raw.find("uddg=") {
        let after = &raw[idx + 5..];
        let end = after.find('&').unwrap_or(after.len());
        let encoded = &after[..end];
        if let Ok(decoded) = urlencoding::decode(encoded) {
            return decoded.to_string();
        }
    }
    raw.to_string()
}

mod urlencoding {
    pub fn decode(s: &str) -> Result<String, ()> {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                let h1 = chars.next().ok_or(())?;
                let h2 = chars.next().ok_or(())?;
                let byte = u8::from_str_radix(&format!("{h1}{h2}"), 16).map_err(|_| ())?;
                out.push(byte as char);
            } else if c == '+' {
                out.push(' ');
            } else {
                out.push(c);
            }
        }
        Ok(out)
    }
}
