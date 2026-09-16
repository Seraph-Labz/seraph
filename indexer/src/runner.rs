use std::time::Duration;

use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::types::{BlockNumberOrTag, Filter};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;
use tracing::{debug, error, info, warn};

use seraph_shared::{ChainId, db, db::CrossChainEventRow};

use crate::adapters::AdapterDispatch;
use crate::log;

/// Alchemy's eth_getLogs limit: 2 000 blocks or 10 000 events per call.
const CHUNK_SIZE: u64 = 2_000;

/// Consecutive failed sweeps tolerated before the runner gives up and lets the
/// supervisor rebuild the connection from scratch.
const MAX_RECONCILE_FAILURES: u32 = 5;

pub struct ChainRunner<P> {
    pub chain_id: ChainId,
    pub provider: P,
    pub adapters: Vec<AdapterDispatch>,
    /// Contract addresses to watch. Empty = watch all (use only for testing).
    pub watched_addresses: Vec<Address>,
    /// Block to sweep from on a cold start, when no cursor has been persisted.
    /// `None` starts at the current tip and indexes no history.
    pub start_block: Option<u64>,
    /// How often the reconcile sweep re-scans from the cursor to the tip.
    pub reconcile_interval: Duration,
    pub pool: PgPool,
}

impl<P: Provider + Clone> ChainRunner<P> {
    /// Index this chain until the connection fails. Returns `Err` on any fault
    /// so the supervisor can back off and restart from the persisted cursor.
    pub async fn run(self) -> Result<()> {
        let from = self.resolve_start_block().await?;
        info!(chain = %self.chain_id, from, "chain runner started");

        // Two paths run concurrently, with different jobs:
        //
        //   subscribe() is a latency optimisation — it surfaces events within a
        //   block or two of them landing.
        //
        //   reconcile() is the correctness guarantee. It cannot be dropped in
        //   favour of the subscription alone, because alloy reconnects and
        //   re-subscribes transparently without surfacing the gap, and
        //   SubscriptionStream discards broadcast-lagged notifications with only
        //   a debug log. Live logs are therefore best-effort by construction.
        //
        // Inserts are idempotent, so the overlap between the two costs nothing.
        tokio::try_join!(self.subscribe(), self.reconcile(from))?;
        Ok(())
    }

    // ── start position ────────────────────────────────────────────────────────

    /// Resume point, in priority order: persisted cursor, then the configured
    /// start block, then the current tip.
    async fn resolve_start_block(&self) -> Result<u64> {
        if let Some(cursor) = db::get_chain_cursor(&self.pool, &self.chain_id.0).await? {
            info!(chain = %self.chain_id, cursor, "resuming from persisted cursor");
            return Ok(cursor + 1);
        }

        if let Some(configured) = self.start_block {
            info!(chain = %self.chain_id, configured, "no cursor yet, starting from configured block");
            return Ok(configured);
        }

        let tip = self.provider.get_block_number().await?;
        warn!(
            chain = %self.chain_id,
            tip,
            "no cursor and no EVM_START_BLOCK — starting at the tip, no history will be indexed"
        );
        Ok(tip)
    }

    // ── reconcile sweep ───────────────────────────────────────────────────────

