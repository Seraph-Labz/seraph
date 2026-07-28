use alloy::primitives::B256;
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── Event signature ───────────────────────────────────────────────────────────
//
// Stargate V2 is built on top of LayerZero V2's OFT (Omnichain Fungible Token)
// standard. Each token pool (ETH, USDC, USDT, mETH, …) is its own contract,
// all emitting the same OFTSent event when a cross-chain transfer is initiated.
//
// Indexed → topics[1..2]:
//   guid — bytes32 derived from the underlying LZ V2 packet GUID
//   from — the address that initiated the send
//
// Non-indexed → data:
//   dstEid           — LayerZero endpoint ID of the destination chain
//   amountSentLD     — amount locked on source in local decimals
//   amountReceivedLD — amount the recipient gets after fees
//
// The recipient address is encoded inside the LZ message payload and is not
// surfaced directly in OFTSent. receiver_address is set to None; it will be
// filled in by the corresponding OFTReceived event on the destination side.
//
// Source: https://github.com/LayerZero-Labs/devtools/blob/main/packages/oft-evm/contracts/OFTCore.sol
sol! {
    event OFTSent(
        bytes32 indexed guid,
        uint32 dstEid,
        address indexed from,
        uint256 amountSentLD,
        uint256 amountReceivedLD
    );
}

pub struct Stargate;

impl Stargate {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&OFTSent::SIGNATURE_HASH) {
            return None;
        }

        let decoded = OFTSent::decode_raw_log(topics, &log.data).ok()?;

        // guid is the LZ V2 packet GUID — the same value appears in the
        // OFTReceived event on the destination chain, giving the stitcher a
        // direct join key without any secondary lookup.
        let correlation_id = decoded.guid.to_string();
        let dest_chain = eid_to_chain(decoded.dstEid);

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: decoded.from.to_string(),
            receiver_address: None,
            amount: Some(decoded.amountSentLD.to_string()),
            // log.address is the pool contract (e.g. the USDC pool), not the
            // underlying ERC-20 token. We store it in metadata so the frontend
            // can identify which asset was bridged.
            token_address: None,
            protocol_id: "stargate".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "dst_eid":            decoded.dstEid,
                "amount_received_ld": decoded.amountReceivedLD.to_string(),
                "pool_address":       log.address,
            }),
            created_at: now,
            updated_at: now,
        })
    }

    #[allow(dead_code)]
    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        event.correlation_id.clone()
    }
}

// LayerZero endpoint ID → internal ChainId (same table as layerzero_v2.rs).
fn eid_to_chain(eid: u32) -> Option<ChainId> {
    match eid {
        30101 => Some(chain::ethereum()),
        30110 => Some(chain::arbitrum()),
        30111 => Some(chain::optimism()),
        30184 => Some(chain::base()),
        30109 => Some(chain::polygon()),
        30102 => Some(chain::bsc()),
        30106 => Some(chain::avalanche()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/stargate.rs"]
mod tests;
