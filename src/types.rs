use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

/// Fill event observed from a target wallet's userFills stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillEvent {
    pub target_address: String,
    pub target_label: String,
    pub coin: String,
    pub side: Side,
    pub px: f64,
    pub sz: f64,
    pub time_ms: u64,
    /// True if this fill reduces/closes the target's position.
    pub is_close: bool,
    /// True if this is part of the initial snapshot (not a live fill).
    pub is_snapshot: bool,
}

/// Mirror order we want to submit on our own account.
#[derive(Debug, Clone)]
pub struct OrderIntent {
    pub coin: String,
    pub asset_index: u32,
    pub side: Side,
    pub px: f64,
    pub sz: f64,
    pub reduce_only: bool,
    pub tif: TimeInForce,
    pub source_target: String,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeInForce {
    Ioc,
    Gtc,
    Alo,
}

#[derive(Debug, Clone)]
pub struct ExecReceipt {
    pub oid: Option<u64>,
    pub status: String,
    pub avg_px: Option<f64>,
    pub filled_sz: Option<f64>,
}
