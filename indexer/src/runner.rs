use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::types::{BlockNumberOrTag, Filter};
use anyhow::Result;
use futures::StreamExt;
use sqlx::PgPool;
use tracing::{debug, error, info};

use seraph_shared::{ChainId, db, db::CrossChainEventRow};

use crate::adapters::AdapterDispatch;
use crate::log;

/// Alchemy's eth_getLogs limit: 2 000 blocks or 10 000 events per call.
const CHUNK_SIZE: u64 = 2_000;

pub struct ChainRunner<P> {
    pub chain_id: ChainId,
    pub provider: P,
    pub adapters: Vec<AdapterDispatch>,
    /// Contract addresses to watch. Empty = watch all (use only for testing).
    pub watched_addresses: Vec<Address>,
    /// First block to include in the backfill pass.
    pub start_block: u64,
    pub pool: PgPool,
}

impl<P: Provider + Clone> ChainRunner<P> {
    /// Run backfill then hand off to the live subscription. Blocks until the
    /// WebSocket stream closes or an unrecoverable error occurs.
    pub async fn run(self) -> Result<()> {
        let tip = self.provider.get_block_number().await?;
        info!(chain = %self.chain_id, tip, "chain runner started");

        self.backfill(self.start_block, tip).await?;
        self.subscribe(tip + 1).await
    }

    // ── backfill ──────────────────────────────────────────────────────────────

    async fn backfill(&self, from: u64, to: u64) -> Result<()> {
        if from > to {
            return Ok(());
        }
        info!(chain = %self.chain_id, from, to, "backfill started");

        let mut cursor = from;
        while cursor <= to {
            let end = (cursor + CHUNK_SIZE - 1).min(to);
            self.fetch_chunk(cursor, end).await?;
            cursor = end + 1;
        }

        info!(chain = %self.chain_id, "backfill complete");
        Ok(())
    }

    async fn fetch_chunk(&self, from: u64, to: u64) -> Result<()> {
        let filter = self.build_filter(
            BlockNumberOrTag::Number(from),
            Some(BlockNumberOrTag::Number(to)),
        );
        let logs = self.provider.get_logs(&filter).await?;
        debug!(chain = %self.chain_id, from, to, count = logs.len(), "chunk fetched");

        for l in &logs {
            self.process(l).await;
        }
        Ok(())
    }

    // ── live subscription ─────────────────────────────────────────────────────

    async fn subscribe(&self, from_block: u64) -> Result<()> {
        info!(chain = %self.chain_id, from_block, "subscribing to live logs");

        let filter = self.build_filter(BlockNumberOrTag::Number(from_block), None);
        let sub = self.provider.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        while let Some(log) = stream.next().await {
            self.process(&log).await;
        }

        Ok(())
    }

    // ── shared helpers ────────────────────────────────────────────────────────

    fn build_filter(&self, from: BlockNumberOrTag, to: Option<BlockNumberOrTag>) -> Filter {
        let mut f = Filter::new().from_block(from);
        if !self.watched_addresses.is_empty() {
            f = f.address(self.watched_addresses.clone());
        }
        if let Some(to) = to {
            f = f.to_block(to);
        }
        f
    }

    async fn process(&self, log: &alloy::rpc::types::Log) {
        let raw = log::to_raw(log, self.chain_id.clone());

        for adapter in &self.adapters {
            if let Some(event) = adapter.parse_event(&raw) {
                let row = CrossChainEventRow::from(&event);
                match db::insert_event(&self.pool, &row).await {
                    Ok(_) => debug!(tx = %event.source_tx_hash, "event inserted"),
                    Err(e) => error!(error = %e, tx = %event.source_tx_hash, "insert failed"),
                }
                return;
            }
        }
    }
}
