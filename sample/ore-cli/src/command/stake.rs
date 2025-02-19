use std::{str::FromStr, collections::HashMap};

use colored::*;
use ore_api::state::proof_pda;
use ore_boost_api::{state::{boost_pda, stake_pda, Boost}, consts::BOOST_DENOMINATOR};
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::signature::Signer;
use spl_token::{amount_to_ui_amount, state::Mint};
use steel::{AccountMeta, Instruction};
use tabled::{Table, settings::{Style, Color, Remove, object::{Rows, Columns}, Alignment, Highlight, style::BorderColor, Border}, Tabled};

use crate::{
    args::{StakeArgs, StakeClaimArgs, StakeCommand, StakeDepositArgs, StakeMigrateArgs, StakeWithdrawArgs}, error::Error, utils::{amount_u64_to_f64, ask_confirm, format_timestamp, get_boost, get_boost_stake_accounts, get_boosts, get_legacy_stake, get_mint, get_pools, get_proof, get_share, get_stake, ComputeBudget, TableData, TableSectionTitle}, Miner, StakeAccountsArgs
};



impl Miner {
    pub async fn stake(&self, args: StakeArgs) {
        if let Some(subcommand) = args.command.clone() {
            match subcommand {
                StakeCommand::Claim(subargs) => self.stake_claim(subargs, args).await.unwrap(),
                StakeCommand::Deposit(subargs) => self.stake_deposit(subargs, args).await.unwrap(),
                StakeCommand::Withdraw(subargs) => self.stake_withdraw(subargs, args).await.unwrap(),
                StakeCommand::Migrate(subargs) => self.stake_migrate(subargs, args).await.unwrap(),
                StakeCommand::Accounts(subargs) => self.stake_accounts(subargs, args).await.unwrap(),
            }
        } else {
            if let Some(mint) = args.mint {
                self.stake_get(mint).await.unwrap();
            } else {
                self.stake_list(args).await.unwrap();
            }
        }
    }

    async fn stake_claim(&self, claim_args: StakeClaimArgs, stake_args: StakeArgs) -> Result<(), Error> {
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let mint_str= stake_args.mint.expect("Mint address is required");
        let mint_address = Pubkey::from_str(&mint_str).expect("Failed to parse mint address");
        let boost_address = boost_pda(mint_address).0;
        let stake_address = stake_pda(pubkey, boost_address).0;

        let mut ixs = vec![];
        let beneficiary = match claim_args.to {
            None => self.initialize_ata(pubkey).await,
            Some(to) => {
                let wallet = Pubkey::from_str(&to).expect("Failed to parse wallet address");
                let beneficiary_tokens = spl_associated_token_account::get_associated_token_address(
                    &wallet,
                    &ore_api::consts::MINT_ADDRESS,
                );
                if self.rpc_client.get_token_account(&beneficiary_tokens).await.is_err() {
                    ixs.push(
                        spl_associated_token_account::instruction::create_associated_token_account(
                            &pubkey,
                            &wallet,
                            &ore_api::consts::MINT_ADDRESS,
                            &spl_token::id(),
                        ),
                    );
                }
                beneficiary_tokens
            }
        };

        // Get stake account data to check rewards balance
        let stake = get_stake(&self.rpc_client, stake_address).await.expect("Failed to fetch stake account");

        // Build claim instruction with amount or max rewards
        ixs.push(ore_boost_api::sdk::claim(
            pubkey,
            beneficiary,
            mint_address,
            claim_args.amount
                .map(|a| crate::utils::amount_f64_to_u64(a))
                .unwrap_or(stake.rewards),
        ));

        // Send and confirm transaction
        println!("Claiming staking yield...");
        self.send_and_confirm(&ixs, ComputeBudget::Fixed(32_000), false).await.ok();

        Ok(())
    }

