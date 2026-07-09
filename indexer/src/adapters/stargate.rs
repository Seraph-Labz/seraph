use seraph_shared::{CrossChainEvent, RawLog};

pub struct Stargate;

impl Stargate {
    pub fn parse_event(&self, _log: &RawLog) -> Option<CrossChainEvent> {
        // Implemented in step 2.4
        None
    }

    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        event.correlation_id.clone()
    }
}
