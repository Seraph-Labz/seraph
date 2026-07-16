use alloy::primitives::B256;
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{CrossChainEvent, RawLog, TxStatus};

// ── Event signature ───────────────────────────────────────────────────────────
//
// Wormhole is a generic message-passing protocol. Its Core Bridge emits
// LogMessagePublished for every cross-chain message, regardless of whether it
// carries tokens or arbitrary data.
//
// Indexed → topics[1]:
//   sender — the emitter contract address (e.g. the Token Bridge or an OApp)
//
// Non-indexed → data:
//   sequence        — monotonically increasing per emitter address
//   nonce           — caller-assigned deduplication nonce
//   payload         — opaque bytes; encoding is application-specific
//   consistencyLevel — finality level required before guardians sign the VAA
//
// A VAA (Verified Action Approval) is uniquely identified by the triple
// (emitter chain, emitter address, sequence). We embed the Wormhole chain ID
// and emitter address in the correlation_id so the stitcher can reconstruct
// the VAA key for the destination-side event lookup.
//
// dest_chain is not directly available in this event — it is encoded inside
// the payload in a format that differs per application (Token Bridge, NFT
// Bridge, OApp). We set it to None; the stitcher can fill it in if needed.
//
// Source: https://github.com/wormhole-foundation/wormhole/blob/main/ethereum/contracts/interfaces/IWormhole.sol
sol! {
    event LogMessagePublished(
        address indexed sender,
        uint64 sequence,
        uint32 nonce,
        bytes payload,
        uint8 consistencyLevel
    );
}

pub struct Wormhole;

impl Wormhole {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&LogMessagePublished::SIGNATURE_HASH) {
            return None;
        }

        let decoded = LogMessagePublished::decode_raw_log(topics, &log.data).ok()?;

        // VAA key = (wormhole_chain_id, emitter_address, sequence).
        // Including the Wormhole chain ID ensures uniqueness even if two chains
        // happen to have the same emitter address and sequence.
        let wh_chain = wormhole_chain_id(log.chain_id.0.as_str());
        let correlation_id = format!(
            "wormhole:{}:{}:{}",
            wh_chain, decoded.sender, decoded.sequence
        );

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain: None,
            sender_address: decoded.sender.to_string(),
            receiver_address: None,
            amount: None,
            token_address: None,
            protocol_id: "wormhole".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "sequence":         decoded.sequence,
                "nonce":            decoded.nonce,
                "consistency_level": decoded.consistencyLevel,
                "payload_len":      decoded.payload.len(),
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

// EVM chain → Wormhole chain ID.
// Wormhole maintains its own chain ID registry, distinct from EVM chain IDs.
// Source: https://docs.wormhole.com/wormhole/reference/constants#chain-ids
fn wormhole_chain_id(chain: &str) -> u16 {
    match chain {
        "ethereum" => 2,
        "bsc" => 4,
        "polygon" => 5,
        "avalanche" => 6,
        "arbitrum" => 23,
        "optimism" => 24,
        "base" => 30,
        _ => 0,
    }
}

#[cfg(test)]
#[path = "tests/wormhole.rs"]
mod tests;
