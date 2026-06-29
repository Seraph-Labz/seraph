pub mod models;

pub use models::{CrossChainEventRow, ProtocolAdapterRow, StitchedTransactionRow};

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::error::{Result, SeraphError};

// ── Pool ──────────────────────────────────────────────────────────────────────

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(SeraphError::Database)
}

/// Run all pending migrations from `shared/migrations/`.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!()
        .run(pool)
        .await
        .map_err(SeraphError::Migration)
}

// ── cross_chain_events ────────────────────────────────────────────────────────

/// Insert a new event.  Silently skips duplicates (same source_tx_hash + source_chain).
pub async fn insert_event(pool: &PgPool, row: &CrossChainEventRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO cross_chain_events (
            id, source_tx_hash, source_chain, dest_chain, sender_address,
            receiver_address, amount, token_address, protocol_id,
            correlation_id, status, metadata, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (source_tx_hash, source_chain) DO NOTHING",
    )
    .bind(row.id)
    .bind(&row.source_tx_hash)
    .bind(&row.source_chain)
    .bind(&row.dest_chain)
    .bind(&row.sender_address)
    .bind(&row.receiver_address)
    .bind(&row.amount)
    .bind(&row.token_address)
    .bind(&row.protocol_id)
    .bind(&row.correlation_id)
    .bind(&row.status)
    .bind(&row.metadata)
    .bind(row.created_at)
    .bind(row.updated_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(SeraphError::Database)
}

pub async fn get_event_by_tx_hash(
    pool: &PgPool,
    tx_hash: &str,
) -> Result<Option<CrossChainEventRow>> {
    sqlx::query_as::<_, CrossChainEventRow>(
        "SELECT * FROM cross_chain_events WHERE source_tx_hash = $1 LIMIT 1",
    )
    .bind(tx_hash)
    .fetch_optional(pool)
    .await
    .map_err(SeraphError::Database)
}

pub async fn get_events_by_correlation_id(
    pool: &PgPool,
    correlation_id: &str,
) -> Result<Vec<CrossChainEventRow>> {
    sqlx::query_as::<_, CrossChainEventRow>(
        "SELECT * FROM cross_chain_events
         WHERE correlation_id = $1
         ORDER BY created_at ASC",
    )
    .bind(correlation_id)
    .fetch_all(pool)
    .await
    .map_err(SeraphError::Database)
}

pub async fn get_events_by_sender(
    pool: &PgPool,
    address: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<CrossChainEventRow>> {
    sqlx::query_as::<_, CrossChainEventRow>(
        "SELECT * FROM cross_chain_events
         WHERE sender_address = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(address)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(SeraphError::Database)
}

/// Returns events not yet joined to a stitched_transaction — used by the stitcher.
pub async fn get_unstitched_events(pool: &PgPool) -> Result<Vec<CrossChainEventRow>> {
    sqlx::query_as::<_, CrossChainEventRow>(
        "SELECT e.* FROM cross_chain_events e
         LEFT JOIN stitched_transactions s ON s.correlation_id = e.correlation_id
         WHERE s.id IS NULL
           AND e.status != 'failed'
         ORDER BY e.created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(SeraphError::Database)
}

pub async fn update_event_status(pool: &PgPool, id: Uuid, status: &str) -> Result<()> {
    sqlx::query(
        "UPDATE cross_chain_events
         SET status = $1, updated_at = NOW()
         WHERE id = $2",
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(SeraphError::Database)
}

// ── stitched_transactions ─────────────────────────────────────────────────────

/// Insert or update a stitched transaction.  On conflict, updates status,
/// dest_event_id, dest_chain, and receiver_address — handles the common case
/// where the destination event arrives after the initial stitch.
pub async fn upsert_stitched_tx(pool: &PgPool, row: &StitchedTransactionRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO stitched_transactions (
            id, correlation_id, source_chain, dest_chain, sender_address,
            receiver_address, amount, token_address, protocol_id, status,
            source_event_id, dest_event_id, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (correlation_id) DO UPDATE SET
            status           = EXCLUDED.status,
            dest_chain       = EXCLUDED.dest_chain,
            dest_event_id    = EXCLUDED.dest_event_id,
            receiver_address = EXCLUDED.receiver_address,
            updated_at       = NOW()",
    )
    .bind(row.id)
    .bind(&row.correlation_id)
    .bind(&row.source_chain)
    .bind(&row.dest_chain)
    .bind(&row.sender_address)
    .bind(&row.receiver_address)
    .bind(&row.amount)
    .bind(&row.token_address)
    .bind(&row.protocol_id)
    .bind(&row.status)
    .bind(row.source_event_id)
    .bind(row.dest_event_id)
    .bind(row.created_at)
    .bind(row.updated_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(SeraphError::Database)
}

pub async fn get_stitched_tx_by_correlation_id(
    pool: &PgPool,
    correlation_id: &str,
) -> Result<Option<StitchedTransactionRow>> {
    sqlx::query_as::<_, StitchedTransactionRow>(
        "SELECT * FROM stitched_transactions WHERE correlation_id = $1",
    )
    .bind(correlation_id)
    .fetch_optional(pool)
    .await
    .map_err(SeraphError::Database)
}

pub async fn get_stitched_txs_by_sender(
    pool: &PgPool,
    address: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<StitchedTransactionRow>> {
    sqlx::query_as::<_, StitchedTransactionRow>(
        "SELECT * FROM stitched_transactions
         WHERE sender_address = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(address)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(SeraphError::Database)
}

// ── protocol_adapters ─────────────────────────────────────────────────────────

pub async fn get_protocol_adapter(
    pool: &PgPool,
    id: &str,
) -> Result<Option<ProtocolAdapterRow>> {
    sqlx::query_as::<_, ProtocolAdapterRow>(
        "SELECT * FROM protocol_adapters WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(SeraphError::Database)
}

pub async fn list_enabled_adapters(pool: &PgPool) -> Result<Vec<ProtocolAdapterRow>> {
    sqlx::query_as::<_, ProtocolAdapterRow>(
        "SELECT * FROM protocol_adapters WHERE enabled = true ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(SeraphError::Database)
}

pub async fn get_protocol_stats(
    pool: &PgPool,
    protocol_id: &str,
) -> Result<ProtocolStats> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'completed') AS completed,
            COUNT(*) FILTER (WHERE status = 'pending')   AS pending,
            COUNT(*) FILTER (WHERE status = 'inflight')  AS inflight,
            COUNT(*) FILTER (WHERE status = 'failed')    AS failed
         FROM stitched_transactions
         WHERE protocol_id = $1",
    )
    .bind(protocol_id)
    .fetch_one(pool)
    .await
    .map_err(SeraphError::Database)?;

    Ok(ProtocolStats {
        protocol_id: protocol_id.to_owned(),
        completed: row.0,
        pending: row.1,
        inflight: row.2,
        failed: row.3,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    pub protocol_id: String,
    pub completed: i64,
    pub pending: i64,
    pub inflight: i64,
    pub failed: i64,
}

use serde::{Deserialize, Serialize};
