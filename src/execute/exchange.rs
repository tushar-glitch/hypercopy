//! POST /exchange — order placement.
//!
//! Wire payload uses compact 1-letter keys (a/b/p/s/r/t) to keep the
//! signed msgpack message small. See HL docs `api/exchange-endpoint`.

use crate::config::Config;
use crate::execute::signer::AgentSigner;
use crate::ingest::info::InfoClient;
use crate::types::{ExecReceipt, OrderIntent, Side, TimeInForce};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct ExchangeClient {
    base: String,
    http: reqwest::Client,
    signer: AgentSigner,
    is_mainnet: bool,
    info: Arc<InfoClient>,
}

impl ExchangeClient {
    pub fn new(cfg: Config, info: Arc<InfoClient>) -> Result<Self> {
        Ok(Self {
            is_mainnet: cfg.hyperliquid.is_mainnet(),
            base: cfg.hyperliquid.api_url,
            http: reqwest::Client::new(),
            signer: AgentSigner::from_env()?,
            info,
        })
    }

    pub async fn place_order(&self, intent: &OrderIntent) -> Result<ExecReceipt> {
        let asset_index = self.info.asset_index(&intent.coin)
            .ok_or_else(|| anyhow!("unknown coin: {} (meta cache miss)", intent.coin))?;

        let action = json!({
            "type": "order",
            "orders": [{
                "a": asset_index,
                "b": matches!(intent.side, Side::Buy),
                "p": format_px(intent.px),
                "s": format_sz(intent.sz),
                "r": intent.reduce_only,
                "t": tif_to_json(intent.tif),
            }],
            "grouping": "na"
        });

        let nonce = now_ms();
        let sig = self.signer.sign_l1_action(&action, nonce, None, self.is_mainnet)?;

        let body = json!({
            "action": action,
            "nonce": nonce,
            "signature": {
                "r": format!("0x{:x}", sig.r),
                "s": format!("0x{:x}", sig.s),
                "v": sig.v,
            }
        });

        let url = format!("{}/exchange", self.base);
        let resp = self.http.post(url).json(&body).send().await.context("exchange POST")?;
        let v: Value = resp.json().await.context("exchange JSON")?;
        tracing::debug!(?v, "exchange raw response");
        parse_receipt(&v)
    }
}

fn tif_to_json(t: TimeInForce) -> Value {
    let s = match t {
        TimeInForce::Ioc => "Ioc",
        TimeInForce::Gtc => "Gtc",
        TimeInForce::Alo => "Alo",
    };
    json!({ "limit": { "tif": s } })
}

fn format_px(px: f64) -> String { format!("{}", px) }
fn format_sz(sz: f64) -> String { format!("{}", sz) }

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

fn parse_receipt(v: &Value) -> Result<ExecReceipt> {
    let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("unknown").to_string();
    let st = v.pointer("/response/data/statuses/0");
    let (oid, avg_px, filled_sz) = match st {
        Some(s) if s.get("filled").is_some() => {
            let f = &s["filled"];
            (f.get("oid").and_then(|o| o.as_u64()),
             f.get("avgPx").and_then(|p| p.as_str()).and_then(|p| p.parse().ok()),
             f.get("totalSz").and_then(|p| p.as_str()).and_then(|p| p.parse().ok()))
        }
        Some(s) if s.get("resting").is_some() => {
            (s["resting"].get("oid").and_then(|o| o.as_u64()), None, None)
        }
        _ => (None, None, None),
    };
    Ok(ExecReceipt { oid, status, avg_px, filled_sz })
}
