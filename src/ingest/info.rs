//! REST /info client. Used for:
//!   - cold-start snapshots of a target's open positions
//!   - reconciling our own state after WS reconnects
//!   - resolving coin symbol -> asset index via the `meta` query

use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct InfoClient {
    base: String,
    http: reqwest::Client,
}

impl InfoClient {
    pub fn new(base: impl Into<String>) -> Self {
        Self { base: base.into(), http: reqwest::Client::new() }
    }

    pub async fn clearinghouse_state(&self, user: &str) -> Result<Value> {
        self.post(json!({ "type": "clearinghouseState", "user": user })).await
    }

    pub async fn meta(&self) -> Result<Value> {
        self.post(json!({ "type": "meta" })).await
    }

    async fn post(&self, body: Value) -> Result<Value> {
        let url = format!("{}/info", self.base);
        let resp = self.http.post(url).json(&body).send().await.context("info POST")?;
        let v: Value = resp.json().await.context("info JSON")?;
        Ok(v)
    }
}
