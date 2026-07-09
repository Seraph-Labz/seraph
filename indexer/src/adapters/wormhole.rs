use seraph_shared::{CrossChainEvent, RawLog};

pub struct Wormhole;

impl Wormhole {
    pub fn parse_event(&self, _log: &RawLog) -> Option<CrossChainEvent> {
        // Implemented in step 2.8
        None
    }

    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        event.correlation_id.clone()
    }
}
