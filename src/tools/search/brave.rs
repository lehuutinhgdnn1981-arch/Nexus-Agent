//! Brave Search — REST API.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::error::{LlmError, Result};
use crate::tools::search::provider::{SearchProvider, SearchResult};

const ENDPOINT: &str = "https://api.search.brave.com/res/v1/web/search";

pub struct BraveSearch {
    client: Client,
    api_key: String,
}

impl BraveSearch {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_key,
        }
    }

    pub fn from_config(api_key: Option<&str>) -> Option<Self> {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("BRAVE_API_KEY").ok())?;
        if key.is_empty() {
            None
        } else {
            Some(Self::new(key))
        }
    }
}

#[derive(Deserialize)]
struct BraveResponse {
    web: Option<BraveWeb>,
}

#[derive(Deserialize)]
struct BraveWeb {
    results: Option<Vec<BraveResult>>,
}

#[derive(Deserialize)]
struct BraveResult {
    title: Option<String>,
    url: Option<String>,
    description: Option<String>,
}

#[async_trait]
impl SearchProvider for BraveSearch {
    fn id(&self) -> &'static str { "brave" }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>> {
        let resp = self
            .client
            .get(ENDPOINT)
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&[("q", query), ("count", &limit.to_string())])
            .send()
            .await
            .map_err(LlmError::Http)?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: "brave".into(),
                status: status.as_u16(),
                body,
            }
            .into());
        }

        let parsed: BraveResponse = resp.json().await.map_err(LlmError::Http)?;
        let results = parsed
            .web
            .and_then(|w| w.results)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                Some(SearchResult {
                    title: r.title?,
                    url: r.url?,
                    snippet: r.description.unwrap_or_default(),
                })
            })
            .take(limit as usize)
            .collect();

        Ok(results)
    }
}
