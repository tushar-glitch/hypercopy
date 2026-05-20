//! One-time helper: authorize the agent wallet to trade on behalf of your main wallet.
//!
//! Usage:
//!   1. Put your MAIN wallet private key in env: HYPERCOPY_MAIN_KEY=0x...
//!   2. Make sure config.toml has [wallet].agent_address set to your bot's agent address.
//!   3. cargo run --bin approve-agent
//!
//! After this succeeds once, your bot (signing as the agent) can place orders
//! that count against your main account. The agent CANNOT withdraw funds.

use anyhow::{anyhow, Context, Result};
use hypercopy::config::Config;
use hypercopy::execute::signer::{sign_approve_agent, user_signed_chain_id};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cfg = Config::load("config.toml")?;
    let is_mainnet = cfg.hyperliquid.is_mainnet();

    let main_key = std::env::var("HYPERCOPY_MAIN_KEY")
        .context("set HYPERCOPY_MAIN_KEY in .env (main wallet private key)")?;

    let agent_addr: alloy_primitives::Address = cfg.wallet.agent_address.parse()
        .context("parsing config.wallet.agent_address")?;

    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let agent_name = "hypercopy";

    let sig = sign_approve_agent(&main_key, agent_addr, agent_name, nonce, is_mainnet)?;

    let action = json!({
        "type": "approveAgent",
        "hyperliquidChain": if is_mainnet { "Mainnet" } else { "Testnet" },
        "signatureChainId": format!("0x{:x}", user_signed_chain_id(is_mainnet)),
        "agentAddress": format!("{:#x}", agent_addr),
        "agentName": agent_name,
        "nonce": nonce,
    });

    let body = json!({
        "action": action,
        "nonce": nonce,
        "signature": {
            "r": format!("0x{:x}", sig.r),
            "s": format!("0x{:x}", sig.s),
            "v": sig.v,
        }
    });

    let url = format!("{}/exchange", cfg.hyperliquid.api_url);
    tracing::info!(%url, agent = %format!("{:#x}", agent_addr), network = %cfg.hyperliquid.network, "submitting approveAgent");

    let resp = reqwest::Client::new().post(&url).json(&body).send().await?;
    let v: Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);

    if v.get("status").and_then(|s| s.as_str()) == Some("ok") {
        println!("\n✅ Agent approved. You can now run `cargo run` to start the bot.");
        Ok(())
    } else {
        Err(anyhow!("approveAgent failed; see response above"))
    }
}
