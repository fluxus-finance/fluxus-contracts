use tokio::fs;
use workspaces::network::Sandbox;
use workspaces::prelude::*;

use near_sdk::json_types::U128;

use std::collections::HashMap;

mod utils;

use near_units::{parse_gas, parse_near};
use workspaces::{prelude::*, testnet, DevNetwork};
use workspaces::{Account, AccountId, Contract, Network, Worker};

use near_contract_standards::storage_management::StorageBalance;
const TOTAL_GAS: u64 = 300_000_000_000_000;

const CONTRACT_ID_EHT_TESTNET: &str = "eth.fakes.testnet";
const CONTRACT_ID_WNEAR_TESTNET: &str = "wrap.testnet";
const CONTRACT_ID_REF_EXC: &str = "exchange.ref-dev.testnet";
const FT_CONTRACT_FILEPATH: &str = "./res/fungible_token.wasm";

// #[tokio::test]
async fn simulate_get_vault_shares() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let owner = worker.root_account();

    // TODO: remove exchange if not necessary
    let ref_exchange_id: AccountId = CONTRACT_ID_REF_EXC.parse()?;
    let _ = utils::deploy_exchange(&owner, &ref_exchange_id, vec![], &worker).await?;

    let vault = utils::deploy_vault_contract(&worker).await?;

    let vault_shares: String = vault
        .call(&worker, "get_vault_shares")
        .args_json(serde_json::json!({}))?
        .transact()
        .await?
        .json()?;

    assert_eq!(vault_shares, "0".to_string());

    Ok(())
}

// #[tokio::test]
async fn simulate_whitelisted_tokens() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let owner = worker.root_account();

    let exchange_id = CONTRACT_ID_REF_EXC.parse().unwrap();

    // // TODO: remove exchange if not necessary
    let _ = utils::deploy_exchange(&owner, &exchange_id, vec![], &worker).await?;
    let vault = utils::deploy_vault_contract(&worker).await?;

    // // any token can be whitelisted, leading to errors
    let tokens: Vec<AccountId> = vec![CONTRACT_ID_EHT_TESTNET.parse().unwrap()];

    vault
        .call(&worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({ "tokens": tokens }))?
        .transact()
        .await?;

    let res: Vec<AccountId> = vault
        .call(&worker, "get_whitelisted_tokens")
        .args_json(serde_json::json!({}))?
        .transact()
        .await?
        .json()?;
    assert_eq!(tokens[0].to_string(), res[0].to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_stake_and_withdraw() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let owner = worker.root_account();

    let exchange_id: AccountId = CONTRACT_ID_REF_EXC.parse().unwrap();

    ///////////////////////////////////////////////////////////////////////////
    // Stage 1: Deploy relevant contracts
    ///////////////////////////////////////////////////////////////////////////

    let token_1 = utils::create_custom_ft(&owner, &worker).await?;
    let token_2 = utils::create_custom_ft(&owner, &worker).await?;
    let token_reward = utils::create_custom_ft(&owner, &worker).await?;
    let wnear = utils::deploy_wnear(&owner, &worker).await?;
    let exchange = utils::deploy_exchange(
        &owner,
        &exchange_id,
        vec![
            &token_1.id(),
            &token_2.id(),
            &token_reward.id(),
            &wnear.id(),
        ],
        &worker,
    )
    .await?;

    // Register contracts into exchange
    utils::register_into_contracts(
        &worker,
        exchange.as_account(),
        vec![token_1.id(), token_2.id(), token_reward.id(), wnear.id()],
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 1: Create pools and farm
    ///////////////////////////////////////////////////////////////////////////

    let (
        pool_token1_token2,
        pool_token1_wnear,
        pool_token2_wnear,
        pool_token1_reward,
        pool_token2_reward,
    ) = utils::create_pools(
        &owner,
        &exchange,
        &token_1,
        &token_2,
        &token_reward,
        &wnear,
        &worker,
    )
    .await?;
    let seed_id: String = format! {"{}@{}", CONTRACT_ID_REF_EXC, pool_token1_token2};
    // Create farm
    let farm = utils::deploy_farm(&owner, &seed_id, &token_reward, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 2: Deploy Vault contract
    ///////////////////////////////////////////////////////////////////////////

    let contract = utils::deploy_full_vault_contract(
        &token_1,
        &token_2,
        &token_reward,
        pool_token1_wnear,
        pool_token2_wnear,
        pool_token1_reward,
        pool_token2_reward,
        pool_token1_token2,
        0,
        &worker,
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 3: Initialize Vault
    ///////////////////////////////////////////////////////////////////////////

    /* Register vault into farm contract */
    let res = contract
        .as_account()
        .call(&worker, farm.id(), "storage_deposit")
        .args_json(serde_json::json!({ "account_id": contract.id() }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    /* Register contract into tokens */
    utils::register_into_contracts(
        &worker,
        contract.as_account(),
        vec![
            &wnear.id(),
            &exchange_id,
            token_1.id(),
            token_2.id(),
            token_reward.id(),
        ],
    )
    .await?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    let res = contract
        .as_account()
        .call(&worker, exchange.id(), "mft_register")
        .args_json(serde_json::json!({ 
            "token_id": pool_id.clone(),
            "account_id": contract.id() }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;
    println!("mft_register {:#?}", res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 4: Start interacting with Vault
    ///////////////////////////////////////////////////////////////////////////

    // Get user shares
    let res = owner
        .call(&worker, exchange.id(), "get_pool_shares")
        .args_json(serde_json::json!({
            "pool_id": pool_token1_token2,
            "account_id": owner.id().to_string()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("get_pool_shares {:#?}\n", res);

    let shares: String = res.json()?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    /* Stake */
    let res = owner
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": pool_id,
            "receiver_id": contract.id().to_string(),
            "amount": shares,
            "msg": ""
        }))?
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    println!("mft_transfer_call {:#?}\n", res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 5: Fast forward in the future
    ///////////////////////////////////////////////////////////////////////////

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo pre-fast_forward {:?}", block_info);

    // Move to block 10000
    worker.fast_forward(500).await?;

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo post-fast_forward {:?}", block_info);

    utils::log_farm_info(&farm, &seed_id, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 6: Auto-compound calls
    ///////////////////////////////////////////////////////////////////////////

    // let res = contract
    //     .call(&worker, "claim_reward")
    //     .args_json(serde_json::json!({}))?
    //     .gas(TOTAL_GAS)
    //     .transact()
    //     .await?;
    // println!("claim_reward {:#?}\n", res);

    // let res = contract
    //     .call(&worker, "withdraw_of_reward")
    //     .args_json(serde_json::json!({}))?
    //     .gas(TOTAL_GAS)
    //     .transact()
    //     .await?;
    // println!("withdraw_of_reward {:#?}\n", res);

    // let res = contract
    //     .call(&worker, "autocompounds_swap")
    //     .args_json(serde_json::json!({}))?
    //     .gas(TOTAL_GAS)
    //     .transact()
    //     .await?;
    // println!("autocompounds_swap {:#?}\n", res);

    // let res = contract
    //     .call(&worker, "autocompounds_liquidity_and_stake")
    //     .args_json(serde_json::json!({}))?
    //     .gas(TOTAL_GAS)
    //     .transact()
    //     .await?;
    // println!("autocompounds_liquidity_and_stake {:#?}\n", res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Withdraw from Vault
    ///////////////////////////////////////////////////////////////////////////

    let res = owner
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("unstake {:#?}\n", res);

    Ok(())
}
