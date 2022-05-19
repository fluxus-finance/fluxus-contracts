mod utils;

use near_units::parse_near;
use workspaces::{
    network::{DevAccountDeployer, TopLevelAccountCreator},
    operations::Transaction,
    Account, AccountId,
};

const TOTAL_GAS: u64 = 300_000_000_000_000;

const CONTRACT_ID_REF_EXC: &str = "exchange.ref-dev.testnet";

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

    // Transfer tokens from owner to new account
    let account_1 = worker.dev_create_account().await?;

    utils::transfer_tokens(
        &owner,
        &account_1,
        &token_1,
        parse_near!("10,000 N").to_string(),
        &worker,
    )
    .await?;

    utils::transfer_tokens(
        &owner,
        &account_1,
        &token_2,
        parse_near!("10,000 N").to_string(),
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

    // register account 1 into exchange and transfer tokens
    utils::register_into_contracts(&worker, &account_1, vec![exchange.id()]).await?;

    utils::deposit_tokens(
        &worker,
        &account_1,
        &exchange,
        maplit::hashmap! {
            token_1.id() => parse_near!("30 N"),
            token_2.id() => parse_near!("30 N"),
        },
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

    let initial_owner_shares: String = res.json()?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    /* Stake */
    let res = owner
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": pool_id,
            "receiver_id": contract.id().to_string(),
            "amount": initial_owner_shares,
            "msg": ""
        }))?
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    println!("mft_transfer_call {:#?}\n", res);

    // ///////////////////////////////////////////////////////////////////////////
    // // Stage 5: Fast forward in the future
    // ///////////////////////////////////////////////////////////////////////////

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo pre-fast_forward {:?}", block_info);

    // Move to block 500
    worker.fast_forward(500).await?;

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo post-fast_forward {:?}", block_info);

    utils::log_farm_info(&farm, &seed_id, &worker).await?;

    // ///////////////////////////////////////////////////////////////////////////
    // // Stage 6: Auto-compound calls
    // ///////////////////////////////////////////////////////////////////////////

    let res = contract
        .call(&worker, "claim_reward")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("claim_reward {:#?}\n", res);

    let res = contract
        .call(&worker, "withdraw_of_reward")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("withdraw_of_reward {:#?}\n", res);

    let res = contract
        .call(&worker, "autocompounds_swap")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("autocompounds_swap {:#?}\n", res);

    let res = contract
        .call(&worker, "autocompounds_liquidity_and_stake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("autocompounds_liquidity_and_stake {:#?}\n", res);

    utils::log_farm_seeds(&contract, &farm, &worker).await?;

    // Get owner shares from auto-compound contract
    // Assert the current value is higher than the initial value deposited
    let res = contract
        .call(&worker, "get_user_shares")
        .args_json(serde_json::json!({
            "account_id": owner.id()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    let round1_owner_shares: String = res.json()?;

    let initial_shares: u128 = utils::str_to_u128(initial_owner_shares.clone());
    let updated_shares: u128 = utils::str_to_u128(round1_owner_shares.clone());

    assert!(
        updated_shares > initial_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 8: Stake again but now with more than one account
    ///////////////////////////////////////////////////////////////////////////

    // add liquidity for account 1
    let res = account_1
        .call(&worker, exchange.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_token1_token2.clone(),
            "amounts": vec![parse_near!("20 N").to_string(), parse_near!("20 N").to_string()],
        }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    println!("add_liquidity {:#?}", res);

    // Get account 1 shares
    let res = account_1
        .call(&worker, exchange.id(), "get_pool_shares")
        .args_json(serde_json::json!({
            "pool_id": pool_token1_token2,
            "account_id": account_1.id().to_string()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("get_pool_shares {:#?}\n", res);

    let account1_initial_shares: String = res.json()?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    /* Stake */
    let res = account_1
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": pool_id.clone(),
            "receiver_id": contract.id().to_string(),
            "amount": account1_initial_shares,
            "msg": ""
        }))?
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    println!("mft_transfer_call {:#?}\n", res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Run another round of auto-compound
    ///////////////////////////////////////////////////////////////////////////

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo pre-fast_forward {:?}", block_info);

    // Move to block 10000
    worker.fast_forward(700).await?;

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo post-fast_forward {:?}", block_info);

    utils::log_farm_info(&farm, &seed_id, &worker).await?;

    let res = contract
        .call(&worker, "claim_reward")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("claim_reward {:#?}\n", res);

    let res = contract
        .call(&worker, "withdraw_of_reward")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("withdraw_of_reward {:#?}\n", res);

    let res = contract
        .call(&worker, "autocompounds_swap")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("autocompounds_swap {:#?}\n", res);

    let res = contract
        .call(&worker, "autocompounds_liquidity_and_stake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("autocompounds_liquidity_and_stake {:#?}\n", res);

    utils::log_farm_seeds(&contract, &farm, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Assert owner and account_1 earned shares through auto-compound
    ///////////////////////////////////////////////////////////////////////////

    // owner shares
    let res = contract
        .call(&worker, "get_user_shares")
        .args_json(serde_json::json!({
            "account_id": owner.id()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    let round2_owner_shares: String = res.json()?;

    assert!(
        round2_owner_shares > round1_owner_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    // account 1 shares
    let res = contract
        .call(&worker, "get_user_shares")
        .args_json(serde_json::json!({
            "account_id": account_1.id()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    let round2_account1_shares: String = res.json()?;

    assert!(
        round2_account1_shares > account1_initial_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Withdraw from Vault and assert received shares are correct
    ///////////////////////////////////////////////////////////////////////////

    let res = owner
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("unstake {:#?}\n", res);

    let res = account_1
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("unstake {:#?}\n", res);

    // Get account 1 shares
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

    let current_owner_shares: String = res.json()?;
    assert_eq!(
        round2_owner_shares, current_owner_shares,
        "ERR_COULD_NOT_WITHDRAW"
    );

    // Get account 1 shares
    let res = account_1
        .call(&worker, exchange.id(), "get_pool_shares")
        .args_json(serde_json::json!({
            "pool_id": pool_token1_token2,
            "account_id": account_1.id().to_string()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("get_pool_shares {:#?}\n", res);

    let current_account1_shares: String = res.json()?;
    assert_eq!(
        round2_account1_shares, current_account1_shares,
        "ERR_COULD_NOT_WITHDRAW"
    );

    Ok(())
}
