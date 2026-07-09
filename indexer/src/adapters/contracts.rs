#![allow(dead_code)]

use alloy::primitives::Address;
use seraph_shared::chain;

/// Returns the contract addresses to watch for a given protocol on a given chain.
///
/// The ChainRunner feeds these into the eth_getLogs filter so we only pull
/// relevant logs instead of every log on the chain. Returns an empty vec if the
/// protocol is not deployed on that chain — the runner skips it silently.
///
/// Sources: each protocol's official deployment docs, verified against on-chain
/// explorers (Etherscan, Arbiscan, etc.).
fn parse(addr: &str) -> Address {
    addr.parse().expect("hardcoded address is valid")
}

pub fn layerzero_v2(chain_id: &str) -> Vec<Address> {
    // EndpointV2 is deployed at the same address on every EVM chain — a
    // deliberate design choice by LayerZero for consistent cross-chain addressing.
    // Source: https://etherscan.io/address/0x1a44076050125825900e736c501f859c50fE728c
    let addr = parse("0x1a44076050125825900e736c501f859c50fE728c");
    match chain_id {
        chain::ETHEREUM
        | chain::ARBITRUM
        | chain::OPTIMISM
        | chain::BASE
        | chain::POLYGON
        | chain::BSC
        | chain::AVALANCHE => vec![addr],
        _ => vec![],
    }
}

pub fn across(chain_id: &str) -> Vec<Address> {
    // Across V3 SpokePool — one contract per chain.
    // Across is L2-focused; Avalanche is not deployed.
    // Source: https://docs.across.to/reference/contract-addresses
    let addr = match chain_id {
        chain::ETHEREUM => "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5",
        chain::ARBITRUM => "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A",
        chain::OPTIMISM => "0x6f26Bf09B1C792e3228e5467807a900A503c0281",
        chain::BASE => "0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64",
        chain::POLYGON => "0x9295ee1d8C5b022Be115A2AD3c30C72E34e7F096",
        chain::BSC => "0x4e8E101924eDE233C13e2D8622DC8aED2872d505",
        _ => return vec![],
    };
    vec![parse(addr)]
}

pub fn stargate(chain_id: &str) -> Vec<Address> {
    // Stargate V2 — each token is its own pool contract, all emitting OFTSent.
    // We watch every pool per chain so no token is missed.
    // Source: https://stargateprotocol.gitbook.io/stargate/v2-developer-docs/technical-reference/mainnet-contracts
    let addrs: &[&str] = match chain_id {
        chain::ETHEREUM => &[
            "0x77b2043768d28E9C9aB44E1aBfC95944bcE57931", // ETH (Native)
            "0xc026395860Db2d07ee33e05fE50ed7bD583189C7", // USDC
            "0x933597a323Eb81cAe705C5bC29985172fd5A3973", // USDT
            "0x268Ca24DAefF1FaC2ed883c598200CcbB79E931D", // mETH
        ],
        chain::ARBITRUM => &[
            "0xA45B5130f36CDcA45667738e2a258AB09f4A5f7F", // ETH (Native)
            "0xe8CDF27AcD73a434D661C84887215F7598e7d0d3", // USDC
            "0xcE8CcA271Ebc0533920C83d39F417ED6A0abB7D0", // USDT
        ],
        chain::OPTIMISM => &[
            "0xe8CDF27AcD73a434D661C84887215F7598e7d0d3", // ETH (Native)
            "0xcE8CcA271Ebc0533920C83d39F417ED6A0abB7D0", // USDC
            "0x19cFCE47eD54a88614648DC3f19A5980097007dD", // USDT
        ],
        chain::BASE => &[
            "0xdc181Bd607330aeeBEF6ea62e03e5e1Fb4B6F7C7", // ETH (Native)
            "0x27a16dc786820B16E5c9028b75B99F6f604b5d26", // USDC
        ],
        chain::POLYGON => &[
            "0x9Aa02D4Fae7F58b8E8f34c66E756cC734DAc7fe4", // USDC
            "0xd47b03ee6d86Cf251ee7860FB2ACf9f91B9fD4d7", // USDT
        ],
        chain::BSC => &[
            "0x962Bd449E630b0d928f308Ce63f1A21F02576057", // USDC
            "0x138EB30f73BC423c6455C53df6D89CB01d9eBc63", // USDT
        ],
        chain::AVALANCHE => &[
            "0x5634c4a5FEd09819E3c46D86A965Dd9447d86e47", // USDC
            "0x12dC9256Acc9895B076f6638D628382881e62CeE", // USDT
        ],
        _ => return vec![],
    };
    addrs.iter().map(|a| parse(a)).collect()
}

pub fn cctp(chain_id: &str) -> Vec<Address> {
    // CCTP V2 TokenMessengerV2 — emits DepositForBurn on the source chain.
    // V2 uses the same address on every supported chain (same as LZ V2's pattern).
    // BSC is supported but USYC-only (not USDC) — still a valid CCTP event.
    // Source: https://developers.circle.com/cctp/evm-smart-contracts
    let addr = parse("0x28b5a0e9C621a5BadaA536219b3a228C8168cf5d");
    match chain_id {
        chain::ETHEREUM
        | chain::ARBITRUM
        | chain::OPTIMISM
        | chain::BASE
        | chain::POLYGON
        | chain::AVALANCHE
        | chain::BSC => vec![addr],
        _ => vec![],
    }
}

