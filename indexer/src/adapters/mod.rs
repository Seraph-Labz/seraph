pub mod contracts;

mod across;
mod axelar;
mod cctp;
mod connext;
mod hop;
mod layerzero_v2;
mod stargate;
mod wormhole;

use across::Across;
use axelar::Axelar;
use cctp::Cctp;
use connext::Connext;
use hop::Hop;
use layerzero_v2::LayerZeroV2;
use stargate::Stargate;
use wormhole::Wormhole;

use seraph_shared::{CrossChainEvent, RawLog};

/// Returns one dispatch variant per active protocol, in priority order.
/// The runner tries each adapter in sequence until one claims the log.
pub fn all() -> Vec<AdapterDispatch> {
    vec![
        AdapterDispatch::LayerZeroV2(LayerZeroV2),
        AdapterDispatch::Across(Across),
        AdapterDispatch::Stargate(Stargate),
        AdapterDispatch::Cctp(Cctp),
        AdapterDispatch::Hop(Hop),
        AdapterDispatch::Connext(Connext),
        AdapterDispatch::Wormhole(Wormhole),
        AdapterDispatch::Axelar(Axelar),
    ]
}

/// Enum-dispatched set of active protocol adapters.
///
/// Each variant wraps one bridge protocol. The compiler monomorphizes each
/// match arm so there is zero heap allocation on the hot path — this is why
/// we use an enum instead of `dyn ProtocolAdapter`.
///
/// To add a new protocol: add a variant here, add arms in parse_event and
/// correlation_id, and register the contract addresses in contracts.rs.
pub enum AdapterDispatch {
    LayerZeroV2(LayerZeroV2),
    Across(Across),
    Stargate(Stargate),
    Cctp(Cctp),
    Hop(Hop),
    Connext(Connext),
    Wormhole(Wormhole),
    Axelar(Axelar),
}

impl AdapterDispatch {
    /// Try to parse a raw log into a CrossChainEvent.
    /// Returns None if the log does not belong to this protocol.
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        match self {
            Self::LayerZeroV2(a) => a.parse_event(log),
            Self::Across(a) => a.parse_event(log),
            Self::Stargate(a) => a.parse_event(log),
            Self::Cctp(a) => a.parse_event(log),
            Self::Hop(a) => a.parse_event(log),
            Self::Connext(a) => a.parse_event(log),
            Self::Wormhole(a) => a.parse_event(log),
            Self::Axelar(a) => a.parse_event(log),
        }
    }

    /// Derive the correlation key used by the stitcher to join source and
    /// destination events into a single journey.
    #[allow(dead_code)]
    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        match self {
            Self::LayerZeroV2(a) => a.correlation_id(event),
            Self::Across(a) => a.correlation_id(event),
            Self::Stargate(a) => a.correlation_id(event),
            Self::Cctp(a) => a.correlation_id(event),
            Self::Hop(a) => a.correlation_id(event),
            Self::Connext(a) => a.correlation_id(event),
            Self::Wormhole(a) => a.correlation_id(event),
            Self::Axelar(a) => a.correlation_id(event),
        }
    }
}
