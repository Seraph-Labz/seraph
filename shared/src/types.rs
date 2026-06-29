use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque chain identifier string.  Use the constants in [`chain`] for well-known chains.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl ChainId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for ChainId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ChainId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// Which chain runtime handles a given adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainRuntime {
    Evm,
    Solana,
    Cosmos,
}

impl std::fmt::Display for ChainRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evm => f.write_str("evm"),
            Self::Solana => f.write_str("solana"),
            Self::Cosmos => f.write_str("cosmos"),
        }
    }
}

impl TryFrom<&str> for ChainRuntime {
    type Error = String;
    fn try_from(s: &str) -> std::result::Result<Self, String> {
        match s {
            "evm" => Ok(Self::Evm),
            "solana" => Ok(Self::Solana),
            "cosmos" => Ok(Self::Cosmos),
            other => Err(format!("unknown ChainRuntime: {other}")),
        }
    }
}

/// Lifecycle status for a cross-chain event or stitched transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    /// Emitted on source chain; destination not yet seen.
    Pending,
    /// Bridge acknowledged; destination not yet confirmed.
    Inflight,
    /// Delivery confirmed on destination chain.
    Completed,
    /// Terminal failure — refunded, reverted, or timed out.
    Failed,
}

impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("pending"),
            Self::Inflight => f.write_str("inflight"),
            Self::Completed => f.write_str("completed"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

impl TryFrom<&str> for TxStatus {
    type Error = String;
    fn try_from(s: &str) -> std::result::Result<Self, String> {
        match s {
            "pending" => Ok(Self::Pending),
            "inflight" => Ok(Self::Inflight),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown TxStatus: {other}")),
        }
    }
}

/// Parsed cross-chain event ready for DB insertion or stitching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainEvent {
    pub id: Uuid,
    /// Transaction hash on the source chain.
    pub source_tx_hash: String,
    pub source_chain: ChainId,
    /// Destination chain — may be unknown at parse time for some protocols.
    pub dest_chain: Option<ChainId>,
    pub sender_address: String,
    pub receiver_address: Option<String>,
    /// Amount as a decimal string to handle u256 without precision loss.
    pub amount: Option<String>,
    /// Token contract address; None for native assets.
    pub token_address: Option<String>,
    /// Protocol that emitted this event, e.g. "layerzero-v2".
    pub protocol_id: String,
    /// Key used by the stitcher to correlate source and destination events.
    pub correlation_id: String,
    pub status: TxStatus,
    /// Protocol-specific fields stored as JSONB.
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A fully correlated cross-chain journey produced by the stitcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StitchedTransaction {
    pub id: Uuid,
    pub correlation_id: String,
    pub source_chain: ChainId,
    pub dest_chain: ChainId,
    pub sender_address: String,
    pub receiver_address: Option<String>,
    pub amount: Option<String>,
    pub token_address: Option<String>,
    pub protocol_id: String,
    pub status: TxStatus,
    pub source_event_id: Option<Uuid>,
    pub dest_event_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw log from any chain runtime, passed to adapters for parsing.
///
/// EVM: built from `alloy::rpc::types::Log`.
/// Solana: built from Helius webhook payload.
/// Cosmos: built from Tendermint ABCI event.
#[derive(Debug, Clone)]
pub struct RawLog {
    /// Contract or program address that emitted the log.
    pub address: String,
    /// Hex-encoded topics (EVM) or instruction discriminators (Solana).
    pub topics: Vec<String>,
    /// Raw ABI-encoded data bytes.
    pub data: Vec<u8>,
    pub block_number: Option<u64>,
    /// Hex-encoded transaction hash.
    pub tx_hash: Option<String>,
    pub log_index: Option<u64>,
    pub chain_id: ChainId,
}

/// Well-known chain ID constants.  Extend as new adapters require more chains.
pub mod chain {
    use super::ChainId;

    pub const ETHEREUM: &str = "ethereum";
    pub const ARBITRUM: &str = "arbitrum";
    pub const OPTIMISM: &str = "optimism";
    pub const BASE: &str = "base";
    pub const POLYGON: &str = "polygon";
    pub const BSC: &str = "bsc";
    pub const AVALANCHE: &str = "avalanche";
    pub const SOLANA: &str = "solana";
    pub const COSMOS_HUB: &str = "cosmoshub";
    pub const OSMOSIS: &str = "osmosis";

    pub fn ethereum() -> ChainId { ChainId::new(ETHEREUM) }
    pub fn arbitrum() -> ChainId { ChainId::new(ARBITRUM) }
    pub fn optimism() -> ChainId { ChainId::new(OPTIMISM) }
    pub fn base() -> ChainId { ChainId::new(BASE) }
    pub fn polygon() -> ChainId { ChainId::new(POLYGON) }
    pub fn bsc() -> ChainId { ChainId::new(BSC) }
    pub fn avalanche() -> ChainId { ChainId::new(AVALANCHE) }
    pub fn solana() -> ChainId { ChainId::new(SOLANA) }
    pub fn cosmos_hub() -> ChainId { ChainId::new(COSMOS_HUB) }
    pub fn osmosis() -> ChainId { ChainId::new(OSMOSIS) }
}
