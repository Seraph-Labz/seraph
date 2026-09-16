use super::all_addresses;
use seraph_shared::chain;

/// Every chain the indexer spawns a runner for must resolve to at least one
/// watched contract. A chain that resolves to none would be skipped at startup,
/// silently going unindexed.
#[test]
fn every_active_chain_has_watched_contracts() {
    for chain_id in [
        chain::ETHEREUM,
        chain::ARBITRUM,
        chain::OPTIMISM,
        chain::BASE,
        chain::POLYGON,
        chain::BSC,
        chain::AVALANCHE,
    ] {
        assert!(
            !all_addresses(chain_id).is_empty(),
            "{chain_id} resolves to no watched contracts"
        );
    }
}

#[test]
fn unknown_chain_resolves_to_no_contracts() {
    assert!(all_addresses("not-a-chain").is_empty());
}

/// The same address listed twice would be sent twice in the eth_getLogs filter.
#[test]
fn a_chain_lists_no_duplicate_addresses() {
    for chain_id in [
        chain::ETHEREUM,
        chain::ARBITRUM,
        chain::OPTIMISM,
        chain::BASE,
        chain::POLYGON,
        chain::BSC,
        chain::AVALANCHE,
    ] {
        let addrs = all_addresses(chain_id);
        let mut deduped = addrs.clone();
        deduped.sort();
        deduped.dedup();

        assert_eq!(
            addrs.len(),
            deduped.len(),
            "{chain_id} lists a duplicate contract address"
        );
    }
}
