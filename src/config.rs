use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub hyperliquid: Hyperliquid,
    pub wallet: Wallet,
    pub risk: Risk,
    pub sizing: Sizing,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hyperliquid {
    pub api_url: String,
    pub ws_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Wallet {
    pub agent_address: String,
    pub main_address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Risk {
    pub max_concurrent_positions: usize,
    pub max_leverage: f64,
    pub max_notional_usd: f64,
    pub daily_drawdown_stop_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sizing {
    pub mode: String,
    pub fixed_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub address: String,
    pub label: String,
    pub weight: f64,
    pub enabled: bool,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.as_ref().display()))?;
        let cfg: Config = toml::from_str(&raw).context("parsing config.toml")?;
        Ok(cfg)
    }
}
