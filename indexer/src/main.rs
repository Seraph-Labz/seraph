mod adapters;
mod log;
mod provider;
mod runner;

use anyhow::Context;
use seraph_shared::{Config, chain, db};
use tracing::{error, info};

use runner::ChainRunner;

struct ChainConfig {
    chain_id: seraph_shared::ChainId,
    wss_url: String,
    /// First block included in the backfill pass.
    /// Set EVM_START_BLOCK to override; defaults to the current tip (no backfill).
    start_block: u64,
}

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

    // EVM_START_BLOCK lets operators replay from a known block.
    // Defaults to u64::MAX so the runner skips the backfill pass and only
    // picks up live events — safe when there is no high-water mark yet.
    let start_block: u64 = std::env::var("EVM_START_BLOCK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(u64::MAX);

    let key = &config.alchemy_api_key;
    let chains = vec![
        ChainConfig {
            chain_id: chain::ethereum(),
            wss_url: format!("wss://eth-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
        ChainConfig {
            chain_id: chain::arbitrum(),
            wss_url: format!("wss://arb-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
        ChainConfig {
            chain_id: chain::optimism(),
            wss_url: format!("wss://opt-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
        ChainConfig {
            chain_id: chain::base(),
            wss_url: format!("wss://base-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
        ChainConfig {
            chain_id: chain::polygon(),
            wss_url: format!("wss://polygon-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
        ChainConfig {
            chain_id: chain::bsc(),
            wss_url: format!("wss://bnb-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
        ChainConfig {
            chain_id: chain::avalanche(),
            wss_url: format!("wss://avax-mainnet.g.alchemy.com/v2/{key}"),
            start_block,
        },
    ];

    let mut handles = Vec::new();

    for chain in chains {
        let pool = pool.clone();

        let handle = tokio::spawn(async move {
            let provider = match provider::connect(&chain.wss_url).await {
                Ok(p) => p,
                Err(e) => {
                    error!(chain = %chain.chain_id, error = %e, "provider connection failed");
                    return;
                }
            };

            let runner = ChainRunner {
                chain_id: chain.chain_id.clone(),
                provider,
                adapters: adapters::all(),
                watched_addresses: vec![],
                start_block: chain.start_block,
                pool,
            };

            if let Err(e) = runner.run().await {
                error!(chain = %chain.chain_id, error = %e, "chain runner exited with error");
            }
        });

        handles.push(handle);
    }

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
