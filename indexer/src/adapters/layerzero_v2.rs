use alloy::primitives::{Address, B256};
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── 2.2.1  Event signature ────────────────────────────────────────────────────
//
// The sol! macro generates a Rust struct from a Solidity event definition.
// It computes SIGNATURE_HASH (keccak256 of the signature string) at compile
// time and generates an ABI decoder — no runtime parsing of the ABI JSON.
//
// Source: https://github.com/LayerZero-Labs/LayerZero-v2/blob/main/packages/layerzero-v2/evm/protocol/contracts/interfaces/ILayerZeroEndpointV2.sol
sol! {
    event PacketSent(bytes encodedPacket, bytes options, address sendLibrary);
}

// ── 2.2.3  encodedPacket byte layout (PacketV1Codec.sol) ─────────────────────
//
// LayerZero encodes all routing information into a single `bytes` field.
// The layout is versioned; V1 (the only version in production) is:
//
//   offset  0 : version  (1 byte,  u8  = 1)
//   offset  1 : nonce    (8 bytes, u64, big-endian)
//   offset  9 : srcEid   (4 bytes, u32, big-endian)  ← LZ endpoint ID, not EVM chainId
//   offset 13 : sender   (32 bytes, bytes32)          ← EVM address right-aligned in 32 bytes
//   offset 45 : dstEid   (4 bytes, u32, big-endian)
//   offset 49 : receiver (32 bytes, bytes32)
//   offset 81 : guid     (32 bytes, bytes32)          ← correlation key
//   offset 113: message  (variable)                   ← application payload, not needed here
//
// Why bytes32 for addresses? LZ is cross-VM — Solana and Aptos have 32-byte
// addresses. EVM addresses (20 bytes) sit in the rightmost 20 bytes; the
// leading 12 bytes are zero padding.
const MIN_PACKET_LEN: usize = 113;

pub struct LayerZeroV2;

impl LayerZeroV2 {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        // ── Step 1: check event signature ─────────────────────────────────────
        //
        // EVM logs have `topics[0]` = keccak256(event_signature). We decode all
        // topic hex strings into B256 values and confirm the first one matches
        // PacketSent::SIGNATURE_HASH before doing any further work.
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&PacketSent::SIGNATURE_HASH) {
            return None;
        }

        // ── Step 2: ABI-decode the log ────────────────────────────────────────
        //
        // decode_raw_log handles the ABI encoding rules for dynamic types
        // (bytes, string) which use a two-step indirect encoding: the data
        // section starts with a 32-byte offset pointer, then the actual bytes.
        // We don't need to think about that — alloy does it for us.
        let decoded = PacketSent::decode_raw_log(topics, &log.data).ok()?;

        // ── Step 3: slice the raw packet bytes ────────────────────────────────
        let packet = decoded.encodedPacket.as_ref();
        if packet.len() < MIN_PACKET_LEN {
            return None;
        }

        let nonce = u64::from_be_bytes(packet[1..9].try_into().ok()?);
        let src_eid = u32::from_be_bytes(packet[9..13].try_into().ok()?);
        let sender32 = &packet[13..45];
        let dst_eid = u32::from_be_bytes(packet[45..49].try_into().ok()?);
        let recv32 = &packet[49..81];
        let guid_bytes = &packet[81..113];

        // ── Step 4: convert raw bytes to typed values ─────────────────────────
        //
        // sender/receiver: take the last 20 bytes of the 32-byte field.
        // Address::to_string() returns EIP-55 checksum format "0xAbCd...".
        //
        // guid: wrap in B256 so .to_string() gives the canonical "0x00..." hex.
        // This is what the stitcher stores in both source and destination events;
        // an exact string match is all it needs to join them.
        let sender_addr = Address::from_slice(&sender32[12..]).to_string();
        let receiver_addr = Address::from_slice(&recv32[12..]).to_string();
        let guid_hex = B256::from_slice(guid_bytes).to_string();

        // ── 2.2.2  Endpoint ID → ChainId ─────────────────────────────────────
        let dest_chain = eid_to_chain(dst_eid);

        // ── Step 5: build the event ───────────────────────────────────────────
        //
        // source_chain comes from log.chain_id — the runner already knows which
        // chain it's indexing and stamps every RawLog with it. We don't re-derive
        // it from srcEid to avoid any mismatch if the runner is started on an
        // unexpected chain.
        //
        // amount / token_address are None because PacketSent is a messaging event,
        // not a token transfer event. The amount lives inside the `message` payload
        // in an application-specific encoding — we don't parse OApp message formats here.
        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: sender_addr,
            receiver_address: Some(receiver_addr),
            amount: None,
            token_address: None,
            protocol_id: "layerzero-v2".to_string(),
            correlation_id: guid_hex,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "src_eid": src_eid,
                "dst_eid": dst_eid,
                "nonce": nonce,
            }),
            created_at: now,
            updated_at: now,
        })
    }

    // ── 2.2.5  correlation_id ─────────────────────────────────────────────────
    //
    // The GUID was already stored in event.correlation_id when parse_event ran.
    // The stitcher calls this to get the join key without re-parsing the event.
    #[allow(dead_code)]
    pub fn correlation_id(&self, event: &CrossChainEvent) -> String {
        event.correlation_id.clone()
    }
}

// ── 2.2.2  Endpoint ID → ChainId lookup ──────────────────────────────────────
//
// LayerZero assigns its own monotonic IDs ("eids") to chains — completely
// separate from EVM chainIds. This table covers our 7 indexed chains.
// Returns None for any eid we don't yet support; the stitcher can handle
// an unknown dest_chain gracefully.
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
#[path = "tests/layerzero_v2.rs"]
mod tests;
