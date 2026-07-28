use alloy::primitives::{B256, U256};
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── Event signature ───────────────────────────────────────────────────────────
//
// Hop Protocol bridges tokens across rollups by running AMM liquidity pools
// (hTokens) on each chain. TransferSent is emitted by the L2_Bridge contract
// when a user initiates a transfer from an L2 to another chain.
//
// Indexed → topics[1..3]:
//   transferId — bytes32 hash uniquely identifying this transfer
//   chainId    — raw EVM chain ID of the destination
//   recipient  — the address that will receive funds on the destination
//
// Non-indexed → data:
//   amount, transferNonce, bonderFee, amountOutMin, deadline
//
// Limitation: TransferSent does not include the sender address — that is
// msg.sender of the originating transaction, which is not stored in the log.
// We leave sender_address as an empty string; a future RawLog extension to
// carry tx.from would allow us to populate it.
//
// Note: Ethereum L1 uses a different event (TransferSentToL2) with no
// transferId. This adapter covers only the L2-source case.
//
// Source: https://github.com/hop-protocol/contracts/blob/master/contracts/bridges/L2_Bridge.sol
sol! {
    event TransferSent(
        bytes32 indexed transferId,
        uint256 indexed chainId,
        address indexed recipient,
        uint256 amount,
        bytes32 transferNonce,
        uint256 bonderFee,
        uint256 amountOutMin,
        uint256 deadline
    );
}

pub struct Hop;

impl Hop {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&TransferSent::SIGNATURE_HASH) {
            return None;
        }

        let decoded = TransferSent::decode_raw_log(topics, &log.data).ok()?;

        let correlation_id = decoded.transferId.to_string();
        let dest_chain = evm_chain_id_to_chain(decoded.chainId);

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: String::new(), // not emitted — see module comment
            receiver_address: Some(decoded.recipient.to_string()),
            amount: Some(decoded.amount.to_string()),
            token_address: None,
            protocol_id: "hop".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "bonder_fee":    decoded.bonderFee.to_string(),
                "amount_out_min": decoded.amountOutMin.to_string(),
                "deadline":      decoded.deadline.to_string(),
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

// Raw EVM chain ID → internal ChainId (same table as across.rs).
fn evm_chain_id_to_chain(chain_id: U256) -> Option<ChainId> {
    match chain_id.to::<u64>() {
        1 => Some(chain::ethereum()),
        42161 => Some(chain::arbitrum()),
        10 => Some(chain::optimism()),
        8453 => Some(chain::base()),
        137 => Some(chain::polygon()),
        56 => Some(chain::bsc()),
        43114 => Some(chain::avalanche()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/hop.rs"]
mod tests;
