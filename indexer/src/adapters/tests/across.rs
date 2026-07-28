use super::*;
use alloy::primitives::U256;
use seraph_shared::types::chain;

// Parameters for a synthetic V3FundsDeposited log.
struct DepositParams {
    input_token: [u8; 20],
    output_token: [u8; 20],
    input_amount: u128,
    output_amount: u128,
    destination_chain_id: u64,
    deposit_id: u32,
    quote_timestamp: u32,
    fill_deadline: u32,
    exclusivity_deadline: u32,
    depositor: [u8; 20],
    recipient: [u8; 20],
    exclusive_relayer: [u8; 20],
}

// Build a RawLog that mirrors what alloy produces when indexing a real
// V3FundsDeposited event.
//
// Topics layout (4 topics total):
//   [0] V3FundsDeposited::SIGNATURE_HASH
//   [1] destinationChainId — uint256, left-padded to 32 bytes
//   [2] depositId          — uint32, left-padded to 32 bytes
//   [3] depositor          — address, left-padded to 32 bytes
//
// Data layout — ABI-encoded non-indexed params (9 static + 1 dynamic):
//   slot  0: inputToken        (address, right-aligned)
//   slot  1: outputToken       (address, right-aligned)
//   slot  2: inputAmount       (uint256)
//   slot  3: outputAmount      (uint256)
//   slot  4: quoteTimestamp    (uint32, right-aligned)
//   slot  5: fillDeadline      (uint32, right-aligned)
//   slot  6: exclusivityDeadline (uint32, right-aligned)
//   slot  7: recipient         (address, right-aligned)
//   slot  8: exclusiveRelayer  (address, right-aligned)
//   slot  9: offset to message (= 10 * 32 = 320)
//   slot 10: message length    (= 0, empty message)
fn make_log(p: &DepositParams) -> RawLog {
    // ── topics ────────────────────────────────────────────────────────────────
    let mut topic1 = [0u8; 32];
    topic1[24..].copy_from_slice(&p.destination_chain_id.to_be_bytes());

    let mut topic2 = [0u8; 32];
    topic2[28..].copy_from_slice(&p.deposit_id.to_be_bytes());

    let mut topic3 = [0u8; 32];
    topic3[12..].copy_from_slice(&p.depositor);

    let topics = vec![
        format!("{}", V3FundsDeposited::SIGNATURE_HASH),
        format!("0x{}", hex::encode(topic1)),
        format!("0x{}", hex::encode(topic2)),
        format!("0x{}", hex::encode(topic3)),
    ];

    // ── data ─────────────────────────────────────────────────────────────────
    let mut data = Vec::<u8>::new();

    let mut slot = |bytes: &[u8]| {
        let mut s = [0u8; 32];
        let offset = 32 - bytes.len();
        s[offset..].copy_from_slice(bytes);
        data.extend_from_slice(&s);
    };

    slot(&p.input_token); // inputToken (address)
    slot(&p.output_token); // outputToken (address)
    slot(&p.input_amount.to_be_bytes()); // inputAmount (uint256 low 128 bits)
    slot(&p.output_amount.to_be_bytes()); // outputAmount

    let mut u32_bytes = [0u8; 4];

    u32_bytes.copy_from_slice(&p.quote_timestamp.to_be_bytes());
    slot(&u32_bytes); // quoteTimestamp

    u32_bytes.copy_from_slice(&p.fill_deadline.to_be_bytes());
    slot(&u32_bytes); // fillDeadline

    u32_bytes.copy_from_slice(&p.exclusivity_deadline.to_be_bytes());
    slot(&u32_bytes); // exclusivityDeadline

    slot(&p.recipient); // recipient (address)
    slot(&p.exclusive_relayer); // exclusiveRelayer (address)

    // offset to message tail = 10 slots * 32 bytes = 320
    slot(&320u64.to_be_bytes()); // message offset

    // message tail: length = 0 (empty message)
    data.extend_from_slice(&[0u8; 32]);

    RawLog {
        address: "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5".to_string(),
        topics,
        data,
        block_number: Some(21_000_000),
        tx_hash: Some("0xcafebabe".to_string()),
        log_index: Some(0),
        chain_id: chain::ethereum(),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

// hex::encode is from the `hex` crate but alloy re-exports it as alloy::hex::encode.
// To avoid adding a direct dep we use the alloy re-export.
mod hex {
    pub fn encode(b: impl AsRef<[u8]>) -> String {
        alloy::primitives::hex::encode(b)
    }
}

fn default_params() -> DepositParams {
    DepositParams {
        input_token: [0x11; 20],
        output_token: [0x22; 20],
        input_amount: 1_000_000, // 1 USDC (6 decimals)
        output_amount: 999_000,
        destination_chain_id: 42161, // Arbitrum
        deposit_id: 7,
        quote_timestamp: 1_700_000_000,
        fill_deadline: 1_700_003_600,
        exclusivity_deadline: 0,
        depositor: [0xDD; 20],
        recipient: [0xEE; 20],
        exclusive_relayer: [0x00; 20],
    }
}

// ── Test 1: happy path ────────────────────────────────────────────────────────
#[test]
fn parses_valid_deposit() {
    let p = default_params();
    let log = make_log(&p);
    let event = Across
        .parse_event(&log)
        .expect("should parse V3FundsDeposited");

    assert_eq!(event.protocol_id, "across");
    assert_eq!(event.source_chain, chain::ethereum());
    assert_eq!(event.dest_chain, Some(chain::arbitrum()));

    assert_eq!(
        event.sender_address.to_lowercase(),
        format!("0x{}", "dd".repeat(20))
    );
    assert_eq!(
        event.receiver_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "ee".repeat(20)))
    );

    assert_eq!(event.amount, Some("1000000".to_string()));
    assert_eq!(
        event.token_address.as_deref().map(str::to_lowercase),
        Some(format!("0x{}", "11".repeat(20)))
    );

    // correlation_id must scope depositId to the source chain
    assert_eq!(event.correlation_id, "across:ethereum:7");
    assert_eq!(event.status, TxStatus::Pending);
    assert_eq!(event.metadata["deposit_id"], 7);
    assert_eq!(event.metadata["fill_deadline"], 1_700_003_600u64);
}