    async fn stake_get(&self, mint: String) -> Result<(), Error> {
        // Fetch onchain data
        let mint_address = Pubkey::from_str(&mint).expect("Failed to parse mint address");
        let boost_address = boost_pda(mint_address).0;
        let stake_address = stake_pda(self.signer().pubkey(), boost_address).0;
        let boost = get_boost(&self.rpc_client, boost_address).await.expect("Failed to fetch boost account");
        let mint = get_mint(&self.rpc_client, mint_address).await.expect("Failed to fetch mint account");
        let metadata_address = mpl_token_metadata::accounts::Metadata::find_pda(&mint_address).0;
        let symbol = match self.rpc_client.get_account_data(&metadata_address).await {
            Ok(metadata_data) => {
                if let Ok(metadata) = mpl_token_metadata::accounts::Metadata::from_bytes(&metadata_data) {
                    format!(" {}", metadata.symbol.trim_matches('\0'))
                } else {
                    " ".to_string()
                }
            }
            Err(_) => " ".to_string()
        };

        // Aggregate data
        let mut data = vec![];
        self.fetch_boost_data(boost_address, boost, mint, symbol.clone(), &mut data).await;
        let len1 = data.len();
        self.fetch_stake_data(stake_address, boost, mint, symbol.clone(), &mut data).await;
        let _len2 = data.len();

        // Build table
        let mut table = Table::new(data);
        table.with(Remove::row(Rows::first()));
        table.modify(Columns::single(1), Alignment::right());
        table.with(Style::blank());
        table.section_title(0, "Boost");
        table.section_title(len1, "Stake");
        println!("{table}\n");
        Ok(())
    }

    async fn fetch_stake_data(&self, address: Pubkey, boost: Boost, mint: Mint, symbol: String, data: &mut Vec<TableData>) {
        let stake = get_stake(&self.rpc_client, address).await;
        data.push(TableData {
            key: "Address".to_string(),
            value: address.to_string(),
        });
        if let Ok(stake) = stake {
            data.push(TableData {
                key: "Deposits".to_string(),
                value: format!(
                    "{}{} ({}% of total)",
                    amount_to_ui_amount(stake.balance, mint.decimals),
                    symbol,
                    (stake.balance as f64 / boost.total_deposits as f64) * 100f64
                ),
            });
            data.push(TableData {
                key: "Pending deposits".to_string(),
                value: format!(
                    "{}{} ({}% of total)",
                    amount_to_ui_amount(stake.balance_pending, mint.decimals),
                    symbol.trim_end_matches(' '),
                    (stake.balance_pending as f64 / boost.total_deposits as f64) * 100f64
                ),
            });
            data.push(TableData {
                key: "Last deposit at".to_string(),
                value: format_timestamp(stake.last_deposit_at),
            });
            data.push(TableData {
                key: "Yield".to_string(),
                value: if stake.rewards > 0 {
                    format!("{} ORE", amount_u64_to_f64(stake.rewards)).yellow().bold().to_string()
                } else {
                    format!("{} ORE", amount_u64_to_f64(stake.rewards))
                },
            });
        } else {
            data.push(TableData {
                key: "Status".to_string(),
                value: "Not found".red().bold().to_string(),
            });
        }
    }

    async fn fetch_boost_data(&self, address: Pubkey, boost: Boost, mint: Mint, symbol: String, data: &mut Vec<TableData>) {
        let boost_proof_address = proof_pda(address).0;
        let boost_proof: ore_api::prelude::Proof = get_proof(&self.rpc_client, boost_proof_address).await.expect("Failed to fetch proof account");
        data.push(TableData {
            key: "Address".to_string(),
            value: address.to_string(),
        });
        data.push(TableData {
            key: "Expires at".to_string(),
            value: format_timestamp(boost.expires_at),
        });
        data.push(TableData {
            key: "Locked".to_string(),
            value: boost.locked.to_string(),
        });
        data.push(TableData {
            key: "Mint".to_string(),
            value: boost.mint.to_string(),
        });
        data.push(TableData {
            key: "Multiplier".to_string(),
            value: format!(
                "{}x",
                boost.multiplier as f64 / BOOST_DENOMINATOR as f64
            ),
        });
        data.push(TableData {
            key: "Pending yield".to_string(),
            value: format!(
                "{} ORE",
                amount_to_ui_amount(boost_proof.balance, ore_api::consts::TOKEN_DECIMALS)
            ),
        });
        data.push(TableData {
            key: "Total deposits".to_string(),
            value: format!(
                "{}{}",
                amount_to_ui_amount(boost.total_deposits, mint.decimals),
                symbol.trim_end_matches(' ')
            ),
        });
        data.push(TableData { 
            key: "Total stakers".to_string(), 
            value: boost.total_stakers.to_string()
        });
    }

