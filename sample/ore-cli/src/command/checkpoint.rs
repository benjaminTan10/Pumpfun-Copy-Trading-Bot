use std::{str::FromStr, time::Duration};

use anyhow::Error;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use ore_boost_api::consts::CHECKPOINT_INTERVAL;  
use ore_boost_api::state::{boost_pda, checkpoint_pda};
use solana_rpc_client::spinner;
use colored::*;
use tokio::time::sleep;

use crate::{
    args::CheckpointArgs,
    Miner, 
    utils::{get_clock, get_boost, get_boost_stake_accounts, get_checkpoint, ComputeBudget},
};

const MAX_ACCOUNTS_PER_TX: usize = 10;
const COMPUTE_BUDGET: u32 = 100_000;

impl Miner {
    pub async fn checkpoint(&self, args: CheckpointArgs) -> Result<(), Error> {
        if args.continuous {
            self.checkpoint_continuous(&args).await?;
        } else {
            self.checkpoint_once(&args).await?;
        }
        Ok(())
    }

    async fn checkpoint_continuous(&self, args: &CheckpointArgs) -> Result<(), Error> {
        let mut progress_bar = spinner::new_progress_bar();
        let mint_address = Pubkey::from_str(&args.mint)?;
        let boost_address = boost_pda(mint_address).0;
        let checkpoint_address = checkpoint_pda(boost_address).0;
        loop {
            // Get current time and checkpoint data
            let clock = get_clock(&self.rpc_client).await.expect("Failed to fetch clock account");
            let checkpoint = get_checkpoint(&self.rpc_client, checkpoint_address).await;
            let time_since_last = clock.unix_timestamp - checkpoint.ts;

            // Call checkpoint if needed
            if time_since_last >= CHECKPOINT_INTERVAL {
                progress_bar.finish_and_clear();
                let _ = self.checkpoint_once(args).await;
                progress_bar = spinner::new_progress_bar();
            }

            // Sleep for 60 seconds
            let mins_remaining = (CHECKPOINT_INTERVAL - time_since_last) / 60;
            progress_bar.set_message(format!("Waiting... ({} min remaining)", mins_remaining));
            sleep(Duration::from_secs(60)).await;
        }
    }

    async fn checkpoint_once(&self, args: &CheckpointArgs) -> Result<(), Error> {
        // Parse mint address
        let progress_bar = spinner::new_progress_bar();
        let mint_address = Pubkey::from_str(&args.mint)?;
        let boost_address = boost_pda(mint_address).0;
        let checkpoint_address = checkpoint_pda(boost_address).0;

        // Get boost account data
        let _boost = get_boost(&self.rpc_client, boost_address).await;
        let checkpoint = get_checkpoint(&self.rpc_client, checkpoint_address).await;

        // TODO Check if enough time has passed since last checkpoint
        let clock = get_clock(&self.rpc_client).await.expect("Failed to fetch clock account");
        let time_since_last = clock.unix_timestamp - checkpoint.ts;
        if time_since_last < CHECKPOINT_INTERVAL {
            progress_bar.finish_with_message(format!(
                "{} Not enough time has passed since last checkpoint. Wait {} more seconds.",
                "WARNING".yellow().bold(),
                CHECKPOINT_INTERVAL - time_since_last
            ));
            return Ok(());
        }

        // Get all stake accounts for this boost
        let mut accounts = get_boost_stake_accounts(&self.rpc_client, boost_address).await?;
        if accounts.is_empty() {
            progress_bar.finish_with_message("No stake accounts found for this boost");
            return Ok(());
        }

        // Sort accounts by stake ID
        accounts.sort_by(|(_, stake_a), (_, stake_b)| {
            stake_a.id.cmp(&stake_b.id)
        });

        // Filter accounts starting from checkpoint.current_id
        let remaining_accounts: Vec<_> = accounts
            .into_iter()
            .filter(|(_, stake)| stake.id >= checkpoint.current_id)
            .collect();

        // Pack instructions for rebase
        let mut ixs = Vec::new();            
        if remaining_accounts.is_empty() {
            // If total stakers is zero, use default account
            ixs.push(ore_boost_api::sdk::rebase(
                self.signer().pubkey(),
                mint_address,
                Pubkey::default(),
            ));
            progress_bar.finish_and_clear();
            let _ = self.send_and_confirm(&ixs, ComputeBudget::Fixed(COMPUTE_BUDGET), false)
                .await?;
        } else {
            // Chunk stake accounts into batches
            let chunks = remaining_accounts.chunks(MAX_ACCOUNTS_PER_TX);
            for chunk in chunks {
                ixs.clear();
                for (stake_pubkey, _stake) in chunk {
                    ixs.push(ore_boost_api::sdk::rebase(
                        self.signer().pubkey(),
                        mint_address,
                        *stake_pubkey,
                    ));
                }
                if !ixs.is_empty() {
                    progress_bar.finish_and_clear();
                    let _ = self.send_and_confirm(&ixs, ComputeBudget::Fixed(COMPUTE_BUDGET), false)
                        .await?;
                }
            }
        }

        Ok(())
    }
} 

