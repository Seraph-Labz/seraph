use alloy::primitives::{B256, U256};
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── Event signature ───────────────────────────────────────────────────────────
//
// Across V3 SpokePool emits V3FundsDeposited on the source chain for every
// deposit. Three params are indexed (stored in topics); the rest live in data.
//
// Indexed → topics[1..3]:
//   destinationChainId — EVM chain ID of the target chain
//   depositId          — monotonic uint32 per SpokePool, reset on each deploy
//   depositor          — address that called deposit()
//
// Non-indexed → ABI-encoded data field:
//   inputToken, outputToken, inputAmount, outputAmount,
//   quoteTimestamp, fillDeadline, exclusivityDeadline,
//   recipient, exclusiveRelayer, message
//
// Source: https://github.com/across-protocol/contracts/blob/master/contracts/interfaces/SpokePoolInterface.sol
sol! {
    event V3FundsDeposited(
        address inputToken,
        address outputToken,
        uint256 inputAmount,
        uint256 outputAmount,
        uint256 indexed destinationChainId,
        uint32 indexed depositId,
        uint32 quoteTimestamp,
        uint32 fillDeadline,
        uint32 exclusivityDeadline,
        address indexed depositor,
        address recipient,
        address exclusiveRelayer,
        bytes message
    );
}

pub struct Across;

impl Across {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        // ── Step 1: signature check ───────────────────────────────────────────
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&V3FundsDeposited::SIGNATURE_HASH) {
            return None;
        }

        // ── Step 2: decode indexed + non-indexed params in one call ───────────
        //
        // Unlike PacketSent (which had no indexed params), this event splits
        // fields across topics and data. decode_raw_log merges both automatically:
        // topics[1] → destinationChainId, topics[2] → depositId,
        // topics[3] → depositor; everything else comes from the data bytes.
        let decoded = V3FundsDeposited::decode_raw_log(topics, &log.data).ok()?;

        // ── Step 3: build correlation_id ──────────────────────────────────────
        //
        // depositId is per-SpokePool, not globally unique. Two chains can both
        // have a depositId of 1. Prefixing with the source chain makes it unique
        // without needing a secondary join key in the stitcher.
        let correlation_id = format!("across:{}:{}", log.chain_id, decoded.depositId);

        // ── Step 4: map EVM chainId → internal ChainId ───────────────────────
        //
        // destinationChainId in the event is the raw EVM chain ID (uint256).
        // We convert it to our string-keyed ChainId for storage.
        let dest_chain = evm_chain_id_to_chain(decoded.destinationChainId);

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: decoded.depositor.to_string(),
            receiver_address: Some(decoded.recipient.to_string()),
            // inputAmount is what the user sends; outputAmount is what the
            // recipient receives (slightly less due to relayer fee).
            // We store inputAmount as the canonical transfer amount.
            amount: Some(decoded.inputAmount.to_string()),
            token_address: Some(decoded.inputToken.to_string()),
            protocol_id: "across".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "deposit_id":    decoded.depositId,
                "output_token":  decoded.outputToken.to_string(),
                "output_amount": decoded.outputAmount.to_string(),
                "fill_deadline": decoded.fillDeadline,
                "quote_timestamp": decoded.quoteTimestamp,
                "exclusive_relayer": decoded.exclusiveRelayer.to_string(),
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

// ── EVM chain ID → internal ChainId ──────────────────────────────────────────
//
// `destinationChainId` is a raw EVM chain ID (uint256 in the event but always
// fits in u32 for the chains we support). We truncate to u64 safely — no real
// chain ID is anywhere near u64::MAX.
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
#[path = "tests/across.rs"]
mod tests;