    async fn stake_list(&self, args: StakeArgs) -> Result<(), Error> {
        // Get the account address
        let authority = match &args.authority {
            Some(authority) => Pubkey::from_str(&authority).expect("Failed to parse account address"),
            None => self.signer().pubkey(),
        };

        // Iterate over all boosts
        let mut data = vec![];
        let boosts = get_boosts(&self.rpc_client).await.expect("Failed to fetch boosts");
        for (address, boost) in boosts {
            // Get relevant accounts
            let stake_address = stake_pda(authority, address).0;
            let stake = get_stake(&self.rpc_client, stake_address).await;
            let mint = get_mint(&self.rpc_client, boost.mint).await.expect("Failed to fetch mint account");
            let metadata_address = mpl_token_metadata::accounts::Metadata::find_pda(&boost.mint).0;
            let symbol = match self.rpc_client.get_account_data(&metadata_address).await {
                Err(_) => "".to_string(),
                Ok(metadata_data) => {
                    if let Ok(metadata) = mpl_token_metadata::accounts::Metadata::from_bytes(&metadata_data) {
                        format!(" {}", metadata.symbol.trim_matches('\0'))
                    } else {
                        "".to_string()
                    }
                }
            };

            // Parse optional stake data
            let (stake_balance, stake_balance_pending, stake_rewards) = if let Ok(stake) = stake {
                (stake.balance, stake.balance_pending, stake.rewards)
            } else {
                (0, 0, 0)
            };


            // Aggregate data
            data.push(StakeTableData {
                mint: boost.mint.to_string(),
                symbol,
                multiplier: format!("{}x", boost.multiplier as f64 / BOOST_DENOMINATOR as f64),
                // expires_at: format_timestamp(boost.expires_at),
                total_deposits: format!("{}", amount_to_ui_amount(boost.total_deposits, mint.decimals)),
                total_stakers: boost.total_stakers.to_string(),
                my_deposits: format!("{}", amount_to_ui_amount(stake_balance, mint.decimals)),
                my_pending_deposits: format!("{}", amount_to_ui_amount(stake_balance_pending, mint.decimals)),
                my_share: if boost.total_deposits > 0 {
                    format!("{:.8}%", (stake_balance as f64 / boost.total_deposits as f64) * 100f64)
                } else {
                    "NaN".to_string()
                },
                my_yield: if stake_rewards > 0 {
                    format!("{} ORE", amount_u64_to_f64(stake_rewards)).yellow().bold().to_string()
                } else {
                    format!("{} ORE", amount_u64_to_f64(stake_rewards))
                },
            });
        }

        // Build table
        let mut table = Table::new(data);
        table.with(Style::blank());
        table.modify(Rows::first(), Color::BOLD);
        table.modify(Columns::new(1..), Alignment::right());
        table.with(Highlight::new(Rows::single(1)).color(BorderColor::default().top(Color::FG_WHITE)));
        table.with(Highlight::new(Rows::single(1)).border(Border::new().top('━')));
        println!("\n{table}\n");
        Ok(())
    }

