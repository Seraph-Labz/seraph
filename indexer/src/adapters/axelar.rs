use seraph_shared::{CrossChainEvent, RawLog};

pub struct Axelar;

impl Axelar {
    pub fn parse_event(&self, _log: &RawLog) -> Option<CrossChainEvent> {
        // Implemented in step 2.9
        None
    }

    #[allow(dead_code)]
    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        event.correlation_id.clone()
    }
}
