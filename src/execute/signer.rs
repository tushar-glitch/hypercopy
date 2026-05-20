//! EIP-712 signing for Hyperliquid L1 actions.
//!
//! HL scheme (mirrors hyperliquid-python-sdk's `signing.py`):
//!
//!   1. action_hash = keccak256(
//!          msgpack(action)
//!        ||  nonce.to_be_bytes(8)
//!        ||  vault_marker
//!      )
//!      where vault_marker is 0x00 if no vault, else 0x01 || vault_address_bytes.
//!
//!   2. Wrap in EIP-712 with:
//!        domain  = { name: "Exchange", version: "1",
//!                    chainId: 1337, verifyingContract: 0x0 }
//!        struct  = Agent { source: string, connectionId: bytes32 }
//!        source  = "a" on mainnet, "b" on testnet
//!        connectionId = action_hash
//!
//!   3. Sign the EIP-712 hash with the agent wallet's secp256k1 key.
//!      Submit { r, s, v } alongside the original action JSON.

use alloy_primitives::{keccak256, Address, B256};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{eip712_domain, sol, SolStruct};
use anyhow::{Context, Result};
use serde_json::Value;

sol! {
    struct Agent {
        string source;
        bytes32 connectionId;
    }
}

// approveAgent is hand-rolled (not via sol!) because HL's EIP-712 type name
// contains a colon: "HyperliquidTransaction:ApproveAgent", which the sol! macro
// can't express. Wrong type name → wrong typehash → wrong recovered address.

/// Real Arbitrum chain id for user-signed actions.
/// Mainnet: Arbitrum One (42161 = 0xa4b1). Testnet: Arbitrum Sepolia (421614 = 0x66eee).
pub fn user_signed_chain_id(is_mainnet: bool) -> u64 {
    if is_mainnet { 42161 } else { 421614 }
}

pub struct AgentSigner {
    signer: PrivateKeySigner,
}

impl AgentSigner {
    pub fn from_env() -> Result<Self> {
        let key = std::env::var("HYPERCOPY_AGENT_KEY")
            .context("HYPERCOPY_AGENT_KEY env var not set (expected 0x-prefixed hex private key)")?;
        let signer: PrivateKeySigner = key.parse().context("parsing HYPERCOPY_AGENT_KEY")?;
        Ok(Self { signer })
    }

    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// Sign an L1 action (order / cancel / modify / approveAgent for non-user-signed paths).
    /// `is_mainnet` controls the EIP-712 `source` byte: "a" for mainnet, "b" for testnet.
    /// `vault_address` is None for personal accounts.
    pub fn sign_l1_action(
        &self,
        action: &Value,
        nonce: u64,
        vault_address: Option<Address>,
        is_mainnet: bool,
    ) -> Result<SignatureRsv> {
        let action_hash = compute_action_hash(action, nonce, vault_address)?;

        let agent = Agent {
            source: (if is_mainnet { "a" } else { "b" }).to_string(),
            connectionId: action_hash.into(),
        };

        let domain = eip712_domain! {
            name: "Exchange",
            version: "1",
            chain_id: 1337u64,
            verifying_contract: Address::ZERO,
        };

        let signing_hash: B256 = agent.eip712_signing_hash(&domain);
        let sig = self.signer.sign_hash_sync(&signing_hash).context("signing EIP-712 hash")?;

        Ok(SignatureRsv {
            r: B256::from(sig.r().to_be_bytes::<32>()),
            s: B256::from(sig.s().to_be_bytes::<32>()),
            // HL expects 27/28 (legacy v). alloy's Signature::v() returns the y-parity bool.
            v: if sig.v().y_parity() { 28 } else { 27 },
        })
    }
}

/// Sign an `approveAgent` action with the MAIN wallet. One-time setup.
/// Hand-rolled EIP-712 because the type name contains a colon.
pub fn sign_approve_agent(
    main_key_hex: &str,
    agent_address: Address,
    agent_name: &str,
    nonce: u64,
    is_mainnet: bool,
) -> Result<SignatureRsv> {
    let signer: PrivateKeySigner = main_key_hex.parse().context("parsing main key")?;
    let chain_id = user_signed_chain_id(is_mainnet);

    // typeHash for "HyperliquidTransaction:ApproveAgent(string hyperliquidChain,address agentAddress,string agentName,uint64 nonce)"
    let type_hash = keccak256(
        b"HyperliquidTransaction:ApproveAgent(string hyperliquidChain,address agentAddress,string agentName,uint64 nonce)"
    );

    // structHash = keccak(typeHash || keccak(chain) || pad(addr) || keccak(name) || pad(nonce))
    let chain_str = if is_mainnet { "Mainnet" } else { "Testnet" };
    let mut buf = Vec::with_capacity(32 * 5);
    buf.extend_from_slice(type_hash.as_slice());
    buf.extend_from_slice(keccak256(chain_str.as_bytes()).as_slice());
    buf.extend_from_slice(&pad_address(agent_address));
    buf.extend_from_slice(keccak256(agent_name.as_bytes()).as_slice());
    buf.extend_from_slice(&pad_u64(nonce));
    let struct_hash = keccak256(&buf);

    // domainSeparator for { name: "HyperliquidSignTransaction", version: "1", chainId, verifyingContract: 0 }
    let domain_separator = compute_domain_separator(
        "HyperliquidSignTransaction",
        "1",
        chain_id,
        Address::ZERO,
    );

    // signingHash = keccak("\x19\x01" || domainSeparator || structHash)
    let mut sbuf = Vec::with_capacity(2 + 32 + 32);
    sbuf.push(0x19);
    sbuf.push(0x01);
    sbuf.extend_from_slice(domain_separator.as_slice());
    sbuf.extend_from_slice(struct_hash.as_slice());
    let signing_hash = keccak256(&sbuf);

    let sig = signer.sign_hash_sync(&signing_hash).context("signing approveAgent")?;

    Ok(SignatureRsv {
        r: B256::from(sig.r().to_be_bytes::<32>()),
        s: B256::from(sig.s().to_be_bytes::<32>()),
        v: if sig.v().y_parity() { 28 } else { 27 },
    })
}

fn pad_address(a: Address) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(a.as_slice());
    out
}

fn pad_u64(n: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&n.to_be_bytes());
    out
}

fn compute_domain_separator(name: &str, version: &str, chain_id: u64, verifying_contract: Address) -> B256 {
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );
    let mut buf = Vec::with_capacity(32 * 5);
    buf.extend_from_slice(type_hash.as_slice());
    buf.extend_from_slice(keccak256(name.as_bytes()).as_slice());
    buf.extend_from_slice(keccak256(version.as_bytes()).as_slice());
    let mut cid = [0u8; 32];
    cid[24..].copy_from_slice(&chain_id.to_be_bytes());
    buf.extend_from_slice(&cid);
    buf.extend_from_slice(&pad_address(verifying_contract));
    keccak256(&buf)
}

fn compute_action_hash(action: &Value, nonce: u64, vault: Option<Address>) -> Result<B256> {
    let mut buf = rmp_serde::to_vec_named(action).context("msgpack encode action")?;
    buf.extend_from_slice(&nonce.to_be_bytes());
    match vault {
        None => buf.push(0x00),
        Some(addr) => {
            buf.push(0x01);
            buf.extend_from_slice(addr.as_slice());
        }
    }
    Ok(keccak256(&buf))
}

#[derive(Debug, Clone)]
pub struct SignatureRsv {
    pub r: B256,
    pub s: B256,
    pub v: u8,
}
