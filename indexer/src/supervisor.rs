use std::time::{Duration, Instant};

use alloy::primitives::Address;
use sqlx::PgPool;
use tracing::{error, warn};

use seraph_shared::ChainId;

use crate::adapters;
use crate::backoff::Backoff;
use crate::provider;
use crate::runner::ChainRunner;

/// Per-chain settings resolved at startup.
pub struct ChainConfig {
    pub chain_id: ChainId,
    pub wss_url: String,
    /// Bridge contracts to watch on this chain. Resolved once at startup and
    /// reused across reconnects; must be non-empty.
    pub watched_addresses: Vec<Address>,
    /// Cold-start block, used only when no cursor has been persisted yet.
    pub start_block: Option<u64>,
    pub reconcile_interval: Duration,
}

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// A connection that stays up this long is treated as healthy, so the next
/// failure starts backing off from scratch.
const STABLE_WINDOW: Duration = Duration::from_secs(300);

/// Keep one chain indexed for the life of the process, rebuilding the provider
/// with exponential backoff whenever the runner faults.
///
/// Never returns — the caller aborts the task on shutdown. Each restart resumes
/// from the persisted cursor, so a reconnect re-scans the downtime window rather
/// than skipping it.
pub async fn supervise(chain: ChainConfig, pool: PgPool) {
    let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF, STABLE_WINDOW);

    loop {
        let started = Instant::now();

        match provider::connect(&chain.wss_url).await {
            Ok(p) => {
                let runner = ChainRunner {
                    chain_id: chain.chain_id.clone(),
                    provider: p,
                    adapters: adapters::all(),
                    watched_addresses: chain.watched_addresses.clone(),
                    start_block: chain.start_block,
                    reconcile_interval: chain.reconcile_interval,
                    pool: pool.clone(),
                };

                match runner.run().await {
                    // run() only resolves Ok if both halves finish, which they
                    // never do — either arm is a fault.
                    Ok(()) => warn!(chain = %chain.chain_id, "chain runner stopped unexpectedly"),
                    Err(e) => error!(chain = %chain.chain_id, error = %e, "chain runner failed"),
                }
            }
            Err(e) => error!(chain = %chain.chain_id, error = %e, "provider connection failed"),
        }

        backoff.record_uptime(started.elapsed());
        let delay = backoff.next_delay();

        warn!(chain = %chain.chain_id, ?delay, "restarting chain runner after backoff");
        tokio::time::sleep(delay).await;
    }
}
