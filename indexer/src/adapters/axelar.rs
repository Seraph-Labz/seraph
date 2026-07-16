use alloy::primitives::B256;
use alloy::sol;
use alloy::sol_types::SolEvent;
use chrono::Utc;
use uuid::Uuid;

use seraph_shared::{ChainId, CrossChainEvent, RawLog, TxStatus, types::chain};

// ── Event signature ───────────────────────────────────────────────────────────
//
// Axelar is a proof-of-stake network that relays general-purpose cross-chain
// messages (GMP). ContractCall is emitted by the AxelarGateway contract when
// a dApp calls callContract() to send a message to another chain.
//
// Indexed → topics[1..2]:
//   sender      — the contract that initiated the call
//   payloadHash — keccak256 of the payload bytes
//
// Non-indexed → data:
//   destinationChain           — Axelar chain name (e.g. "ethereum", "arbitrum")
//   destinationContractAddress — the target contract on the destination chain
//   payload                    — arbitrary bytes passed to the target
//
// payloadHash is the correlation key: Axelar validators sign over it, and the
// Approved event on the destination gateway carries the same hash.
//
// Note: ContractCallWithToken (which includes asset + amount) is a separate
// event and is not handled by this adapter. It can be added in a follow-up.
//
// Source: https://github.com/axelarnetwork/axelar-cgp-solidity/blob/main/contracts/interfaces/IAxelarGateway.sol
sol! {
    event ContractCall(
        address indexed sender,
        string destinationChain,
        string destinationContractAddress,
        bytes32 indexed payloadHash,
        bytes payload
    );
}

pub struct Axelar;

impl Axelar {
    pub fn parse_event(&self, log: &RawLog) -> Option<CrossChainEvent> {
        let topics: Vec<B256> = log
            .topics
            .iter()
            .filter_map(|t| t.parse::<B256>().ok())
            .collect();

        if topics.first() != Some(&ContractCall::SIGNATURE_HASH) {
            return None;
        }

        let decoded = ContractCall::decode_raw_log(topics, &log.data).ok()?;

        let correlation_id = decoded.payloadHash.to_string();
        let dest_chain = axelar_chain_to_chain(&decoded.destinationChain);

        let now = Utc::now();

        Some(CrossChainEvent {
            id: Uuid::new_v4(),
            source_tx_hash: log.tx_hash.clone().unwrap_or_default(),
            source_chain: log.chain_id.clone(),
            dest_chain,
            sender_address: decoded.sender.to_string(),
            // destinationContractAddress is the raw string from the event;
            // on EVM chains it is a checksummed address, on Cosmos it may be
            // a bech32 address. We store it as-is.
            receiver_address: Some(decoded.destinationContractAddress.clone()),
            amount: None,
            token_address: None,
            protocol_id: "axelar".to_string(),
            correlation_id,
            status: TxStatus::Pending,
            metadata: serde_json::json!({
                "destination_chain":    decoded.destinationChain,
                "destination_contract": decoded.destinationContractAddress,
                "payload_len":          decoded.payload.len(),
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

// Axelar uses its own chain name strings.  "binance" maps to our "bsc".
// Source: https://docs.axelar.dev/dev/reference/mainnet-chain-names
fn axelar_chain_to_chain(name: &str) -> Option<ChainId> {
    match name {
        "ethereum" | "Ethereum" => Some(chain::ethereum()),
        "arbitrum" | "arbitrum-sepolia" => Some(chain::arbitrum()),
        "optimism" | "optimism-sepolia" => Some(chain::optimism()),
        "base" | "base-sepolia" => Some(chain::base()),
        "polygon" | "Polygon" => Some(chain::polygon()),
        "binance" | "binance-sepolia" => Some(chain::bsc()),
        "avalanche" | "Avalanche" => Some(chain::avalanche()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/axelar.rs"]
mod tests;
