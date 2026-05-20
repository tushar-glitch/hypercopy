pub mod signer;
pub mod exchange;

use crate::config::Config;
use crate::ingest::info::InfoClient;
use crate::types::{ExecReceipt, OrderIntent};
use anyhow::Result;
use std::sync::Arc;

pub struct Executor {
    inner: exchange::ExchangeClient,
}

impl Executor {
    pub fn new(cfg: Config, info: Arc<InfoClient>) -> Result<Self> {
        let inner = exchange::ExchangeClient::new(cfg, info)?;
        Ok(Self { inner })
    }

    pub async fn submit(&self, intent: &OrderIntent) -> Result<ExecReceipt> {
        self.inner.place_order(intent).await
    }
}
