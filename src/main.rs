use {
    dotenv::dotenv,
    pump_fun_copy_trader::{
        common::{
            logger::Logger,
            utils::{create_nonblocking_rpc_client, create_rpc_client, import_env_var, import_wallet, AppState},
        },
        engine::monitor::wallet_monitor::monitor_wallet,
        services::jito,
    },
};

#[tokio::main]
async fn main() {
    let logger = Logger::new("[INIT] => ".to_string());
    dotenv().ok();

    // Initialize clients and wallet
    let rpc_wss = import_env_var("RPC_WSS");
    let rpc_client = create_rpc_client().unwrap();
    let rpc_nonblocking_client = create_nonblocking_rpc_client().await.unwrap();
    let wallet = import_wallet().unwrap();

    let state = AppState {
        rpc_client: rpc_client.clone(),
        rpc_nonblocking_client: rpc_nonblocking_client.clone(),
        wallet: wallet.clone(),
    };

    let slippage = import_env_var("SLIPPAGE").parse::<u64>().unwrap_or(5);
    let use_jito = true;

    // Initialize Jito for faster transactions
    if use_jito {
        jito::init_tip_accounts().await.unwrap();
    }

    logger.log("Starting PumpFun copy trading bot...".to_string());
    
    // Start monitoring target wallet
    monitor_wallet(&rpc_wss, state, slippage, use_jito).await.unwrap();
}
