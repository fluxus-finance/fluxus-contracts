use std::{collections::HashMap, hash::Hash};

use fluxus_safe::{self};
mod utils;

use near_units::parse_near;
use percentage::Percentage;
use workspaces::{
    network::{Sandbox},
    Account, AccountId, Contract, Network, Worker,
};

const TOTAL_GAS: u64 = 300_000_000_000_000;

const TOTAL_PROTOCOL_FEE: u128 = 10;
const SENTRY_FEES_PERCENT: u128 = 10;
const STRAT_FEES_PERCENT: u128 = 10;
const TREASURY_FEES_PERCENT: u128 = 80;
/// Runs the full cycle of auto-compound and fast forward
async fn jumbo_do_auto_compound_with_fast_forward(
    sentry_acc: &Account,
    contract: &Contract,
    farm_id_str: &String,
    blocks_to_forward: u64,
    fast_forward_token: &mut u64,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<u128> {
    if blocks_to_forward > 0 {
        let block_info = worker.view_latest_block().await?;
        println!(
            "BlockInfo {fast_forward_token} pre-fast_forward {:?}",
            block_info
        );

        worker.fast_forward(blocks_to_forward).await?;

        let block_info = worker.view_latest_block().await?;
        println!(
            "BlockInfo {fast_forward_token} post-fast_forward {:?}",
            block_info
        );

        *fast_forward_token += 1;
    }

    for i in 0..5 {
        let _res = sentry_acc
            .call(contract.id(), "harvest")
            .args_json(
                serde_json::json!({ "farm_id_str": farm_id_str, "strat_name": "".to_string() }),
            )
            .gas(TOTAL_GAS)
            .transact()
            .await?;
        println!("harvest step {}: {:#?}\n", i + 1, _res);
    }

    Ok(0)
}

/// Return the number of shares that the account has in the auto-compound contract
async fn get_user_shares(
    contract: &Contract,
    account_id: &AccountId,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    let args = serde_json::json!({
        "seed_id": seed_id,
        "user": account_id.to_string(),
    })
    .to_string()
    .into_bytes();

    let account_shares = contract.view("user_share_seed_id", args).await?;

    println!("debug: {:#?}", account_shares.logs);

    Ok(account_shares.json()?)
}

async fn get_seed_total_amount(
    contract: &Contract,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    let args = serde_json::json!({ "seed_id": seed_id })
        .to_string()
        .into_bytes();

    let res = contract.view("seed_total_amount", args).await?;
    let seed_total_amount: u128 = res.json()?;
    println!("seed total amount {:#?}\n", seed_total_amount);

    Ok(seed_total_amount)
}

/// Create new account, register into exchange and deposit into exchange
async fn create_ready_account(
    owner: &Account,
    exchange: &Contract,
    token_1: &Contract,
    token_2: &Contract,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<Account> {
    let new_account = worker.dev_create_account().await?;
    // Transfer from owner to multiple accounts
    utils::transfer_tokens(
        owner,
        vec![&new_account],
        maplit::hashmap! {
            token_1.id() => parse_near!("1,000 N"),
            token_2.id() => parse_near!("1,000 N"),
        },
        worker,
    )
    .await?;
    // register accounts into exchange and transfer tokens
    utils::register_into_contracts(&new_account, vec![exchange.id()]).await?;
    utils::deposit_tokens(
        &new_account,
        exchange,
        maplit::hashmap! {
            token_1.id() => parse_near!("30 N"),
            token_2.id() => parse_near!("30 N"),
        },
    )
    .await?;

    Ok(new_account)
}

/// Adds liquidity to the pool and stake received shares into safe
async fn stake_into_safe(
    safe_contract: &Contract,
    exchange: &Contract,
    account: &Account,
    pool_id: u64,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    // add liquidity to pool
    let _res = account
        .call(exchange.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_id.clone(),
            "amounts": vec![parse_near!("20 N").to_string(), parse_near!("20 N").to_string()],
        }))
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    // Get account 1 shares
    let initial_shares: String = utils::get_pool_shares(account, exchange, pool_id, worker).await?;

    let token_id: String = format!(":{}", pool_id);

    /* Stake */
    let _res = account
        .call(exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": token_id.clone(),
            "receiver_id": safe_contract.id().to_string(),
            "amount": initial_shares,
            "msg": ""
        }))
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    // println!("mft_transfer_call {:#?}\n", res);//

    let deposited_shares = get_user_shares(safe_contract, account.id(), seed_id, worker).await?;

    // assert that contract received the correct number of shares, due to precision issues derived of recurring tithe we must not accept aboslute errors bigger than 9
    // TODO: validates with fuzzing and way more testing to ensure absolute error is not bigger than 9
    let account1_shares_as_int = i128::try_from(deposited_shares).unwrap();
    assert!(
        (account1_shares_as_int - utils::str_to_i128(&initial_shares)).abs() < 9,
        "ERR: the amount of shares doesn't match there is : {} should be {}",
        deposited_shares,
        initial_shares
    );

    Ok(deposited_shares)
}