// ── Test 2: wrong topic ───────────────────────────────────────────────────────
#[test]
fn rejects_wrong_topic() {
    let mut log = make_log(&default_params());
    log.topics[0] =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(Across.parse_event(&log).is_none());
}

// ── Test 3: unknown destination chain ────────────────────────────────────────
#[test]
fn unknown_chain_id_gives_none_dest_chain() {
    let mut p = default_params();
    p.destination_chain_id = 999_999;
    let event = Across
        .parse_event(&make_log(&p))
        .expect("should still parse with unknown chain");
    assert!(event.dest_chain.is_none());
}

// ── Test 4: correlation_id scopes depositId to source chain ──────────────────
// Two deposits with the same depositId on different source chains must produce
// different correlation IDs so the stitcher never cross-joins them.
#[test]
fn correlation_id_includes_source_chain() {
    let p = default_params(); // depositId = 7, source = ethereum

    let mut log_eth = make_log(&p);
    log_eth.chain_id = chain::ethereum();

    let mut log_arb = make_log(&p);
    log_arb.chain_id = chain::arbitrum();
    // Make it look like an Arbitrum SpokePool address
    log_arb.address = "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A".to_string();

    let event_eth = Across.parse_event(&log_eth).unwrap();
    let event_arb = Across.parse_event(&log_arb).unwrap();

    assert_ne!(event_eth.correlation_id, event_arb.correlation_id);
    assert_eq!(event_eth.correlation_id, "across:ethereum:7");
    assert_eq!(event_arb.correlation_id, "across:arbitrum:7");
}

// ── Test 5: large amounts don't overflow ──────────────────────────────────────
#[test]
fn handles_large_amount() {
    let mut p = default_params();
    p.input_amount = u128::MAX; // 340_282_366_920_938_463_463_374_607_431_768_211_455
    let event = Across
        .parse_event(&make_log(&p))
        .expect("should parse large amount");
    assert_eq!(event.amount, Some(U256::from(u128::MAX).to_string()));
}
