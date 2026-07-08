use alloy::providers::{Provider, ProviderBuilder};
use alloy::transports::ws::WsConnect;
use anyhow::Result;
use tracing::info;

/// Connect to an EVM node via WebSocket and confirm with a chain_id call.
pub async fn connect(url: &str) -> Result<impl Provider + Clone + 'static> {
    let provider = ProviderBuilder::new().connect_ws(WsConnect::new(url)).await?;

    let chain_id = provider.get_chain_id().await?;
    info!(chain_id, "connected to EVM node");

    Ok(provider)
}
