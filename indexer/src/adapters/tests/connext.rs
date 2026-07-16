use super::*;
use seraph_shared::types::chain;

// XCalled: 3 indexed params, 5 non-indexed.
//
// Topics: [SIGNATURE_HASH, transferId(bytes32), nonce(uint256), messageHash(bytes32)]
//
// Non-indexed data encodes (TransferInfo, address asset, uint256 amount,
// address local, bytes messageBody). TransferInfo is dynamic because it
// contains `bytes callData`. Full layout = 640 bytes; see byte-offset
// comments inside make_log for details.
#[allow(clippy::too_many_arguments)]
fn make_log(
    transfer_id: [u8; 32],
    nonce_indexed: u64,
    origin_domain: u32,
    dest_domain: u32,
    to: [u8; 20],
    origin_sender: [u8; 20],
    amount: u128,
    asset: [u8; 20],
) -> RawLog {
    let mut t_nonce = [0u8; 32];
    t_nonce[24..].copy_from_slice(&nonce_indexed.to_be_bytes());

    // Pre-zeroed; we only write non-zero values.
    let mut data = vec![0u8; 640];

    // Right-align `bytes` into a 32-byte slot starting at `off`.
    fn write_slot(data: &mut [u8], off: usize, bytes: &[u8]) {
        data[off + 32 - bytes.len()..off + 32].copy_from_slice(bytes);
    }

    // ── Top-level head (5 × 32 = 160 B) ────────────────────────────────────
    // [  0.. 32] offset to params   = 160
    write_slot(&mut data, 0, &160u64.to_be_bytes());
    // [ 32.. 64] asset (address, right-aligned 20 B in 32 B)
    write_slot(&mut data, 32, &asset);
    // [ 64.. 96] amount (uint256)
    write_slot(&mut data, 64, &amount.to_be_bytes());
    // [ 96..128] local = zero address (already zeroed)
    // [128..160] offset to messageBody = 608 (160 + 448)
    write_slot(&mut data, 128, &608u64.to_be_bytes());

    // ── TransferInfo head (13 × 32 = 416 B, starts at 160) ─────────────────
    // [160..192] originDomain (uint32)
    write_slot(&mut data, 160, &origin_domain.to_be_bytes());
    // [192..224] destinationDomain (uint32)
    write_slot(&mut data, 192, &dest_domain.to_be_bytes());
    // [224..256] canonicalDomain = 0
    // [256..288] to (address)
    write_slot(&mut data, 256, &to);
    // [288..320] delegate = zero
    // [320..352] receiveLocal = false
    // [352..384] offset to callData = 416 (relative to TransferInfo start)
    write_slot(&mut data, 352, &416u64.to_be_bytes());
    // [384..416] slippage = 0
    // [416..448] originSender (address)
    write_slot(&mut data, 416, &origin_sender);
    // [448..480] bridgedAmt = 0
    // [480..512] normalizedIn = 0
    // [512..544] nonce = 0
    // [544..576] canonicalId = zero bytes32

    // ── TransferInfo tail: callData length = 0 at data[576..608] ────────────
    // ── Top-level tail: messageBody length = 0 at data[608..640] ────────────
    // Both already zeroed.

    RawLog {
        address: "0x8898B472C54c31894e3B9bb83cEA802a5d0e63C6".to_string(),
        topics: vec![
            format!("{}", XCalled::SIGNATURE_HASH),
            format!("0x{}", alloy::primitives::hex::encode(transfer_id)),
            format!("0x{}", alloy::primitives::hex::encode(t_nonce)),
            format!("0x{}", alloy::primitives::hex::encode([0u8; 32])), // messageHash
        ],
        data,
        block_number: Some(22_000_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

#[test]
fn parses_valid_xcalled() {
    let transfer_id = [0xAB; 32];
    let to = [0xCC; 20];
    let origin_sender = [0xDD; 20];
    let asset = [0xEE; 20];

    let log = make_log(
        transfer_id,
        1,
        6_648_936,     // Connext domain: ethereum
        1_634_886_255, // Connext domain: arbitrum
        to,
        origin_sender,
        5_000_000,
        asset,
    );
    let event = Connext.parse_event(&log).expect("should parse XCalled");

    assert_eq!(event.protocol_id, "connext");
    assert_eq!(event.source_chain, chain::ethereum());
    assert_eq!(event.dest_chain, Some(chain::arbitrum()));
    assert_eq!(event.correlation_id, format!("0x{}", "ab".repeat(32)));
    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "dd".repeat(20))
    );
    assert_eq!(
        event.receiver_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "cc".repeat(20)))
    );
    assert_eq!(event.amount, Some("5000000".to_string()));
    assert_eq!(
        event.token_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "ee".repeat(20)))
    );
    assert_eq!(event.metadata["origin_domain"], 6_648_936);
    assert_eq!(event.metadata["destination_domain"], 1_634_886_255);
}

#[test]
fn rejects_wrong_topic() {
    let mut log = make_log(
        [0u8; 32],
        0,
        6_648_936,
        1_634_886_255,
        [0u8; 20],
        [0u8; 20],
        0,
        [0u8; 20],
    );
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Connext.parse_event(&log).is_none());
}

#[test]
fn unknown_dest_domain_gives_none_dest_chain() {
    let log = make_log(
        [0u8; 32], 0, 6_648_936, 99_999, [0u8; 20], [0u8; 20], 0, [0u8; 20],
    );
    let event = Connext.parse_event(&log).expect("should still parse");
    assert!(event.dest_chain.is_none());
}

#[test]
fn transfer_id_is_correlation_id() {
    let id = [0xFF; 32];
    let log = make_log(
        id,
        0,
        6_648_936,
        1_634_886_255,
        [0u8; 20],
        [0u8; 20],
        0,
        [0u8; 20],
    );
    let event = Connext.parse_event(&log).unwrap();
    assert_eq!(event.correlation_id, format!("0x{}", "ff".repeat(32)));
}
