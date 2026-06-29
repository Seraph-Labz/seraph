use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::SeraphError,
    types::{ChainId, CrossChainEvent, StitchedTransaction, TxStatus},
};

/// DB row for `cross_chain_events`.  Use [`CrossChainEvent`] in application code.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrossChainEventRow {
    pub id: Uuid,
    pub source_tx_hash: String,
    pub source_chain: String,
    pub dest_chain: Option<String>,
    pub sender_address: String,
    pub receiver_address: Option<String>,
    pub amount: Option<String>,
    pub token_address: Option<String>,
    pub protocol_id: String,
    pub correlation_id: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<CrossChainEventRow> for CrossChainEvent {
    type Error = SeraphError;

    fn try_from(row: CrossChainEventRow) -> Result<Self, SeraphError> {
        Ok(Self {
            id: row.id,
            source_tx_hash: row.source_tx_hash,
            source_chain: ChainId::new(row.source_chain),
            dest_chain: row.dest_chain.map(ChainId::new),
            sender_address: row.sender_address,
            receiver_address: row.receiver_address,
            amount: row.amount,
            token_address: row.token_address,
            protocol_id: row.protocol_id,
            correlation_id: row.correlation_id,
            status: TxStatus::try_from(row.status.as_str()).map_err(SeraphError::Parse)?,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl From<&CrossChainEvent> for CrossChainEventRow {
    fn from(e: &CrossChainEvent) -> Self {
        Self {
            id: e.id,
            source_tx_hash: e.source_tx_hash.clone(),
            source_chain: e.source_chain.0.clone(),
            dest_chain: e.dest_chain.as_ref().map(|c| c.0.clone()),
            sender_address: e.sender_address.clone(),
            receiver_address: e.receiver_address.clone(),
            amount: e.amount.clone(),
            token_address: e.token_address.clone(),
            protocol_id: e.protocol_id.clone(),
            correlation_id: e.correlation_id.clone(),
            status: e.status.to_string(),
            metadata: e.metadata.clone(),
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

/// DB row for `stitched_transactions`.  Use [`StitchedTransaction`] in application code.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StitchedTransactionRow {
    pub id: Uuid,
    pub correlation_id: String,
    pub source_chain: String,
    pub dest_chain: String,
    pub sender_address: String,
    pub receiver_address: Option<String>,
    pub amount: Option<String>,
    pub token_address: Option<String>,
    pub protocol_id: String,
    pub status: String,
    pub source_event_id: Option<Uuid>,
    pub dest_event_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<StitchedTransactionRow> for StitchedTransaction {
    type Error = SeraphError;

    fn try_from(row: StitchedTransactionRow) -> Result<Self, SeraphError> {
        Ok(Self {
            id: row.id,
            correlation_id: row.correlation_id,
            source_chain: ChainId::new(row.source_chain),
            dest_chain: ChainId::new(row.dest_chain),
            sender_address: row.sender_address,
            receiver_address: row.receiver_address,
            amount: row.amount,
            token_address: row.token_address,
            protocol_id: row.protocol_id,
            status: TxStatus::try_from(row.status.as_str()).map_err(SeraphError::Parse)?,
            source_event_id: row.source_event_id,
            dest_event_id: row.dest_event_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl From<&StitchedTransaction> for StitchedTransactionRow {
    fn from(tx: &StitchedTransaction) -> Self {
        Self {
            id: tx.id,
            correlation_id: tx.correlation_id.clone(),
            source_chain: tx.source_chain.0.clone(),
            dest_chain: tx.dest_chain.0.clone(),
            sender_address: tx.sender_address.clone(),
            receiver_address: tx.receiver_address.clone(),
            amount: tx.amount.clone(),
            token_address: tx.token_address.clone(),
            protocol_id: tx.protocol_id.clone(),
            status: tx.status.to_string(),
            source_event_id: tx.source_event_id,
            dest_event_id: tx.dest_event_id,
            created_at: tx.created_at,
            updated_at: tx.updated_at,
        }
    }
}

/// DB row for `protocol_adapters` — the registry of supported protocols.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolAdapterRow {
    pub id: String,
    pub name: String,
    pub chain_runtime: String,
    pub supported_chains: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
