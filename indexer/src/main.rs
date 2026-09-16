mod adapters;
mod backoff;
mod log;
mod provider;
mod runner;
mod supervisor;

use std::time::Duration;

use anyhow::Context;
use seraph_shared::{Config, chain, db};
use tracing::info;

use supervisor::ChainConfig;

/// How often each chain re-scans from its cursor to the tip.
const DEFAULT_RECONCILE_INTERVAL_SECS: u64 = 30;

/// Floor for the reconcile interval. tokio::time::interval panics on a zero
/// period, and a sub-second sweep would hammer the RPC provider regardless.
const MIN_RECONCILE_INTERVAL_SECS: u64 = 1;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    dotenvy::dotenv().ok();

    let config = Config::from_env().context("failed to load config")?;

    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .context("failed to connect to database")?;

    info!("running migrations");
    db::run_migrations(&pool)
        .await
        .context("failed to run migrations")?;

    // Cold-start block, used only the first time a chain is indexed. Once a
    // cursor exists in chain_cursors it wins, so restarts resume where they
    // left off regardless of what this is set to.
    let start_block: Option<u64> = std::env::var("EVM_START_BLOCK")
        .ok()
        .and_then(|v| v.parse().ok());

    let reconcile_interval =
        parse_reconcile_interval(std::env::var("EVM_RECONCILE_INTERVAL_SECS").ok().as_deref());

    let key = &config.alchemy_api_key;
    let chains = [
        (
            chain::ethereum(),
            format!("wss://eth-mainnet.g.alchemy.com/v2/{key}"),
        ),
        (
            chain::arbitrum(),
            format!("wss://arb-mainnet.g.alchemy.com/v2/{key}"),
        ),
        (
            chain::optimism(),
            format!("wss://opt-mainnet.g.alchemy.com/v2/{key}"),
        ),
        (
            chain::base(),
            format!("wss://base-mainnet.g.alchemy.com/v2/{key}"),
        ),
        (
            chain::polygon(),
            format!("wss://polygon-mainnet.g.alchemy.com/v2/{key}"),
        ),
        (
            chain::bsc(),
            format!("wss://bnb-mainnet.g.alchemy.com/v2/{key}"),
        ),
        (
            chain::avalanche(),
            format!("wss://avax-mainnet.g.alchemy.com/v2/{key}"),
        ),
    ];

    let handles: Vec<_> = chains
        .into_iter()
        .map(|(chain_id, wss_url)| {
            let chain = ChainConfig {
                chain_id,
                wss_url,
                start_block,
                reconcile_interval,
            };
            let pool = pool.clone();

            // supervise() owns reconnection and never returns, so a chain whose
            // connection drops recovers on its own instead of going dark.
            tokio::spawn(supervisor::supervise(chain, pool))
        })
        .collect();

    info!("indexer running — press Ctrl+C to stop");
    tokio::signal::ctrl_c()
        .await
        .context("failed to listen for Ctrl+C")?;

    info!("shutting down");
    for handle in handles {
        handle.abort();
    }
    pool.close().await;

    Ok(())
}

/// Parse EVM_RECONCILE_INTERVAL_SECS, falling back to the default when unset or
/// unparseable and clamping to a non-zero floor.
fn parse_reconcile_interval(raw: Option<&str>) -> Duration {
    let secs = raw
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_RECONCILE_INTERVAL_SECS)
        .max(MIN_RECONCILE_INTERVAL_SECS);

    Duration::from_secs(secs)
}

#[cfg(test)]
#[path = "tests/startup.rs"]
mod tests;
