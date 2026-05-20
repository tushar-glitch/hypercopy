pub mod signer;
pub mod exchange;

use crate::config::Config;
use crate::types::{ExecReceipt, OrderIntent};
use anyhow::Result;

pub struct Executor {
    inner: exchange::ExchangeClient,
}

impl Executor {
    pub fn new(cfg: Config) -> Result<Self> {
        let inner = exchange::ExchangeClient::new(cfg)?;
        Ok(Self { inner })
    }

    pub async fn submit(&self, intent: &OrderIntent) -> Result<ExecReceipt> {
        self.inner.place_order(intent).await
    }
}
