//! SearchProvider trait.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Result;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>>;
}
