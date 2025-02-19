use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub async fn init_tip_accounts() -> Result<()> {
    Ok(())
}

pub async fn get_tip_account() -> Result<Pubkey> {
    Ok(Pubkey::from_str("11111111111111111111111111111111").unwrap())
}

pub async fn get_tip_value() -> Result<f64> {
    Ok(0.004)
} 