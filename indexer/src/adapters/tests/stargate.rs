use super::*;
use seraph_shared::types::chain;

fn push_slot(data: &mut Vec<u8>, bytes: &[u8]) {
    let mut s = [0u8; 32];
    s[32 - bytes.len()..].copy_from_slice(bytes);
    data.extend_from_slice(&s);
}

// OFTSent has 2 indexed params (guid, from) and 3 non-indexed (dstEid,
// amountSentLD, amountReceivedLD).
//
// Topics: [SIGNATURE_HASH, guid (bytes32), from (address)]
// Data:   [dstEid (uint32), amountSentLD (uint256), amountReceivedLD (uint256)]
fn make_log(guid: [u8; 32], from: [u8; 20], dst_eid: u32, amount_sent: u128) -> RawLog {
    let mut topic1 = [0u8; 32];
    topic1.copy_from_slice(&guid);

    let mut topic2 = [0u8; 32];
    topic2[12..].copy_from_slice(&from);

    let mut data = Vec::<u8>::new();
    push_slot(&mut data, &dst_eid.to_be_bytes()); // dstEid (uint32)
    push_slot(&mut data, &amount_sent.to_be_bytes()); // amountSentLD (uint256, low 128 bits)
    push_slot(&mut data, &(amount_sent - 1000).to_be_bytes()); // amountReceivedLD

    RawLog {
        address: "0x77b2043768d28E9C9aB44E1aBfC95944bcE57931".to_string(),
        topics: vec![
            format!("{}", OFTSent::SIGNATURE_HASH),
            format!("0x{}", alloy::primitives::hex::encode(topic1)),
            format!("0x{}", alloy::primitives::hex::encode(topic2)),
        ],
        data,
        block_number: Some(22_000_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

#[test]
fn parses_valid_oft_sent() {
    let guid = [0xAB; 32];
    let from = [0x11; 20];
    let log = make_log(guid, from, 30110, 5_000_000);

    let event = Stargate.parse_event(&log).expect("should parse OFTSent");

    assert_eq!(event.protocol_id, "stargate");
    assert_eq!(event.source_chain, chain::ethereum());
    assert_eq!(event.dest_chain, Some(chain::arbitrum()));
    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "11".repeat(20))
    );
    // guid bytes32 → "0xabab...ab"
    assert_eq!(event.correlation_id, format!("0x{}", "ab".repeat(32)));
    assert_eq!(event.amount, Some("5000000".to_string()));
    assert!(event.receiver_address.is_none());
    assert_eq!(event.metadata["dst_eid"], 30110);
}

#[test]
fn rejects_wrong_topic() {
    let mut log = make_log([0u8; 32], [0u8; 20], 30110, 1_000);
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Stargate.parse_event(&log).is_none());
}

#[test]
fn unknown_dst_eid_gives_none_dest_chain() {
    let log = make_log([0u8; 32], [0u8; 20], 99_999, 1_000);
    let event = Stargate.parse_event(&log).expect("should still parse");
    assert!(event.dest_chain.is_none());
}

#[test]
fn guid_is_correlation_id() {
    let guid = [0xCC; 32];
    let log = make_log(guid, [0u8; 20], 30110, 1_000);
    let event = Stargate.parse_event(&log).unwrap();
    assert_eq!(event.correlation_id, format!("0x{}", "cc".repeat(32)));
}
