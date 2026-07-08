use seraph_shared::{CrossChainEvent, RawLog};

/// Enum-dispatched set of active adapters.
///
/// Add one variant per bridge protocol. Never use `dyn ProtocolAdapter` here —
/// the compiler monomorphizes each arm so there is zero heap allocation on the
/// hot path.
pub enum AdapterDispatch {
    /// Placeholder that matches nothing. Remove once real adapters land.
    NoOp,
}

impl AdapterDispatch {
    /// Try to parse a raw log with this adapter. Returns `None` if the log
    /// does not belong to this protocol.
    pub fn parse_event(&self, _log: &RawLog) -> Option<CrossChainEvent> {
        match self {
            Self::NoOp => None,
        }
    }

    /// Derive the correlation key used by the stitcher to join source and
    /// destination events.
    #[allow(dead_code)]
    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        match self {
            Self::NoOp => event.correlation_id.clone(),
        }
    }
}
