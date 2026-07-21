use super::*;
use seraph_shared::types::chain;

fn push_slot(data: &mut Vec<u8>, bytes: &[u8]) {
    let mut s = [0u8; 32];
    s[32 - bytes.len()..].copy_from_slice(bytes);
    data.extend_from_slice(&s);
}

// TransferSent: 3 indexed params, 5 non-indexed (all static).
//
// Topics: [SIGNATURE_HASH, transferId(bytes32), chainId(uint256), recipient(address)]
// Data:   [amount(uint256), transferNonce(bytes32), bonderFee(uint256),
//           amountOutMin(uint256), deadline(uint256)]
fn make_log(transfer_id: [u8; 32], chain_id_evm: u64, recipient: [u8; 20], amount: u128) -> RawLog {
    let mut t_chain_id = [0u8; 32];
    t_chain_id[24..].copy_from_slice(&chain_id_evm.to_be_bytes());

    let mut t_recipient = [0u8; 32];
    t_recipient[12..].copy_from_slice(&recipient);

    let mut data = Vec::<u8>::new();
    push_slot(&mut data, &amount.to_be_bytes()); // amount (uint256)
    data.extend_from_slice(&[0u8; 32]); // transferNonce (bytes32)
    data.extend_from_slice(&[0u8; 32]); // bonderFee (uint256)
    data.extend_from_slice(&[0u8; 32]); // amountOutMin (uint256)
    push_slot(&mut data, &u64::MAX.to_be_bytes()); // deadline (uint256, far future)

    RawLog {
        address: "0x3d4Cc8A61c7528Fd86C55cfe061a78dCBA48EDd1".to_string(),
        topics: vec![
            format!("{}", TransferSent::SIGNATURE_HASH),
            format!("0x{}", alloy::primitives::hex::encode(transfer_id)),
            format!("0x{}", alloy::primitives::hex::encode(t_chain_id)),
            format!("0x{}", alloy::primitives::hex::encode(t_recipient)),
        ],
        data,
        block_number: Some(22_000_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::arbitrum(),
    }
}

#[test]
fn parses_valid_transfer_sent() {
    let transfer_id = [0xAB; 32];
    let recipient = [0xCC; 20];

    let log = make_log(transfer_id, 1, recipient, 5_000_000_000_000_000_000); // dest = mainnet
    let event = Hop.parse_event(&log).expect("should parse TransferSent");

    assert_eq!(event.protocol_id, "hop");
    assert_eq!(event.source_chain, chain::arbitrum());
    assert_eq!(event.dest_chain, Some(chain::ethereum())); // EVM chain ID 1
    assert_eq!(event.correlation_id, format!("0x{}", "ab".repeat(32)));
    assert_eq!(
        event.receiver_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "cc".repeat(20)))
    );
    assert_eq!(event.amount, Some("5000000000000000000".to_string()));
    // sender_address is intentionally empty — msg.sender is not in TransferSent
    assert!(event.sender_address.is_empty());
    assert!(event.token_address.is_none());
}

#[test]
fn rejects_wrong_topic() {
    let mut log = make_log([0u8; 32], 1, [0u8; 20], 1_000);
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Hop.parse_event(&log).is_none());
}

#[test]
fn unknown_dest_chain_id_gives_none_dest_chain() {
    let log = make_log([0u8; 32], 99_999, [0u8; 20], 1_000);
    let event = Hop.parse_event(&log).expect("should still parse");
    assert!(event.dest_chain.is_none());
}

#[test]
fn transfer_id_is_correlation_id() {
    let id = [0xDD; 32];
    let log = make_log(id, 42161, [0u8; 20], 1_000);
    let event = Hop.parse_event(&log).unwrap();
    assert_eq!(event.correlation_id, format!("0x{}", "dd".repeat(32)));
    // dest = arbitrum (chain ID 42161)
    assert_eq!(event.dest_chain, Some(chain::arbitrum()));
}
