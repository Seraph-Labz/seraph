use seraph_shared::{CrossChainEvent, RawLog};

pub struct Connext;

impl Connext {
    pub fn parse_event(&self, _log: &RawLog) -> Option<CrossChainEvent> {
        // Implemented in step 2.7
        None
    }

    #[allow(dead_code)]
    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        event.correlation_id.clone()
    }
}
