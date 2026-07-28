use alloy::primitives::B256;
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── Event signature ───────────────────────────────────────────────────────────
//
// Connext is a modular interoperability protocol. XCalled is emitted on the
// source chain's Connext diamond contract when a user initiates a cross-chain
// call (with or without asset transfer).
//
// Indexed → topics[1..3]:
//   transferId  — bytes32 uniquely identifying this cross-chain transfer
//   nonce       — per-sender monotonic counter
//   messageHash — hash of the cross-chain message body
//
// Non-indexed → data (ABI-encoded):
//   params (TransferInfo struct), asset, amount, local, messageBody
//
// TransferInfo contains the origin/destination domains, the recipient (to),
// the sender (originSender), the bridged amount, and routing metadata.
//
// Source: https://github.com/connext/monorepo/blob/main/packages/deployments/contracts/contracts/core/connext/interfaces/IConnext.sol
sol! {
    struct TransferInfo {
        uint32 originDomain;
        uint32 destinationDomain;
        uint32 canonicalDomain;
        address to;
        address delegate;
        bool receiveLocal;
        bytes callData;
        uint256 slippage;
        address originSender;
        uint256 bridgedAmt;
        uint256 normalizedIn;
        uint256 nonce;
        bytes32 canonicalId;
    }

    event XCalled(
        bytes32 indexed transferId,
        uint256 indexed nonce,
        bytes32 indexed messageHash,
        TransferInfo params,
        address asset,
        uint256 amount,
        address local,
        bytes messageBody
    );
}

pub struct Connext;

impl Connext {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&XCalled::SIGNATURE_HASH) {
            return None;
        }

        let decoded = XCalled::decode_raw_log(topics, &log.data).ok()?;

        let correlation_id = decoded.transferId.to_string();
        let dest_chain = connext_domain_to_chain(decoded.params.destinationDomain);

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: decoded.params.originSender.to_string(),
            receiver_address: Some(decoded.params.to.to_string()),
            amount: Some(decoded.amount.to_string()),
            token_address: Some(decoded.asset.to_string()),
            protocol_id: "connext".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "origin_domain":      decoded.params.originDomain,
                "destination_domain": decoded.params.destinationDomain,
                "slippage":           decoded.params.slippage.to_string(),
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

// Connext assigns its own domain IDs, separate from EVM chain IDs.
// Source: https://docs.connext.network/resources/supported-chains
fn connext_domain_to_chain(domain: u32) -> Option<ChainId> {
    match domain {
        6_648_936 => Some(chain::ethereum()),
        1_634_886_255 => Some(chain::arbitrum()),
        1_869_640_809 => Some(chain::optimism()),
        1_650_553_709 => Some(chain::base()),
        1_886_350_457 => Some(chain::polygon()),
        6_450_786 => Some(chain::bsc()),
        1_635_148_152 => Some(chain::avalanche()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/connext.rs"]
mod tests;
