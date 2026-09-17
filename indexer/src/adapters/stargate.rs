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

        // ── guid is zero for bus-mode transfers ───────────────────────────────
        //
        // In "taxi" mode the transfer gets its own LayerZero packet immediately
        // and guid is the packet GUID, which also appears in OFTReceived on the
        // destination — a direct join key.
        //
        // In "bus" mode Stargate batches transfers. OFTSent fires when a transfer
        // boards the bus, before any packet exists, so guid is zero(bytes32). The
        // real GUID is only assigned later by BusDriven, in a different
        // transaction.
        //
        // Verified against Base tx 0x2e9b21b090f4f1642ccf5962133ccf66c436f0ce…:
        // the OFTSent log has topic[1] == 0x00…00 while every other field decodes
        // correctly, and the same transaction carries a BusRode log plus a LI.FI
        // "stargateV2Bus" marker.
        //
        // Falling back to a per-log key keeps each transfer distinct. Without it
        // every bus transfer shares correlation_id 0x00…00, and because
        // stitched_transactions has a UNIQUE constraint on correlation_id the
        // stitcher would fold all Stargate traffic into a single journey.
        //
        // This fallback key cannot join to the destination event. Doing that
        // needs BusRode's ticketId and BusDriven's guid, which live in sibling
        // logs this adapter cannot see — parse_event is handed one log at a
        // time. Tracked in #12.
        let correlation_id = if decoded.guid == B256::ZERO {
            format!(
                "stargate:bus:{}:{}:{}",
                log.chain_id,
                log.tx_hash.as_deref().unwrap_or("unknown"),
                log.log_index.unwrap_or_default()
            )
        } else {
            decoded.guid.to_string()
        };

        let bus_mode = decoded.guid == B256::ZERO;
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
                // Flags a synthetic correlation_id that cannot join to a
                // destination event, so the stitcher can tell the two apart.
                "bus_mode":           bus_mode,
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