async fn deploy_aux_contracts(
    owner: &Account,
    proxy_contract: &Contract,
    worker: &Worker<Sandbox>,
) -> (Contract, Contract, Contract, Contract, Contract, Contract) {
    let token_1 = (utils::create_custom_ft(owner, worker).await).unwrap();
    let token_2 = (utils::create_custom_ft(owner, worker).await).unwrap();
    let token_reward_1 = (utils::create_custom_ft(owner, worker).await).unwrap();
    let token_reward_2 = (utils::create_custom_ft(owner, worker).await).unwrap();

    let exchange = (utils::jumbo_deploy_exchange(
        owner,
        proxy_contract,
        vec![
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
        worker,
    )
    .await)
        .unwrap();

    let treasury = (utils::deploy_treasure(owner, &exchange, &token_1, worker).await).unwrap();
    (
        token_1,
        token_2,
        token_reward_1,
        token_reward_2,
        exchange,
        treasury,
    )
}

#[tokio::test]
async fn test_jumbo() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let owner = worker.root_account().expect("");

    ///////////////////////////////////////////////////////////////////////////
    // Stage 1: Deploy relevant contracts
    ///////////////////////////////////////////////////////////////////////////
    let proxy_contract = utils::deploy_proxy_contract(&owner, &worker).await?;

    let (token_1, token_2, token_reward_1, token_reward_2, exchange, treasury) =
        deploy_aux_contracts(&owner, &proxy_contract, &worker).await;

    println!(
        "token1 {} token2 {} reward1 {} reward2 {} exchange {} treasury {}",
        token_1.id(),
        token_2.id(),
        token_reward_1.id(),
        token_reward_2.id(),
        exchange.id(),
        treasury.id(),
    );

    // Create multiple accounts
    let farmer1 = worker.dev_create_account().await?;
    let strat_creator_acc = worker.dev_create_account().await?;
    let sentry_acc = worker.dev_create_account().await?;

    println!(
        "Ids: owner {} farmer1 {} strat_creator_acc {} sentry_acc {}",
        owner.id(),
        farmer1.id(),
        strat_creator_acc.id(),
        sentry_acc.id()
    );

    // Transfer from owner to multiple accounts
    utils::transfer_tokens(
        &owner,
        vec![&farmer1, &strat_creator_acc, treasury.as_account()],
        maplit::hashmap! {
            token_1.id() => parse_near!("10,000 N"),
            token_2.id() => parse_near!("10,000 N"),
        },
        &worker,
    )
    .await?;

    // Register contracts into exchange
    utils::register_into_contracts(
        exchange.as_account(),
        vec![
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
    )
    .await?;

    // Register Sentry into tokens
    utils::register_into_contracts(
        &sentry_acc,
        vec![
            exchange.id(),
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
    )
    .await?;

    // Register Strat creator into tokens
    utils::register_into_contracts(
        &strat_creator_acc,
        vec![
            exchange.id(),
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
    )
    .await?;

    utils::register_into_contracts(
        treasury.as_account(),
        vec![
            exchange.id(),
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
    )
    .await?;

    // register accounts into exchange and transfer tokens
    utils::register_into_contracts(&&farmer1, vec![exchange.id()]).await?;

    utils::deposit_tokens(
        &farmer1,
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

    let reward_liquidity = parse_near!("0.000000000000001 N");
    let base_liquidity = parse_near!("1 N");

    // Create farms
    let farm = utils::jumbo_deploy_farm(&owner, &worker).await?;
    println!("farm contract: {}", farm.id());

    let (
        pool_token1_token2,
        pool_token1_reward1,
        pool_token2_reward1,
        pool_token1_reward2,
        pool_token2_reward2,
    ) = utils::create_pools(
        &owner,
        &exchange,
        &farm,
        &token_1,
        &token_2,
        &token_reward_1,
        &token_reward_2,
        reward_liquidity,
        base_liquidity,
        &worker,
    )
    .await?;

    let seed_id1: String = format! {"{}@{}", exchange.id(), pool_token1_token2};

    println!("seed_id1: {}", seed_id1);

    let (farm_str0, farm_id0) =
        utils::jumbo_create_farm(&owner, &farm, &seed_id1, &token_reward_1, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 3: Deploy Safe contract
    ///////////////////////////////////////////////////////////////////////////

    let safe_contract = utils::deploy_safe_contract(&strat_creator_acc, &treasury, &worker).await?;
    println!("safe contract {}", safe_contract.id());

    utils::create_strategy(
        &strat_creator_acc,
        &safe_contract,
        &exchange,
        &farm,
        &token_1,
        &token_2,
        pool_token1_token2,
        "create_jumbo_strategy",
        &worker,
    )
    .await?;

    utils::add_strategy(
        &safe_contract,
        &token_reward_1,
        seed_id1.clone(),
        pool_token1_reward1,
        pool_token2_reward1,
        farm_id0,
        "add_farm_to_jumbo_strategy",
        &worker,
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 4: Initialize Safe
    ///////////////////////////////////////////////////////////////////////////

    /* Register into farm contract */
    let _res = safe_contract
        .as_account()
        .call(&farm.id(), "storage_deposit")
        .args_json(serde_json::json!({ "account_id": safe_contract.id() }))
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    println!("storage deposit: {:#?}", _res);

    /* Register contract into tokens */
    utils::register_into_contracts(
        safe_contract.as_account(),
        vec![
            exchange.id(),
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
    )
    .await?;

    let token_id: String = format!(":{}", pool_token1_token2);

    // let _res = safe_contract
    //     .as_account()
    //     .call(&exchange.id(), "mft_register")
    //     .args_json(serde_json::json!({
    //         "token_id": token_id.clone(),
    //         "account_id": safe_contract.id() }))
    //     .deposit(parse_near!("1 N"))
    //     .transact()
    //     .await?;
    // println!("mft_register {:#?}", _res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 5: Start interacting with Safe
    ///////////////////////////////////////////////////////////////////////////

    let initial_owner_shares: String =
        utils::get_pool_shares(&owner, &exchange, pool_token1_token2, &worker).await?;

    /* Stake */
    let res = owner
        .call(&exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": token_id,
            "receiver_id": safe_contract.id().to_string(),
            "amount": initial_owner_shares.clone(),
            "msg": ""
        }))
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    println!("mft_transfer_call {:#?}\n", res);

    let owner_shares_on_contract =
        get_user_shares(&safe_contract, &owner.id(), &seed_id1, &worker).await?;

    let seed_total: u128 = get_seed_total_amount(&safe_contract, &seed_id1, &worker).await?;

    assert_eq!(utils::str_to_u128(&initial_owner_shares), seed_total);

    println!(
        "initial: {} owner shares: {}",
        initial_owner_shares, owner_shares_on_contract
    );

    // assert that contract received the correct number of shares
    assert_eq!(
        owner_shares_on_contract,
        utils::str_to_u128(&initial_owner_shares),
        "ERR: the amount of shares doesn't match there is : {} should be {}",
        owner_shares_on_contract,
        initial_owner_shares
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 6: Fast forward in the future and auto-compound
    ///////////////////////////////////////////////////////////////////////////

    let mut fast_forward_counter: u64 = 0;

    jumbo_do_auto_compound_with_fast_forward(
        &owner,
        &safe_contract,
        &farm_str0,
        700,
        &mut fast_forward_counter,
        &worker,
    )
    .await?;

    let owner_deposited_shares: u128 = utils::str_to_u128(&initial_owner_shares);

    // Get owner shares from auto-compound contract
    let round1_owner_shares: u128 =
        get_user_shares(&safe_contract, owner.id(), &seed_id1, &worker).await?;

    println!(
        "owner deposit: {} round1 owner shares: {}",
        owner_deposited_shares, round1_owner_shares
    );

    // Assert the current value is higher than the initial value deposited
    assert!(
        round1_owner_shares > owner_deposited_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK. Expected {} < {}",
        owner_deposited_shares,
        round1_owner_shares
    );

    println!("First round of auto-compound succeeded!");

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Stake with another account
    ///////////////////////////////////////////////////////////////////////////

    // add liquidity for account 1
    let res = farmer1
        .call(&exchange.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_token1_token2.clone(),
            "amounts": vec![parse_near!("20 N").to_string(), parse_near!("20 N").to_string()],
        }))
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    // println!("add_liquidity {:#?}", res);

    // Get account 1 shares
    let account1_initial_shares: String =
        utils::get_pool_shares(&farmer1, &exchange, pool_token1_token2, &worker).await?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    /* Stake */
    let res = farmer1
        .call(&exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": pool_id.clone(),
            "receiver_id": safe_contract.id().to_string(),
            "amount": account1_initial_shares,
            "msg": ""
        }))
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    // println!("mft_transfer_call {:#?}\n", res);//

    let account1_shares_on_contract =
        get_user_shares(&safe_contract, farmer1.id(), &seed_id1, &worker).await?;

    // assert that contract received the correct number of shares, due to precision issues derived of recurring tithe we must not accept aboslute errors bigger than 9
    // TODO: validates with fuzzing and way more testing to ensure absolute error is not bigger than 9
    let account1_shares_as_int = i128::try_from(account1_shares_on_contract).unwrap();
    assert!(
        (account1_shares_as_int - utils::str_to_i128(&account1_initial_shares)).abs() < 9,
        "ERR: the amount of shares doesn't match there is : {} should be {}",
        account1_shares_on_contract,
        account1_initial_shares
    );
    ///////////////////////////////////////////////////////////////////////////
    // Stage 8: Fast forward in the future and auto-compound
    ///////////////////////////////////////////////////////////////////////////

    jumbo_do_auto_compound_with_fast_forward(
        &sentry_acc,
        &safe_contract,
        &farm_str0,
        900,
        &mut fast_forward_counter,
        &worker,
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 9: Assert owner and farmer1 earned shares from auto-compounder strategy
    ///////////////////////////////////////////////////////////////////////////

    // owner shares
    let round2_owner_shares: u128 =
        get_user_shares(&safe_contract, &owner.id(), &seed_id1, &worker).await?;

    assert!(
        round2_owner_shares > round1_owner_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK. Expected {} and received {}",
        round1_owner_shares,
        round2_owner_shares
    );

    // get account 1 shares from auto-compounder contract
    let round2_account1_shares: u128 =
        get_user_shares(&safe_contract, &farmer1.id(), &seed_id1, &worker).await?;

    // parse String to u128
    let account1_initial_shares: u128 = utils::str_to_u128(&account1_initial_shares);

    assert!(
        round2_account1_shares > account1_initial_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK. Expected {} and received {}",
        account1_initial_shares,
        round2_account1_shares
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 10: Withdraw from Safe and assert received shares are correct
    ///////////////////////////////////////////////////////////////////////////

    let _res = owner
        .call(&safe_contract.id(), "unstake")
        .args_json(serde_json::json!({ "seed_id": seed_id1 }))
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    let _res = farmer1
        .call(&safe_contract.id(), "unstake")
        .args_json(serde_json::json!({ "seed_id": seed_id1 }))
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
        utils::get_pool_shares(&farmer1, &exchange, pool_token1_token2, &worker).await?;

    let account1_shares_on_exchange: u128 = utils::str_to_u128(&account1_shares_on_exchange);

    // assert that contract received the correct number of shares, due to precision issues derived of recurring tithe we must not accept aboslute errors bigger than 9
    // TODO: validates with fuzzing and way more testing to ensure absolute error is not bigger than 9
    let round2_account1_shares_as_int = i128::try_from(round2_account1_shares).unwrap();
    let account1_shares_on_exchange_as_int = i128::try_from(account1_shares_on_exchange).unwrap();
    assert!(
        (round2_account1_shares_as_int - account1_shares_on_exchange_as_int).abs() < 9,
        "ERR: the amount of shares doesn't match. There is {} should be {}",
        account1_shares_on_contract,
        account1_initial_shares
    );

    let seed_total_amount: u128 = get_seed_total_amount(&safe_contract, &seed_id1, &worker).await?;

    assert!(
        seed_total_amount == 0u128,
        "ERR: After withdraw from all users, the supply should be 0"
    );

    println!("Unstake was successful!");

    ///////////////////////////////////////////////////////////////////////////
    // Stage 11: Create another strategy and
    // after each round of auto-compound, create another account and stake into safe
    ///////////////////////////////////////////////////////////////////////////

    // create another farm with different reward token from the same seed
    let (farm_str1, farm_id1) =
        utils::jumbo_create_farm(&owner, &farm, &seed_id1, &token_reward_2, &worker).await?;

    // common tokens 1
    // create farm with (token1, token2) pair and token1 as reward
    let (farm_str2, farm_id2) =
        utils::jumbo_create_farm(&owner, &farm, &seed_id1, &token_1, &worker).await?;

    // common tokens 2
    // create farm with (token1, token2) pair and token2 as reward
    let (farm_str3, farm_id3) =
        utils::jumbo_create_farm(&owner, &farm, &seed_id1, &token_2, &worker).await?;

    // create farms map to iterate over
    let mut farms: HashMap<String, u64> = HashMap::new();
    farms.insert(farm_str0, farm_id0);
    farms.insert(farm_str1, farm_id1);
    farms.insert(farm_str2, farm_id2);
    farms.insert(farm_str3, farm_id3);

    // Adds new strategy to safe
    utils::add_strategy(
        &safe_contract,
        &token_reward_2,
        seed_id1.clone(),
        pool_token1_reward2,
        pool_token2_reward2,
        farm_id1,
        "add_farm_to_jumbo_strategy",
        &worker,
    )
    .await?;

    // (token1, token2) -> token1
    utils::add_strategy(
        &safe_contract,
        &token_1,
        seed_id1.clone(),
        utils::POOL_ID_PLACEHOLDER,
        pool_token1_token2,
        farm_id2,
        "add_farm_to_jumbo_strategy",
        &worker,
    )
    .await?;

    // (token1, token2) -> token2
    utils::add_strategy(
        &safe_contract,
        &token_2,
        seed_id1.clone(),
        pool_token1_token2,
        utils::POOL_ID_PLACEHOLDER,
        farm_id3,
        "add_farm_to_jumbo_strategy",
        &worker,
    )
    .await?;

    // TODO: add farms with common tokens

    let mut farmers_map: HashMap<AccountId, u128> = HashMap::new();

    let blocks_to_forward = 300;

    println!("Starting auto-compound test with multiple strategies");
    for i in 0..2u64 {
        // creates new farmer
        let new_farmer =
            create_ready_account(&owner, &exchange, &token_1, &token_2, &worker).await?;
        // stake into safe
        let staked_shares = stake_into_safe(
            &safe_contract,
            &exchange,
            &new_farmer,
            pool_token1_token2,
            &seed_id1,
            &worker,
        )
        .await?;

        // store farmer and initial shares
        farmers_map.insert(new_farmer.id().clone(), staked_shares);

        println!(
            "Created new farmer {} with {} shares",
            new_farmer.id(),
            staked_shares
        );

        // auto-compound from seed1, farmX
        for (farm_str, _) in farms.iter() {
            jumbo_do_auto_compound_with_fast_forward(
                &sentry_acc,
                &safe_contract,
                farm_str,
                blocks_to_forward,
                &mut fast_forward_counter,
                &worker,
            )
            .await?;

            // checks farmers earnings
            for mut farmer in farmers_map.iter_mut() {
                let farmer_id = farmer.0;
                let current_shares = farmer.1;

                let mut latest_shares: u128 =
                    get_user_shares(&safe_contract, farmer_id, &seed_id1, &worker).await?;

                assert!(
                    latest_shares > *current_shares,
                    "Auto-compound failed. In loop {} with farm {} from account {} expected {} to be greater than {}",
                    i,
                    farm_str,
                    farmer_id,
                    latest_shares,
                    current_shares
                );

                println!(
                    "{} had {} and now has {}",
                    farmer_id, current_shares, latest_shares
                );

                // update shares
                farmer.1 = &mut latest_shares;
            }
        }

        println!("Auto-compound test round {} succeed", i);
    }

    Ok(())
}
