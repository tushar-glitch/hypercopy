//! REST /info client + meta (coin -> asset index) cache.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::RwLock;

pub struct InfoClient {
    base: String,
    http: reqwest::Client,
    meta_cache: RwLock<Option<HashMap<String, u32>>>,
}

impl InfoClient {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            http: reqwest::Client::new(),
            meta_cache: RwLock::new(None),
        }
    }

    pub async fn clearinghouse_state(&self, user: &str) -> Result<Value> {
        self.post(json!({ "type": "clearinghouseState", "user": user })).await
    }

    /// Returns `universe` from `meta`: ordered list of perp assets.
    /// Asset index `a` for orders is the position in this list.
    pub async fn refresh_meta(&self) -> Result<()> {
        let v = self.post(json!({ "type": "meta" })).await?;
        let universe = v.get("universe").and_then(|u| u.as_array())
            .ok_or_else(|| anyhow!("meta response missing universe"))?;
        let mut map = HashMap::with_capacity(universe.len());
        for (idx, item) in universe.iter().enumerate() {
            if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                map.insert(name.to_string(), idx as u32);
            }
        }
        *self.meta_cache.write().unwrap() = Some(map);
        Ok(())
    }

    pub fn asset_index(&self, coin: &str) -> Option<u32> {
        self.meta_cache.read().unwrap().as_ref()
            .and_then(|m| m.get(coin).copied())
    }

    async fn post(&self, body: Value) -> Result<Value> {
        let url = format!("{}/info", self.base);
        let resp = self.http.post(url).json(&body).send().await.context("info POST")?;
        let v: Value = resp.json().await.context("info JSON")?;
        Ok(v)
    }
}
