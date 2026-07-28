use super::*;
use seraph_shared::types::chain;

// LogMessagePublished: 1 indexed param, 4 non-indexed (sequence, nonce static;
// payload dynamic; consistencyLevel static).
//
// Topics: [SIGNATURE_HASH, sender(address)]
// Data ABI layout (4-slot head + tail):
//   slot 0: sequence (uint64)
//   slot 1: nonce (uint32)
//   slot 2: offset to payload = 4 * 32 = 128
//   slot 3: consistencyLevel (uint8)
//   tail:   payload length (0 for empty)
fn make_log(sender: [u8; 20], sequence: u64, nonce: u32, consistency: u8) -> RawLog {
    let mut t_sender = [0u8; 32];
    t_sender[12..].copy_from_slice(&sender);

    let mut data = Vec::<u8>::new();
    let mut u256_slot = |val: u64| {
        let mut s = [0u8; 32];
        s[24..].copy_from_slice(&val.to_be_bytes());
        data.extend_from_slice(&s);
    };

    u256_slot(sequence); // sequence (uint64 right-aligned)
    u256_slot(nonce as u64); // nonce (uint32 right-aligned)
    u256_slot(4 * 32); // offset to payload = 128
    u256_slot(consistency as u64); // consistencyLevel (uint8 right-aligned)
    data.extend_from_slice(&[0u8; 32]); // payload length = 0 (empty)

    RawLog {
        address: "0x98f3c9e6E3fAce36bAAd05FE09d375Ef1464288B".to_string(),
        topics: vec![
            format!("{}", LogMessagePublished::SIGNATURE_HASH),
            format!("0x{}", alloy::primitives::hex::encode(t_sender)),
        ],
        data,
        block_number: Some(22_000_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

#[test]
fn parses_valid_log_message_published() {
    let sender = [0xAA; 20];
    let log = make_log(sender, 7, 0, 1);

    let event = Wormhole
        .parse_event(&log)
        .expect("should parse LogMessagePublished");

    assert_eq!(event.protocol_id, "wormhole");
    assert_eq!(event.source_chain, chain::ethereum());
    // dest_chain is not in the event (opaque payload)
    assert!(event.dest_chain.is_none());
    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "aa".repeat(20))
    );
    // correlation_id = "wormhole:<wh_chain_id>:<sender>:<sequence>"
    // Ethereum → wormhole chain ID 2
    assert!(event.correlation_id.starts_with("wormhole:2:"));
    assert!(event.correlation_id.ends_with(":7"));
    assert_eq!(event.metadata["sequence"], 7);
    assert_eq!(event.metadata["nonce"], 0);
    assert_eq!(event.metadata["consistency_level"], 1);
}

#[test]
fn rejects_wrong_topic() {
    let mut log = make_log([0u8; 20], 1, 0, 1);
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Wormhole.parse_event(&log).is_none());
}

#[test]
fn correlation_id_includes_wormhole_chain_id() {
    // BSC → wormhole chain ID 4
    let mut log = make_log([0xBB; 20], 99, 0, 1);
    log.chain_id = chain::bsc();
    let event = Wormhole.parse_event(&log).unwrap();
    assert!(event.correlation_id.starts_with("wormhole:4:"));
    assert!(event.correlation_id.ends_with(":99"));
}

#[test]
fn unknown_evm_chain_gives_wh_id_zero() {
    // An unrecognized chain maps to wh_chain 0 but the event still parses.
    let mut log = make_log([0u8; 20], 1, 0, 1);
    log.chain_id = seraph_shared::ChainId("unknown".to_string());
    let event = Wormhole.parse_event(&log).unwrap();
    assert!(event.correlation_id.starts_with("wormhole:0:"));
}
