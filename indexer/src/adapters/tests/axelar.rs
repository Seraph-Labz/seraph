use super::*;
use seraph_shared::types::chain;

// ContractCall: 2 indexed params, 3 non-indexed (all dynamic strings/bytes).
//
// Topics: [SIGNATURE_HASH, sender(address), payloadHash(bytes32)]
// Data ABI layout:
//   head (3 × 32 = 96 bytes):
//     slot 0: offset to destinationChain  = 96
//     slot 1: offset to destinationContractAddress
//     slot 2: offset to payload
//   tails: each dynamic field as (length, bytes-padded-to-32-multiple)
fn make_log(
    sender: [u8; 20],
    payload_hash: [u8; 32],
    dest_chain_str: &str,
    dest_contract: &str,
) -> RawLog {
    let mut t_sender = [0u8; 32];
    t_sender[12..].copy_from_slice(&sender);

    let chain_bytes = dest_chain_str.as_bytes();
    let contract_bytes = dest_contract.as_bytes();
    let payload_bytes: &[u8] = &[];

    // Encode one dynamic field as (length, padded-bytes)
    let encode_dyn = |s: &[u8]| -> Vec<u8> {
        let n = s.len();
        let padded = (n + 31) & !31;
        let mut out = Vec::with_capacity(32 + padded);
        let mut len_slot = [0u8; 32];
        len_slot[24..].copy_from_slice(&(n as u64).to_be_bytes());
        out.extend_from_slice(&len_slot);
        out.extend_from_slice(s);
        out.extend_from_slice(&vec![0u8; padded - n]);
        out
    };

    let enc_chain = encode_dyn(chain_bytes);
    let enc_contract = encode_dyn(contract_bytes);
    let enc_payload = encode_dyn(payload_bytes);

    let head_size: u64 = 3 * 32;
    let offset_chain = head_size;
    let offset_contract = head_size + enc_chain.len() as u64;
    let offset_payload = offset_contract + enc_contract.len() as u64;

    let mut data = Vec::<u8>::new();
    let mut u256 = |val: u64| {
        let mut s = [0u8; 32];
        s[24..].copy_from_slice(&val.to_be_bytes());
        data.extend_from_slice(&s);
    };

    u256(offset_chain);
    u256(offset_contract);
    u256(offset_payload);
    data.extend_from_slice(&enc_chain);
    data.extend_from_slice(&enc_contract);
    data.extend_from_slice(&enc_payload);

    RawLog {
        address: "0x4F4495243837681061C4743b74B3eEdf548D56A5".to_string(),
        topics: vec![
            format!("{}", ContractCall::SIGNATURE_HASH),
            format!("0x{}", alloy::primitives::hex::encode(t_sender)),
            format!("0x{}", alloy::primitives::hex::encode(payload_hash)),
        ],
        data,
        block_number: Some(22_000_000),
        tx_hash: Some("0xdeadbeef".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

#[test]
fn parses_valid_contract_call() {
    let sender = [0xAA; 20];
    let payload_hash = [0xBB; 32];
    let dest_contract = "0xDeadDeAddeAddEAddeadDEaDDEAdDeaDDeAD0000";

    let log = make_log(sender, payload_hash, "arbitrum", dest_contract);
    let event = Axelar.parse_event(&log).expect("should parse ContractCall");

    assert_eq!(event.protocol_id, "axelar");
    assert_eq!(event.source_chain, chain::ethereum());
    assert_eq!(event.dest_chain, Some(chain::arbitrum()));
    assert_eq!(event.correlation_id, format!("0x{}", "bb".repeat(32)));
    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "aa".repeat(20))
    );
    assert_eq!(event.receiver_address.as_deref(), Some(dest_contract));
    assert!(event.amount.is_none());
    assert_eq!(event.metadata["destination_chain"], "arbitrum");
}

#[test]
fn rejects_wrong_topic() {
    let mut log = make_log([0u8; 20], [0u8; 32], "ethereum", "0x0");
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Axelar.parse_event(&log).is_none());
}

#[test]
fn unknown_dest_chain_name_gives_none() {
    let log = make_log([0u8; 20], [0u8; 32], "solana-mainnet-beta", "someaddress");
    let event = Axelar.parse_event(&log).expect("should still parse");
    assert!(event.dest_chain.is_none());
}

#[test]
fn payload_hash_is_correlation_id() {
    let hash = [0xCC; 32];
    let log = make_log([0u8; 20], hash, "ethereum", "0x0");
    let event = Axelar.parse_event(&log).unwrap();
    assert_eq!(event.correlation_id, format!("0x{}", "cc".repeat(32)));
}
