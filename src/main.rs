use {
    dotenvy::dotenv,
    crate::{
        common::{
            logger::Logger,
            utils::{create_nonblocking_rpc_client, create_rpc_client, import_env_var, import_wallet, AppState},
        },
        dex::pump_fun::Pump,
    },
    anyhow::Result,
    std::sync::Arc,
};

mod common;
mod dex;
mod engine;
mod services;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment
    dotenv().ok();
    
    // Setup logging
    let logger = Logger::new("[MAIN] => ".to_string());
    logger.log("Starting PumpFun copy trading bot...".to_string());

    // Initialize clients
    let rpc_client = create_rpc_client()?;
    let rpc_nonblocking_client = create_nonblocking_rpc_client().await?;
    let wallet = import_wallet()?;

    // Initialize Pump instance
    let pump = Pump::new(
        Arc::new(rpc_nonblocking_client),
        wallet,
    );

    // TODO: Implement main trading loop
    logger.log("Trading bot initialized and ready".to_string());

    Ok(())
}