    /// Re-scan from the cursor to the tip on a fixed interval, forever.
    ///
    /// This is what makes a dropped connection lossless: whatever the live
    /// subscription missed is picked up on the next tick, whether it was lost to
    /// a silent reconnect, broadcast lag, or the process being restarted.
    async fn reconcile(&self, from: u64) -> Result<()> {
        let mut cursor = from;
        let mut failures: u32 = 0;

        let mut ticker = tokio::time::interval(self.reconcile_interval);
        // A sweep that overruns its interval must not be followed by a burst of
        // catch-up ticks — space them out instead.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            // The first tick resolves immediately, so the initial catch-up runs
            // before we ever wait.
            ticker.tick().await;

            match self.sweep(cursor).await {
                Ok(Some(swept_to)) => {
                    cursor = swept_to + 1;
                    failures = 0;
                }
                // Chain has not advanced past the cursor — nothing to do.
                Ok(None) => failures = 0,
                Err(e) => {
                    failures += 1;
                    error!(chain = %self.chain_id, error = %e, failures, "reconcile sweep failed");

                    // Transient RPC errors are expected; a run of them means the
                    // connection is no longer usable and needs rebuilding.
                    if failures >= MAX_RECONCILE_FAILURES {
                        return Err(anyhow!(
                            "reconcile failed {failures} times consecutively on {}",
                            self.chain_id
                        ));
                    }
                }
            }
        }
    }

    /// One sweep from `from` to the current tip.
    /// Returns the last block covered, or `None` if the chain has not advanced.
    async fn sweep(&self, from: u64) -> Result<Option<u64>> {
        let tip = self.provider.get_block_number().await?;
        if from > tip {
            return Ok(None);
        }

        debug!(chain = %self.chain_id, from, tip, "sweep started");

        let mut cursor = from;
        while cursor <= tip {
            let end = (cursor + CHUNK_SIZE - 1).min(tip);
            self.fetch_chunk(cursor, end).await?;

            // Persist per chunk rather than per sweep: a crash mid-sweep then
            // costs one chunk of rework instead of the whole range.
            db::upsert_chain_cursor(&self.pool, &self.chain_id.0, end).await?;
            cursor = end + 1;
        }

        Ok(Some(tip))
    }

    async fn fetch_chunk(&self, from: u64, to: u64) -> Result<()> {
        let filter = self.build_filter(
            BlockNumberOrTag::Number(from),
            Some(BlockNumberOrTag::Number(to)),
        );
        let logs = self.provider.get_logs(&filter).await?;
        debug!(chain = %self.chain_id, from, to, count = logs.len(), "chunk fetched");

        for l in &logs {
            // Propagate rather than log-and-continue: the cursor advances only
            // once the whole chunk is written, so swallowing an insert error
            // here would step over the event and lose it for good.
            self.process(l).await?;
        }
        Ok(())
    }

    // ── live subscription ─────────────────────────────────────────────────────

    async fn subscribe(&self) -> Result<()> {
        // `fromBlock` is ignored by eth_subscribe — a subscription only ever
        // delivers new logs. History is the reconcile sweep's job.
        let filter = self.build_filter(BlockNumberOrTag::Latest, None);
        let sub = self.provider.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        info!(chain = %self.chain_id, "subscribed to live logs");

        while let Some(log) = stream.next().await {
            // Live inserts are best-effort. The block is at or ahead of the
            // cursor, so a failure here is picked up by the next sweep.
            if let Err(e) = self.process(&log).await {
                warn!(chain = %self.chain_id, error = %e, "live insert failed, deferring to sweep");
            }
        }

        // The stream only ends once alloy has exhausted its own reconnect budget,
        // so this is a hard failure. Returning Err tears down the sibling sweep
        // and hands control back to the supervisor.
        Err(anyhow!("live log stream closed on {}", self.chain_id))
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

    /// Offer a log to each adapter in turn and persist whatever claims it.
    ///
    /// Returns `Err` only on a write failure — a log that no adapter recognises
    /// is the overwhelmingly common case and is not an error.
    async fn process(&self, log: &alloy::rpc::types::Log) -> Result<()> {
        let raw = log::to_raw(log, self.chain_id.clone());

        for adapter in &self.adapters {
            if let Some(event) = adapter.parse_event(&raw) {
                let row = CrossChainEventRow::from(&event);
                db::insert_event(&self.pool, &row)
                    .await
                    .map_err(|e| anyhow!("insert failed for {}: {e}", event.source_tx_hash))?;

                debug!(tx = %event.source_tx_hash, "event inserted");
                return Ok(());
            }
        }

        Ok(())
    }
}
