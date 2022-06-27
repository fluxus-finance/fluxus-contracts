mod utils;

use near_units::parse_near;
use workspaces::{
    network::{DevAccountDeployer, TopLevelAccountCreator},
    operations::Transaction,
    Account, AccountId, Contract, Network, Worker,
};

const TOTAL_GAS: u64 = 300_000_000_000_000;

const CONTRACT_ID_REF_EXC: &str = "exchange.ref-dev.testnet";

/// Runs the full cycle of auto-compound
async fn do_auto_compound(
    contract: &Contract,
    farm: &Contract,
    worker: &Worker<impl Network>,
) -> anyhow::Result<()> {
    let res = contract
        .call(&worker, "claim_reward")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // println!("claim_reward {:#?}\n", res);

    let res = contract
        .call(&worker, "withdraw_of_reward")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // println!("withdraw_of_reward {:#?}\n", res);

    let res = contract
        .call(&worker, "autocompounds_swap")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // println!("autocompounds_swap {:#?}\n", res);

    let res = contract
        .call(&worker, "autocompounds_liquidity_and_stake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // println!("autocompounds_liquidity_and_stake {:#?}\n", res);

    // utils::log_farm_seeds(&contract, &farm, &worker).await?;

    Ok(())
}

/// Return the number of shares that the account has in the auto-compound contract
async fn get_user_shares(
    contract: &Contract,
    account: &Account,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    let res = contract
        .call(&worker, "get_user_shares")
        .args_json(serde_json::json!({
            "account_id": account.id()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    let account_shares: String = res.json()?;
    let shares: u128 = utils::str_to_u128(&account_shares);

    Ok(shares)
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
    let exchange = utils::deploy_exchange(
        &owner,
        &exchange_id,
        vec![&token_1.id(), &token_2.id(), &token_reward.id()],
        &worker,
    )
    .await?;

    // Transfer tokens from owner to new account
    let account_1 = worker.dev_create_account().await?;

    utils::transfer_tokens(
        &owner,
        &account_1,
        maplit::hashmap! {
            token_1.id() => parse_near!("10,000 N"),
            token_2.id() => parse_near!("10,000 N"),
        },
        &worker,
    )
    .await?;

    // Register contracts into exchange
    utils::register_into_contracts(
        &worker,
        exchange.as_account(),
        vec![token_1.id(), token_2.id(), token_reward.id()],
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
    // Stage 2: Create pools and farm
    ///////////////////////////////////////////////////////////////////////////

    let (pool_token1_token2, pool_token1_reward, pool_token2_reward) = utils::create_pools(
        &owner,
        &exchange,
        &token_1,
        &token_2,
        &token_reward,
        &worker,
    )
    .await?;

    let seed_id: String = format! {"{}@{}", CONTRACT_ID_REF_EXC, pool_token1_token2};

    // Create farm
    let farm = utils::deploy_farm(&owner, &seed_id, &token_reward, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 3: Deploy Vault contract
    ///////////////////////////////////////////////////////////////////////////

    let contract = utils::deploy_full_vault_contract(
        &token_1,
        &token_2,
        &token_reward,
        pool_token1_reward,
        pool_token2_reward,
        pool_token1_token2,
        0,
        &worker,
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 4: Initialize Vault
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
        vec![&exchange_id, token_1.id(), token_2.id(), token_reward.id()],
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
    // println!("mft_register {:#?}", res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 5: Start interacting with Vault
    ///////////////////////////////////////////////////////////////////////////

    let initial_owner_shares: String =
        utils::get_pool_shares(&owner, &exchange, pool_token1_token2, &worker).await?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    /* Stake */
    let res = owner
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": pool_id,
            "receiver_id": contract.id().to_string(),
            "amount": initial_owner_shares.clone(),
            "msg": ""
        }))?
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    // println!("mft_transfer_call {:#?}\n", res);

    let owner_shares_on_contract = get_user_shares(&contract, &owner, &worker).await?;

    // assert that contract received the correct number of shares
    assert_eq!(
        owner_shares_on_contract,
        utils::str_to_u128(&initial_owner_shares),
        "ERR"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 6: Fast forward in the future
    ///////////////////////////////////////////////////////////////////////////

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo pre-fast_forward {:?}", block_info);

    worker.fast_forward(700).await?;

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo post-fast_forward {:?}", block_info);

    // utils::log_farm_info(&farm, &seed_id, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Auto-compound calls
    ///////////////////////////////////////////////////////////////////////////

    do_auto_compound(&contract, &farm, &worker).await?;

    let owner_deposited_shares: u128 = utils::str_to_u128(&initial_owner_shares);

    // Get owner shares from auto-compound contract
    let round1_owner_shares: u128 = get_user_shares(&contract, &owner, &worker).await?;

    // Assert the current value is higher than the initial value deposited
    assert!(
        round1_owner_shares > owner_deposited_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 8: Stake with another account
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

    // println!("add_liquidity {:#?}", res);

    // Get account 1 shares
    let account1_initial_shares: String =
        utils::get_pool_shares(&account_1, &exchange, pool_token1_token2.clone(), &worker).await?;

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
    // println!("mft_transfer_call {:#?}\n", res);

    let account1_shares_on_contract = get_user_shares(&contract, &account_1, &worker).await?;

    // assert that contract received the correct number of shares
    assert_eq!(
        account1_shares_on_contract,
        utils::str_to_u128(&account1_initial_shares),
        "ERR"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 9: Fast forward in the future
    ///////////////////////////////////////////////////////////////////////////

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo pre-fast_forward {:?}", block_info);

    worker.fast_forward(900).await?;

    let block_info = worker.view_latest_block().await?;
    println!("BlockInfo post-fast_forward {:?}", block_info);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 10: Run another round of auto-compound
    ///////////////////////////////////////////////////////////////////////////

    do_auto_compound(&contract, &farm, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 11: Assert owner and account_1 earned shares from auto-compounder strategy
    ///////////////////////////////////////////////////////////////////////////

    // owner shares
    let round2_owner_shares: u128 = get_user_shares(&contract, &owner, &worker).await?;

    assert!(
        round2_owner_shares > round1_owner_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    // get account 1 shares from auto-compounder contract
    let round2_account1_shares: u128 = get_user_shares(&contract, &account_1, &worker).await?;

    // parse String to u128
    let account1_initial_shares: u128 = utils::str_to_u128(&account1_initial_shares);

    assert!(
        round2_account1_shares > account1_initial_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 12: Withdraw from Vault and assert received shares are correct
    ///////////////////////////////////////////////////////////////////////////

    let res = owner
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    let res = account_1
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({}))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    // Get owner shares from exchange
    let owner_shares_on_exchange: String =
        utils::get_pool_shares(&owner, &exchange, pool_token1_token2, &worker).await?;

    let owner_shares_on_exchange: u128 = utils::str_to_u128(&owner_shares_on_exchange);

    assert_eq!(
        round2_owner_shares, owner_shares_on_exchange,
        "ERR_COULD_NOT_WITHDRAW"
    );

    // Get account 1 shares from exchange
    let account1_shares_on_exchange: String =
        utils::get_pool_shares(&account_1, &exchange, pool_token1_token2, &worker).await?;

    let account1_shares_on_exchange: u128 = utils::str_to_u128(&account1_shares_on_exchange);

    assert_eq!(
        round2_account1_shares, account1_shares_on_exchange,
        "ERR_COULD_NOT_WITHDRAW"
    );

    Ok(())
}
