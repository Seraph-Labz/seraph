use alloy::primitives::{Address, B256};
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── Event signature ───────────────────────────────────────────────────────────
//
// CCTP V2 (Circle's Cross-Chain Transfer Protocol) burns USDC/USYC on the
// source chain and mints it natively on the destination. The TokenMessengerV2
// contract is deployed at the same address on every supported chain.
//
// Indexed → topics[1..3]:
//   nonce       — monotonically increasing uint64 per source domain
//   burnToken   — the token being burned (USDC or USYC)
//   depositor   — address that initiated the burn
//
// Non-indexed → data:
//   amount, mintRecipient (bytes32), destinationDomain,
//   destinationTokenMessenger, destinationCaller
//
// Source: https://developers.circle.com/stablecoins/cctp-technical-reference
sol! {
    event DepositForBurn(
        uint64 indexed nonce,
        address indexed burnToken,
        uint256 amount,
        address indexed depositor,
        bytes32 mintRecipient,
        uint32 destinationDomain,
        bytes32 destinationTokenMessenger,
        bytes32 destinationCaller
    );
}

pub struct Cctp;

impl Cctp {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&DepositForBurn::SIGNATURE_HASH) {
            return None;
        }

        let decoded = DepositForBurn::decode_raw_log(topics, &log.data).ok()?;

        // nonce is unique per source domain, not globally unique.
        // Scoping it to the source chain makes it a safe stitcher join key.
        let correlation_id = format!("cctp:{}:{}", log.chain_id, decoded.nonce);

        // mintRecipient is bytes32: EVM addresses are right-aligned (last 20 bytes).
        let receiver = Address::from_slice(&decoded.mintRecipient.0[12..]).to_string();

        let dest_chain = cctp_domain_to_chain(decoded.destinationDomain);

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: decoded.depositor.to_string(),
            receiver_address: Some(receiver),
            amount: Some(decoded.amount.to_string()),
            token_address: Some(decoded.burnToken.to_string()),
            protocol_id: "cctp".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "nonce":              decoded.nonce,
                "destination_domain": decoded.destinationDomain,
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

// CCTP domain ID → internal ChainId.
// Domains are assigned by Circle, distinct from EVM chain IDs.
// Source: https://developers.circle.com/stablecoins/docs/supported-domains
fn cctp_domain_to_chain(domain: u32) -> Option<ChainId> {
    match domain {
        0 => Some(chain::ethereum()),
        1 => Some(chain::avalanche()),
        2 => Some(chain::optimism()),
        3 => Some(chain::arbitrum()),
        6 => Some(chain::base()),
        7 => Some(chain::polygon()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/cctp.rs"]
mod tests;
