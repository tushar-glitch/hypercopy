//! Decision engine: takes an observed fill and either emits an OrderIntent
//! (mirror open / mirror close) or skips with a reason.
//!
//! v0 logic:
//!   - skip snapshot fills (we only mirror live activity)
//!   - skip if risk caps already hit
//!   - on close-direction fill from target: emit reduce-only close of our mirror
//!   - on open-direction fill: size by config.sizing.fixed_pct of our equity,
//!     send IOC near mid

use crate::config::Config;
use crate::positions::Manager;
use crate::types::{FillEvent, OrderIntent, TimeInForce};

pub enum Action {
    Mirror(OrderIntent),
    CloseMirror(OrderIntent),
    Skip(String),
}

pub struct Engine {
    cfg: Config,
}

impl Engine {
    pub fn new(cfg: Config) -> Self { Self { cfg } }

    pub fn decide(&self, fill: &FillEvent, positions: &Manager) -> Action {
        if fill.is_snapshot {
            return Action::Skip("snapshot fill — not mirroring history".into());
        }

        if fill.is_close {
            // Mirror-close path: only if we actually have a mirror open for this coin/target.
            if let Some(open) = positions.find_open(&fill.target_address, &fill.coin) {
                let intent = OrderIntent {
                    coin: fill.coin.clone(),
                    asset_index: 0, // TODO: resolve via InfoClient::meta() cache
                    side: invert(open.side),
                    px: fill.px, // IOC near observed px; executor may re-quote off mid
                    sz: open.sz,
                    reduce_only: true,
                    tif: TimeInForce::Ioc,
                    source_target: fill.target_address.clone(),
                };
                return Action::CloseMirror(intent);
            }
            return Action::Skip("close fill but no matching mirror open".into());
        }

        // Open-direction fill. Apply risk caps.
        if positions.open_count() >= self.cfg.risk.max_concurrent_positions {
            return Action::Skip("max_concurrent_positions reached".into());
        }

        // TODO: real sizing needs our current equity (clearinghouseState).
        // For scaffold, derive notional from config.sizing.fixed_pct of max_notional_usd.
        let notional = self.cfg.risk.max_notional_usd * (self.cfg.sizing.fixed_pct / 100.0);
        let sz = notional / fill.px;
        if sz <= 0.0 {
            return Action::Skip("computed size <= 0".into());
        }

        Action::Mirror(OrderIntent {
            coin: fill.coin.clone(),
            asset_index: 0, // TODO: resolve via meta
            side: fill.side,
            px: fill.px,
            sz,
            reduce_only: false,
            tif: TimeInForce::Ioc,
            source_target: fill.target_address.clone(),
        })
    }
}

fn invert(s: crate::types::Side) -> crate::types::Side {
    match s {
        crate::types::Side::Buy => crate::types::Side::Sell,
        crate::types::Side::Sell => crate::types::Side::Buy,
    }
}
