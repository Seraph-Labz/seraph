use seraph_shared::{ChainId, RawLog};

/// Convert an alloy RPC log into our chain-agnostic RawLog.
pub fn to_raw(log: &alloy::rpc::types::Log, chain_id: ChainId) -> RawLog {
    let inner = &log.inner;
    RawLog {
        address: inner.address.to_string(),
        topics: inner.topics().iter().map(|t| t.to_string()).collect(),
        data: inner.data.data.to_vec(),
        block_number: log.block_number,
        tx_hash: log.transaction_hash.map(|h| h.to_string()),
        log_index: log.log_index,
        chain_id,
    }
}