    async fn stake_deposit(
        &self,
        args: StakeDepositArgs,
        stake_args: StakeArgs,
    ) -> Result<(), Error> {
        // Parse mint address
        let mint_str= stake_args.mint.expect("Mint address is required");
        let mint_address = Pubkey::from_str(&mint_str).expect("Failed to parse mint address");

        // Get signer
        let signer = self.signer();
        let sender = match &args.token_account {
            Some(address) => {
                Pubkey::from_str(&address).expect("Failed to parse token account address")
            }
            None => spl_associated_token_account::get_associated_token_address(
                &signer.pubkey(),
                &mint_address,
            ),
        };

        // Get token account
        let mint_data = self.rpc_client.get_account_data(&mint_address).await.expect("Failed to fetch mint account");
        let mint = Mint::unpack(&mint_data).expect("Failed to parse mint account");
        let token_account = self
            .rpc_client
            .get_token_account(&sender)
            .await
            .expect("Failed to fetch token account")
            .expect("Token account not found");

        // Parse amount
        let amount: u64 = if let Some(amount) = args.amount {
            (amount * 10f64.powf(mint.decimals as f64)) as u64
        } else {
            u64::from_str(token_account.token_amount.amount.as_str())
                .expect("Failed to parse token balance")
        };

        // Get addresses
        let boost_address = boost_pda(mint_address).0;
        let stake_address = stake_pda(signer.pubkey(), boost_address).0;
        let _boost = get_boost(&self.rpc_client, boost_address).await.expect("Failed to fetch boost account");

        // Open stake account, if needed
        if self.rpc_client.get_account_data(&stake_address).await.is_err() {
            println!("Initializing stake account...");
            let ix = ore_boost_api::sdk::open(signer.pubkey(), signer.pubkey(), mint_address);
            self.send_and_confirm(&[ix], ComputeBudget::Fixed(50_000), false).await.ok();
        }

        // Send tx
        println!("Depositing stake...");
        let ix = ore_boost_api::sdk::deposit(signer.pubkey(), mint_address, amount);
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(50_000), false).await.ok();
        Ok(())
    }

    async fn stake_withdraw(
        &self,
        args: StakeWithdrawArgs,
        stake_args: StakeArgs,
    ) -> Result<(), Error> {
        // Parse mint address
        let mint_str= stake_args.mint.expect("Mint address is required");
        let mint_address = Pubkey::from_str(&mint_str).expect("Failed to parse mint address");

        // Get signer
        let signer = self.signer();

        // Get beneficiary token account
        let beneficiary = match &args.token_account {
            Some(address) => {
                Pubkey::from_str(&address).expect("Failed to parse token account address")
            }
            None => spl_associated_token_account::get_associated_token_address(
                &signer.pubkey(),
                &mint_address,
            ),
        };

        // Create token account if necessary
        let mut ixs = vec![];
        if self.rpc_client.get_token_account(&beneficiary).await.is_err() {
            ixs.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &signer.pubkey(),
                    &signer.pubkey(),
                    &mint_address,
                    &spl_token::id(),
                ),
            );
        };

        // Get mint account
        let mint_data = self.rpc_client.get_account_data(&mint_address).await.expect("Failed to fetch mint account");
        let mint = Mint::unpack(&mint_data).expect("Failed to parse mint account");

        // Get addresses
        let boost_address = boost_pda(mint_address).0;
        let stake_address = stake_pda(signer.pubkey(), boost_address).0;
        let _boost = get_boost(&self.rpc_client, boost_address).await.expect("Failed to fetch boost account");
        let stake = get_stake(&self.rpc_client, stake_address).await.expect("Failed to fetch stake account");
        
        // Parse amount
        let amount: u64 = if let Some(amount) = args.amount {
            (amount * 10f64.powf(mint.decimals as f64)) as u64
        } else {
            stake.balance
        };

        // Send tx
        ixs.push(ore_boost_api::sdk::withdraw(signer.pubkey(), mint_address, amount));
        self.send_and_confirm(&ixs, ComputeBudget::Fixed(100_000), false)
            .await
            .ok();

        Ok(())
    }

}


