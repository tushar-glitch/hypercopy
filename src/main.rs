mod config;
mod types;
mod ingest;
mod decision;
mod execute;
mod positions;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,hypercopy=debug".into()))
        .init();

    let cfg = config::Config::load("config.toml")?;
    tracing::info!(targets = cfg.targets.len(), "hypercopy starting");

    let (tx, rx) = tokio::sync::mpsc::channel::<types::FillEvent>(1024);

    // Spawn one WS subscriber per enabled target.
    for target in cfg.targets.iter().filter(|t| t.enabled) {
        let url = cfg.hyperliquid.ws_url.clone();
        let target = target.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = ingest::ws::run(url, target, tx).await {
                tracing::error!(?e, "ws subscriber exited");
            }
        });
    }
    drop(tx);

    // Main loop: consume fills, decide, execute.
    let engine = decision::Engine::new(cfg.clone());
    let executor = execute::Executor::new(cfg.clone())?;
    let mut positions = positions::Manager::new();

    let mut rx = rx;
    while let Some(fill) = rx.recv().await {
        tracing::info!(?fill, "fill observed");
        match engine.decide(&fill, &positions) {
            decision::Action::Mirror(intent) => {
                match executor.submit(&intent).await {
                    Ok(receipt) => {
                        tracing::info!(?receipt, "order submitted");
                        positions.record_open(&intent, &receipt);
                    }
                    Err(e) => tracing::error!(?e, "submit failed"),
                }
            }
            decision::Action::CloseMirror(intent) => {
                match executor.submit(&intent).await {
                    Ok(receipt) => {
                        tracing::info!(?receipt, "close submitted");
                        positions.record_close(&intent, &receipt);
                    }
                    Err(e) => tracing::error!(?e, "close failed"),
                }
            }
            decision::Action::Skip(reason) => {
                tracing::debug!(%reason, "skipped");
            }
        }
    }

    Ok(())
}
