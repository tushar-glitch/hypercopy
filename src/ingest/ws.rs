//! Hyperliquid WebSocket subscriber.
//!
//! Subscribes to `userFills` for a target wallet, parses each fill,
//! and forwards normalized `FillEvent`s onto the decision channel.
//!
//! Reconnects with exponential backoff. After reconnect, the snapshot
//! frame is marked `is_snapshot = true` so the decision engine can
//! avoid re-mirroring historical trades.

use crate::config::Target;
use crate::types::{FillEvent, Side};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn run(ws_url: String, target: Target, tx: Sender<FillEvent>) -> Result<()> {
    let mut backoff_ms: u64 = 500;
    loop {
        match run_once(&ws_url, &target, &tx).await {
            Ok(_) => {
                tracing::warn!(label = %target.label, "ws stream closed cleanly, reconnecting");
            }
            Err(e) => {
                tracing::error!(label = %target.label, ?e, "ws error, reconnecting");
            }
        }
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        backoff_ms = (backoff_ms * 2).min(30_000);
    }
}

async fn run_once(ws_url: &str, target: &Target, tx: &Sender<FillEvent>) -> Result<()> {
    let (mut ws, _) = connect_async(ws_url).await.context("ws connect")?;
    let sub = json!({
        "method": "subscribe",
        "subscription": { "type": "userFills", "user": target.address }
    });
    ws.send(Message::Text(sub.to_string())).await?;
    tracing::info!(label = %target.label, addr = %target.address, "subscribed to userFills");

    let mut got_snapshot = false;
    while let Some(msg) = ws.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Ping(p) => { ws.send(Message::Pong(p)).await.ok(); continue; }
            Message::Close(_) => break,
            _ => continue,
        };

        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // HL frame shape: { "channel": "userFills", "data": { "isSnapshot": bool, "fills": [...] } }
        let channel = v.get("channel").and_then(|c| c.as_str()).unwrap_or("");
        if channel != "userFills" { continue; }

        let data = match v.get("data") { Some(d) => d, None => continue };
        let is_snapshot = data.get("isSnapshot").and_then(|b| b.as_bool()).unwrap_or(false);
        if is_snapshot && got_snapshot { continue; }
        got_snapshot = got_snapshot || is_snapshot;

        let fills = match data.get("fills").and_then(|f| f.as_array()) {
            Some(a) => a,
            None => continue,
        };

        for f in fills {
            if let Some(event) = parse_fill(target, f, is_snapshot) {
                if tx.send(event).await.is_err() { return Ok(()); }
            }
        }
    }
    Ok(())
}

fn parse_fill(target: &Target, f: &Value, is_snapshot: bool) -> Option<FillEvent> {
    let coin = f.get("coin")?.as_str()?.to_string();
    let side_s = f.get("side")?.as_str()?;
    let side = match side_s { "B" => Side::Buy, "A" => Side::Sell, _ => return None };
    let px: f64 = f.get("px")?.as_str()?.parse().ok()?;
    let sz: f64 = f.get("sz")?.as_str()?.parse().ok()?;
    let time_ms = f.get("time")?.as_u64()?;
    // HL's `dir` field describes intent: "Open Long" / "Close Long" / "Open Short" / "Close Short".
    let dir = f.get("dir").and_then(|d| d.as_str()).unwrap_or("");
    let is_close = dir.starts_with("Close");

    Some(FillEvent {
        target_address: target.address.clone(),
        target_label: target.label.clone(),
        coin,
        side,
        px,
        sz,
        time_ms,
        is_close,
        is_snapshot,
    })
}