impl Miner {
    async fn stake_migrate(&self, _args: StakeMigrateArgs, _stake_args: StakeArgs) -> Result<(), Error> {
        let signer = self.signer();
        let pubkey = signer.pubkey();

        // Get boost metadata
        let boost_mints = [
            Pubkey::from_str("8H8rPiWW4iTFCfEkSnf7jpqeNpFfvdH9gLouAL3Fe2Zx").unwrap(),
            Pubkey::from_str("DrSS5RM7zUd9qjUEdDaf31vnDUSbCrMto6mjqTrHFifN").unwrap(),
            Pubkey::from_str("7G3dfZkSk1HpDGnyL37LMBbPEgT4Ca6vZmZPUyi2syWt").unwrap(),
            Pubkey::from_str("meUwDp23AaxhiNKaQCyJ2EAF2T4oe1gSkEkGXSRVdZb").unwrap(),
            Pubkey::from_str("oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp").unwrap(),
        ];
        let mut boost_metadatas = HashMap::new();
        for mint_address in boost_mints {   
            let mint_account = get_mint(&self.rpc_client, mint_address).await.expect("Failed to fetch mint account");
            let metadata_address = mpl_token_metadata::accounts::Metadata::find_pda(&mint_address).0;
            let symbol = match self.rpc_client.get_account_data(&metadata_address).await {
                Ok(metadata_data) => {
                    if let Ok(metadata) = mpl_token_metadata::accounts::Metadata::from_bytes(&metadata_data) {
                        format!(" {}", metadata.symbol.trim_matches('\0'))
                    } else {
                        "".to_string()
                    }
                }
                Err(_) => "".to_string()
            };
            boost_metadatas.insert(mint_address, (mint_account, symbol));
        };

        // Migrate stake from solo accounts
        let mut ixs: Vec<Instruction> = vec![];
        println!("{}", "Scanning personal stake accounts...".bold().to_string());
        for mint in boost_mints {
            let legacy_boost_address = ore_pool_api::state::legacy_boost_pda(mint).0;
            let legacy_stake_address = ore_pool_api::state::legacy_stake_pda(pubkey, legacy_boost_address).0;
            let new_boost_address = boost_pda(mint).0;
            let new_stake_address = stake_pda(pubkey, new_boost_address).0;
            if let Ok(legacy_stake_account) = get_legacy_stake(&self.rpc_client, legacy_stake_address).await {
                if legacy_stake_account.balance > 0 {
                    if ask_confirm(
                        format!(
                            "\nFound {}{} tokens in legacy stake account.\nDo you want to migrate this stake to global boosts? [Y/n]",
                            amount_to_ui_amount(legacy_stake_account.balance, boost_metadatas[&mint].0.decimals).to_string().bold(),
                            boost_metadatas[&mint].1.bold()
                        )
                        .as_str(),
                    ) {
                        let beneficiary_address = spl_associated_token_account::get_associated_token_address(&pubkey, &mint);
                        let legacy_boost_token_account = spl_associated_token_account::get_associated_token_address(&legacy_boost_address, &mint);
                        ixs.push(
                            Instruction {
                                program_id: ore_pool_api::consts::LEGACY_BOOST_PROGRAM_ID,
                                accounts: vec![
                                    AccountMeta::new(pubkey, true),
                                    AccountMeta::new(beneficiary_address, false),
                                    AccountMeta::new(legacy_boost_address, false),
                                    AccountMeta::new(legacy_boost_token_account, false),
                                    AccountMeta::new_readonly(mint, false),
                                    AccountMeta::new(legacy_stake_address, false),
                                    AccountMeta::new_readonly(spl_token::id(), false),
                                ],
                                data: [
                                    [3 as u8].to_vec(),
                                    bytemuck::bytes_of(&legacy_stake_account.balance).to_vec(),
                                ]
                                .concat(),
                            }
                        );
                        if self.rpc_client.get_account_data(&new_stake_address).await.is_err() {
                            ixs.push(ore_boost_api::sdk::open(pubkey, pubkey, mint));
                        }
                        ixs.push(ore_boost_api::sdk::deposit(pubkey, mint, legacy_stake_account.balance));
                        println!("Migrating stake...");
                        self.send_and_confirm(&ixs.clone(), ComputeBudget::Fixed(400_000), false).await.ok();
                        println!("");
                    }
                }
            }
        }

        // Migrate stake from pools
        println!("\n{}", "Scanning pool stake accounts...".bold().to_string());
        let pools = get_pools(&self.rpc_client).await.expect("Failed to fetch pool accounts");
        for (pool_address, pool) in pools {
            let pool_url = String::from_utf8(pool.url.to_vec()).unwrap_or_default();
            let pool_url = pool_url.trim_end_matches('\0');
            for mint in boost_mints {
                let boost_address = boost_pda(mint).0;
                let share_address = ore_pool_api::state::share_pda(signer.pubkey(), pool_address, mint).0;
                if let Ok(share) = get_share(&self.rpc_client, share_address).await {
                    let new_stake_address = stake_pda(pubkey, boost_address).0;
                    if share.balance > 0 {
                        if ask_confirm(
                            format!(
                                "\nFound {}{} tokens in pool stake account ({}).\nDo you want to migrate this stake to global boosts? [Y/n]",
                                amount_to_ui_amount(share.balance, boost_metadatas[&mint].0.decimals).to_string().bold(),
                                boost_metadatas[&mint].1.bold(),
                                pool_url
                            )
                            .as_str(),
                        ) {
                            let beneficiary_address = spl_associated_token_account::get_associated_token_address(&signer.pubkey(), &mint);
                            ixs.push(ore_pool_api::sdk::unstake(signer.pubkey(), mint, pool_address, beneficiary_address, share.balance));
                            if self.rpc_client.get_account_data(&new_stake_address).await.is_err() {
                                ixs.push(ore_boost_api::sdk::open(signer.pubkey(), signer.pubkey(), mint));
                            }
                            ixs.push(ore_boost_api::sdk::deposit(signer.pubkey(), mint, share.balance));
                            println!("Migrating stake...");
                            self.send_and_confirm(&ixs.clone(), ComputeBudget::Fixed(400_000), false).await.ok();
                            println!("");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn stake_accounts(&self, _args: StakeAccountsArgs, stake_args: StakeArgs) -> Result<(), Error> {
        let mint_str= stake_args.mint.expect("Mint address is required");
        let mint_address = Pubkey::from_str(&mint_str).expect("Failed to parse mint address");
        let boost_address = boost_pda(mint_address).0;
        let boost = get_boost(&self.rpc_client, boost_address).await.expect("Failed to fetch boost account");
        let mint_account = get_mint(&self.rpc_client, mint_address).await.expect("Failed to fetch mint account");
        let mut stake_accounts = get_boost_stake_accounts(&self.rpc_client, boost_address).await.expect("Failed to fetch stake accounts");
        stake_accounts.sort_by(|(_addr1, stake1), (_addr2, stake2)| stake2.balance.cmp(&stake1.balance));
        let mut data = vec![];
        for (_stake_address, stake) in stake_accounts {
            data.push(StakerTableData {
                authority: stake.authority.to_string(),
                deposits: format!("{:#.11}", amount_to_ui_amount(stake.balance, mint_account.decimals)),
                deposits_pending: format!("{:#.11}", amount_to_ui_amount(stake.balance_pending, mint_account.decimals)),
                share: if boost.total_deposits > 0 {
                    format!("{:.5}%", stake.balance as f64 / boost.total_deposits as f64 * 100f64)
                } else {
                    "NaN".to_string()
                },
                rewards: format!("{:#.11} ORE", amount_to_ui_amount(stake.rewards, ore_api::consts::TOKEN_DECIMALS)),
            });
        }
        let mut table = Table::new(data);
        table.with(Style::blank());
        table.modify(Rows::first(), Color::BOLD);
        table.modify(Columns::new(1..), Alignment::right());
        table.with(Highlight::new(Rows::single(1)).color(BorderColor::default().top(Color::FG_WHITE)));
        table.with(Highlight::new(Rows::single(1)).border(Border::new().top('━')));
        println!("\n{table}\n");
        Ok(())
    }
}

#[derive(Tabled)]
pub struct StakeTableData {
    #[tabled(rename = "Mint")]
    pub mint: String,
    #[tabled(rename = "Symbol")]
    pub symbol: String,
    #[tabled(rename = "Multiplier")]
    pub multiplier: String,
    #[tabled(rename = "Stakers")]
    pub total_stakers: String,
    #[tabled(rename = "Deposits")]
    pub total_deposits: String,
    #[tabled(rename = "My deposits")]
    pub my_deposits: String,
    #[tabled(rename = "My pending deposits")]
    pub my_pending_deposits: String,
    #[tabled(rename = "My share")]
    pub my_share: String,
    #[tabled(rename = "My yield")]
    pub my_yield: String,
}

#[derive(Tabled)]
pub struct StakerTableData {
    #[tabled(rename = "Authority")]
    pub authority: String,
    #[tabled(rename = "Deposits")]
    pub deposits: String,
    #[tabled(rename = "Pending deposits")]
    pub deposits_pending: String,
    #[tabled(rename = "Share")]
    pub share: String,
    #[tabled(rename = "Yield")]
    pub rewards: String,
}