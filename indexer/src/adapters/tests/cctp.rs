use super::*;
use seraph_shared::types::chain;

fn push_slot(data: &mut Vec<u8>, bytes: &[u8]) {
    let mut s = [0u8; 32];
    s[32 - bytes.len()..].copy_from_slice(bytes);
    data.extend_from_slice(&s);
}

// DepositForBurn: 3 indexed params, 5 non-indexed (all static).
//
// Topics: [SIGNATURE_HASH, nonce(uint64), burnToken(address), depositor(address)]
// Data:   [amount(uint256), mintRecipient(bytes32), destinationDomain(uint32),
//           destinationTokenMessenger(bytes32), destinationCaller(bytes32)]
fn make_log(
    nonce: u64,
    burn_token: [u8; 20],
    depositor: [u8; 20],
    amount: u128,
    mint_recipient: [u8; 32],
    dest_domain: u32,
) -> RawLog {
    let mut t_nonce = [0u8; 32];
    t_nonce[24..].copy_from_slice(&nonce.to_be_bytes());

    let mut t_burn_token = [0u8; 32];
    t_burn_token[12..].copy_from_slice(&burn_token);

    let mut t_depositor = [0u8; 32];
    t_depositor[12..].copy_from_slice(&depositor);

    let mut data = Vec::<u8>::new();
    push_slot(&mut data, &amount.to_be_bytes()); // amount (uint256, u128 right-aligned)
    data.extend_from_slice(&mint_recipient); // mintRecipient (bytes32, already 32 bytes)
    push_slot(&mut data, &dest_domain.to_be_bytes()); // destinationDomain (uint32)
    data.extend_from_slice(&[0u8; 32]); // destinationTokenMessenger (bytes32)
    data.extend_from_slice(&[0u8; 32]); // destinationCaller (bytes32)

    RawLog {
        address: "0xBd3fa81B58Ba92a82136038B25aDec7066af3155".to_string(),
        topics: vec![
            format!("{}", DepositForBurn::SIGNATURE_HASH),
            format!("0x{}", alloy::primitives::hex::encode(t_nonce)),
            format!("0x{}", alloy::primitives::hex::encode(t_burn_token)),
            format!("0x{}", alloy::primitives::hex::encode(t_depositor)),
        ],
        data,
        block_number: Some(22_000_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

#[test]
fn parses_valid_deposit_for_burn() {
    let burn_token = [0xAA; 20];
    let depositor = [0xBB; 20];
    let mut mint_recipient = [0u8; 32];
    mint_recipient[12..].copy_from_slice(&[0xCC; 20]); // EVM: right-aligned in bytes32

    let log = make_log(42, burn_token, depositor, 1_000_000_000, mint_recipient, 3);
    let event = Cctp.parse_event(&log).expect("should parse DepositForBurn");

    assert_eq!(event.protocol_id, "cctp");
    assert_eq!(event.source_chain, chain::ethereum());
    assert_eq!(event.dest_chain, Some(chain::arbitrum())); // domain 3
    assert_eq!(event.correlation_id, "cctp:ethereum:42");
    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "bb".repeat(20))
    );
    assert_eq!(
        event.receiver_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "cc".repeat(20)))
    );
    assert_eq!(event.amount, Some("1000000000".to_string()));
    assert_eq!(
        event.token_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "aa".repeat(20)))
    );
    assert_eq!(event.metadata["nonce"], 42);
    assert_eq!(event.metadata["destination_domain"], 3);
}

#[test]
fn rejects_wrong_topic() {
    let mut log = make_log(1, [0u8; 20], [0u8; 20], 100, [0u8; 32], 3);
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Cctp.parse_event(&log).is_none());
}

#[test]
fn unknown_dest_domain_gives_none_dest_chain() {
    let log = make_log(1, [0u8; 20], [0u8; 20], 100, [0u8; 32], 99);
    let event = Cctp.parse_event(&log).expect("should still parse");
    assert!(event.dest_chain.is_none());
}

#[test]
fn correlation_id_scoped_to_source_chain() {
    // Same nonce on different source chains must produce different correlation IDs.
    let log_eth = make_log(1, [0u8; 20], [0u8; 20], 100, [0u8; 32], 3);
    let mut log_arb = make_log(1, [0u8; 20], [0u8; 20], 100, [0u8; 32], 3);
    log_arb.chain_id = chain::arbitrum();

    let id_eth = Cctp.parse_event(&log_eth).unwrap().correlation_id;
    let id_arb = Cctp.parse_event(&log_arb).unwrap().correlation_id;
    assert_ne!(id_eth, id_arb);
    assert_eq!(id_eth, "cctp:ethereum:1");
    assert_eq!(id_arb, "cctp:arbitrum:1");
}
