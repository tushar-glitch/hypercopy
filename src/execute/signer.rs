//! EIP-712 signing for Hyperliquid /exchange actions.
//!
//! ⚠️ STUB — DO NOT TRUST FOR REAL TRADES YET.
//!
//! Hyperliquid uses a domain-specific EIP-712 scheme. Two flavors exist:
//!   1. "L1 action" signing (most order/cancel/modify actions): action hashed
//!      with msgpack-like canonical encoding, then signed with an EIP-712
//!      wrapper using domain `Exchange` on chain id 1337 (mainnet phantom).
//!   2. "User signed actions" (withdrawals, transfers, approveAgent): straight
//!      EIP-712 with real chain id 42161 (Arbitrum) and human-readable types.
//!
//! Reference impls to mirror:
//!   - Python:  hyperliquid-python-sdk/hyperliquid/utils/signing.py
//!   - TS:      @nktkas/hyperliquid  src/signing/
//!
//! Once you've confirmed alloy vs ethers-rs, fill `sign_l1_action` below.

use alloy_primitives::B256;
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;
use serde_json::Value;

pub struct AgentSigner {
    signer: PrivateKeySigner,
}

impl AgentSigner {
    pub fn from_env() -> Result<Self> {
        let key = std::env::var("HYPERCOPY_AGENT_KEY")
            .map_err(|_| anyhow::anyhow!("HYPERCOPY_AGENT_KEY not set"))?;
        let signer: PrivateKeySigner = key.parse()?;
        Ok(Self { signer })
    }

    pub fn address(&self) -> String {
        format!("{:?}", self.signer.address())
    }

    /// Sign an L1 action (order/cancel/modify).
    /// TODO: implement msgpack-like canonical encoding of `action`,
    ///       hash with nonce + vault_address, wrap in EIP-712 domain
    ///       { name: "Exchange", version: "1", chainId: 1337,
    ///         verifyingContract: 0x0000...0000 }, sign hash.
    pub fn sign_l1_action(
        &self,
        _action: &Value,
        _nonce: u64,
        _vault_address: Option<&str>,
    ) -> Result<SignatureRsv> {
        anyhow::bail!("sign_l1_action: not yet implemented — see module docstring")
    }
}

#[derive(Debug, Clone)]
pub struct SignatureRsv {
    pub r: B256,
    pub s: B256,
    pub v: u8,
}