pub fn hop(chain_id: &str) -> Vec<Address> {
    // Hop Protocol USDC bridge entry points.
    // On Ethereum: L1Bridge. On L2s: L2_AmmWrapper.
    // Hop is rollup-native — not deployed on BSC or Avalanche.
    // Source: https://github.com/hop-protocol/hop/blob/develop/packages/sdk/src/addresses/mainnet.ts
    let addr = match chain_id {
        chain::ETHEREUM => "0x3666f603Cc164936C1b87e207F36BEBa4AC5f18a",
        chain::ARBITRUM => "0xe22D2beDb3Eca35E6397e0C6D62857094aA26F52",
        chain::OPTIMISM => "0x2ad09850b0CA4c7c1B33f5AcD6cBAbCaB5d6e796",
        chain::BASE => "0x7D269D3E0d61A05a0bA976b7DBF8805bF844AF3F",
        chain::POLYGON => "0x76b22b8C1079A44F1211D867D68b1eda76a635A7",
        _ => return vec![],
    };
    vec![parse(addr)]
}

pub fn connext(chain_id: &str) -> Vec<Address> {
    // Connext Diamond Proxy — single entry point per chain for XCalled events.
    // Not deployed on Base or Avalanche.
    // Source: https://docs.connext.network/resources/deployments
    let addr = match chain_id {
        chain::ETHEREUM => "0x8898B472C54c31894e3B9bb83cEA802a5d0e63C6",
        chain::ARBITRUM => "0xEE9deC2712cCE65174B561151701Bf54b99C24C8",
        chain::OPTIMISM => "0x8f7492DE823025b4CfaAB1D34c58963F2af5DEDA",
        chain::BASE => "0xB8448C6f7f7887D36DcA487370778e419e9ebE3F",
        chain::POLYGON => "0x11984dc4465481512eb5b777E44061C158CF2259",
        chain::BSC => "0xCd401c10afa37d641d2F594852DA94C700e4F2CE",
        _ => return vec![],
    };
    vec![parse(addr)]
}

pub fn wormhole(chain_id: &str) -> Vec<Address> {
    // Wormhole Core Bridge — emits LogMessagePublished for every cross-chain
    // message. The VAA sequence number + emitter chain is the correlation anchor.
    // Source: https://wormhole.com/docs/products/reference/contract-addresses/
    let addr = match chain_id {
        chain::ETHEREUM => "0x98f3c9e6E3fAce36bAAd05FE09d375Ef1464288B",
        chain::ARBITRUM => "0xa5f208e072434bC67592E4C49C1B991BA79BCA46",
        chain::OPTIMISM => "0xEe91C335eab126dF5fDB3797EA9d6aD93aeC9722",
        chain::BASE => "0xbebdb6C8ddC678FfA9f8748f85C815C556Dd8ac6",
        chain::POLYGON => "0x7A4B5a56256163F07b2C80A7cA55aBE66c4ec4d7",
        chain::BSC => "0x98f3c9e6E3fAce36bAAd05FE09d375Ef1464288B",
        chain::AVALANCHE => "0x54a8e5f9c4CbA08F9943965859F6c34eAF03E26c",
        _ => return vec![],
    };
    vec![parse(addr)]
}

pub fn axelar(chain_id: &str) -> Vec<Address> {
    // Axelar Gateway — emits ContractCall for GMP and TokenSent for transfers.
    // Arbitrum, Optimism, and Base share the same address (create2 deployment).
    // Source: https://docs.axelar.dev/dev/reference/mainnet-contract-addresses/
    let addr = match chain_id {
        chain::ETHEREUM => "0x4F4495243837681061C4743b74B3eEdf548D56A5",
        chain::ARBITRUM | chain::OPTIMISM | chain::BASE => {
            "0xe432150cce91c13a887f7D836923d5597adD8E31"
        }
        chain::POLYGON => "0x6f015F16De9fC8791b234eF68D486d2bF203FBA8",
        chain::BSC => "0x304acf330bbE08d1e512eefaa92F6a57871fD895",
        chain::AVALANCHE => "0x5029C0EFf6C34351a0CEc334542cDb22c7928f78",
        _ => return vec![],
    };
    vec![parse(addr)]
}

/// Collect all watched addresses across every active protocol for a given chain.
/// Used by ChainRunner in step 2.10 to build the eth_getLogs address filter.
pub fn all_addresses(chain_id: &str) -> Vec<Address> {
    let mut addrs = Vec::new();
    addrs.extend(layerzero_v2(chain_id));
    addrs.extend(across(chain_id));
    addrs.extend(stargate(chain_id));
    addrs.extend(cctp(chain_id));
    addrs.extend(hop(chain_id));
    addrs.extend(connext(chain_id));
    addrs.extend(wormhole(chain_id));
    addrs.extend(axelar(chain_id));
    addrs
}
