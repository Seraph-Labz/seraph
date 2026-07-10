use super::*;
use seraph_shared::types::chain;

// Build a valid PacketV1 byte array with the given routing fields.
// sender/receiver are right-aligned in their bytes32 slots (leading 12 zero bytes).
fn make_packet(
    nonce: u64,
    src_eid: u32,
    sender: [u8; 20],
    dst_eid: u32,
    receiver: [u8; 20],
    guid: [u8; 32],
) -> Vec<u8> {
    let mut p = vec![0u8; 113];
    p[0] = 1; // version
    p[1..9].copy_from_slice(&nonce.to_be_bytes());
    p[9..13].copy_from_slice(&src_eid.to_be_bytes());
    p[25..45].copy_from_slice(&sender); // sender bytes32: zeros[0..12] + address[12..32]
    p[45..49].copy_from_slice(&dst_eid.to_be_bytes());
    p[61..81].copy_from_slice(&receiver); // receiver bytes32: same layout
    p[81..113].copy_from_slice(&guid);
    p
}

// ABI-encode a PacketSent(bytes, bytes, address) log from a raw packet blob.
//
// ABI head section (3 × 32 = 96 bytes):
//   slot 0: byte offset to encodedPacket tail (= 96)
//   slot 1: byte offset to options tail       (= 96 + 32 + padded_len(packet))
//   slot 2: sendLibrary — zero address, static, right-aligned
//
// Tail section:
//   length(encodedPacket) | encodedPacket bytes (padded to 32-byte multiple)
//   length(options) = 0
fn make_log(packet: &[u8]) -> RawLog {
    let n = packet.len();
    let padded = (n + 31) & !31;

    let offset_packet: u64 = 96;
    let offset_options: u64 = 96 + 32 + padded as u64;

    let mut data: Vec<u8> = Vec::new();
    let mut slot = [0u8; 32];

    slot[24..].copy_from_slice(&offset_packet.to_be_bytes());
    data.extend_from_slice(&slot);

    slot[24..].copy_from_slice(&offset_options.to_be_bytes());
    data.extend_from_slice(&slot);

    data.extend_from_slice(&[0u8; 32]); // sendLibrary = zero address

    slot = [0u8; 32];
    slot[24..].copy_from_slice(&(n as u64).to_be_bytes());
    data.extend_from_slice(&slot);
    data.extend_from_slice(packet);
    data.extend_from_slice(&vec![0u8; padded - n]);

    data.extend_from_slice(&[0u8; 32]); // options length = 0

    RawLog {
        address: "0x1a44076050125825900e736c501f859c50fE728c".to_string(),
        topics: vec![format!("{}", PacketSent::SIGNATURE_HASH)],
        data,
        block_number: Some(22_900_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

// ── Test 1: happy path ────────────────────────────────────────────────────────
// Every field sliced from encodedPacket must map to the right CrossChainEvent
// field. This exercises the full parse path end-to-end.
#[test]
fn parses_valid_packet_sent() {
    let sender: [u8; 20] = [0xAA; 20];
    let receiver: [u8; 20] = [0xBB; 20];
    let guid: [u8; 32] = [0x01; 32];

    let packet = make_packet(42, 30101, sender, 30110, receiver, guid);
    let log = make_log(&packet);

    let event = LayerZeroV2
        .parse_event(&log)
        .expect("should parse valid PacketSent");

    assert_eq!(event.protocol_id, "layerzero-v2");
    assert_eq!(event.source_chain, chain::ethereum());
    assert_eq!(event.dest_chain, Some(chain::arbitrum()));

    // EIP-55 checksum only changes casing; compare lowercase to avoid flakiness
    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "aa".repeat(20))
    );
    assert_eq!(
        event.receiver_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "bb".repeat(20)))
    );

    assert_eq!(event.correlation_id, format!("0x{}", "01".repeat(32)));
    assert_eq!(event.status, TxStatus::Pending);
    assert!(event.amount.is_none());
    assert!(event.token_address.is_none());
    assert_eq!(event.source_tx_hash, "0xdeadbeef");
    assert_eq!(event.metadata["src_eid"], 30101);
    assert_eq!(event.metadata["dst_eid"], 30110);
    assert_eq!(event.metadata["nonce"], 42);
}

// ── Test 2: wrong event signature ─────────────────────────────────────────────
// Adapters are tried in sequence by the runner. parse_event must return None
// immediately when topic[0] doesn't match so the runner can try the next one.
#[test]
fn rejects_wrong_topic() {
    let packet = make_packet(1, 30101, [0u8; 20], 30110, [0u8; 20], [0u8; 32]);
    let mut log = make_log(&packet);
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();

    assert!(LayerZeroV2.parse_event(&log).is_none());
}

// ── Test 3: truncated encodedPacket ───────────────────────────────────────────
// If the packet is shorter than 113 bytes the GUID field is missing. parse_event
// must return None without an out-of-bounds slice panic.
#[test]
fn returns_none_for_short_packet() {
    let short = vec![1u8; 80];
    assert!(LayerZeroV2.parse_event(&make_log(&short)).is_none());
}

// ── Test 4: unknown destination eid ───────────────────────────────────────────
// An eid we haven't mapped (e.g. a newly launched chain) must still produce a
// parseable event — just with dest_chain: None — rather than dropping the log.
#[test]
fn unknown_eid_gives_none_dest_chain() {
    let packet = make_packet(1, 30101, [0u8; 20], 99_999, [0u8; 20], [0u8; 32]);
    let event = LayerZeroV2
        .parse_event(&make_log(&packet))
        .expect("should still parse");
    assert!(event.dest_chain.is_none());
}
