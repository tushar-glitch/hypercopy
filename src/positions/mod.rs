//! In-memory mirror-state tracker.
//!
//! v0 keeps everything in process. Persist to sqlite/postgres once you
//! care about surviving restarts without re-reading clearinghouseState.

use crate::types::{ExecReceipt, OrderIntent, Side};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OpenMirror {
    pub target: String,
    pub coin: String,
    pub side: Side,
    pub sz: f64,
    pub entry_px: f64,
    pub oid: Option<u64>,
}

pub struct Manager {
    by_target_coin: HashMap<(String, String), OpenMirror>,
}

impl Manager {
    pub fn new() -> Self { Self { by_target_coin: HashMap::new() } }

    pub fn open_count(&self) -> usize { self.by_target_coin.len() }

    pub fn find_open(&self, target: &str, coin: &str) -> Option<&OpenMirror> {
        self.by_target_coin.get(&(target.to_string(), coin.to_string()))
    }

    pub fn record_open(&mut self, intent: &OrderIntent, receipt: &ExecReceipt) {
        let key = (intent.source_target.clone(), intent.coin.clone());
        self.by_target_coin.insert(key, OpenMirror {
            target: intent.source_target.clone(),
            coin: intent.coin.clone(),
            side: intent.side,
            sz: receipt.filled_sz.unwrap_or(intent.sz),
            entry_px: receipt.avg_px.unwrap_or(intent.px),
            oid: receipt.oid,
        });
    }

    pub fn record_close(&mut self, intent: &OrderIntent, _receipt: &ExecReceipt) {
        let key = (intent.source_target.clone(), intent.coin.clone());
        self.by_target_coin.remove(&key);
    }
}
