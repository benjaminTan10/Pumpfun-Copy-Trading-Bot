use anyhow::Result;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
use std::{env, sync::Arc};

#[derive(Debug, Clone)]
pub struct SwapConfig {
    pub slippage: u64,
    pub use_jito: bool,
    pub amount: u64,
    pub swap_direction: SwapDirection,
}

#[derive(Debug, Clone)]
pub enum SwapDirection {
    Buy,
    Sell,
}

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<solana_client::rpc_client::RpcClient>,
    pub rpc_nonblocking_client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    pub wallet: Arc<Keypair>,
}

pub fn import_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Environment variable {} not set", key))
}

pub fn import_wallet() -> Result<Arc<Keypair>> {
    let priv_key = import_env_var("PRIVATE_KEY");
    let wallet = Keypair::from_base58_string(&priv_key);
    Ok(Arc::new(wallet))
}

pub async fn create_nonblocking_rpc_client() -> Result<solana_client::nonblocking::rpc_client::RpcClient> {
    let rpc_url = import_env_var("RPC_HTTPS");
    Ok(solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ))
}

pub fn create_rpc_client() -> Result<solana_client::rpc_client::RpcClient> {
    let rpc_url = import_env_var("RPC_HTTPS");
    Ok(solana_client::rpc_client::RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ))
}
