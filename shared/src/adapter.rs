use async_trait::async_trait;

use crate::types::{ChainId, ChainRuntime, CrossChainEvent, RawLog, TxStatus};

/// Core abstraction for a bridge protocol.
///
/// Adapters are dispatched via enum on the hot path — do not box these as
/// `dyn ProtocolAdapter` in the indexer's event loop.
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Stable identifier for this protocol, e.g. "layerzero-v2".
    fn protocol_id(&self) -> &str;

    /// Which chain runtime processes this adapter's events.
    fn chain_runtime(&self) -> ChainRuntime;

    /// Attempt to parse a raw log into a CrossChainEvent.
    /// Returns None if this log does not belong to this protocol.
    fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent>;

    /// Derive the correlation key used by the stitcher to join source and
    /// destination events into a single journey.  Must be deterministic.
    fn correlation_id(&self, event: &CrossChainEvent) -> String;

    /// Infer the current status of an event from its parsed data.
    fn status(&self, event: &CrossChainEvent) -> TxStatus;

    /// Chains this adapter can produce events for.
    fn supported_chains(&self) -> Vec<ChainId>;
}
