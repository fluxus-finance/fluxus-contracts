use std::collections::HashMap;

use fluxus_safe::{self, get_ids_from_farm};
mod utils;

use near_sdk::json_types::U128;
use near_units::parse_near;
use percentage::Percentage;
use workspaces::{
    network::{DevAccountDeployer, Sandbox},
    Account, AccountId, Contract, Network, Worker,
};

async fn fast_forward(
    blocks_to_forward: u64,
    fast_forward_counter: &mut u64,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<u128> {
    let block_info = worker.view_latest_block().await?;
    println!(
        "BlockInfo {fast_forward_counter} pre-fast_forward {:?}",
        block_info
    );

    worker.fast_forward(blocks_to_forward).await?;

    let block_info = worker.view_latest_block().await?;
    println!(
        "BlockInfo {fast_forward_counter} post-fast_forward {:?}",
        block_info
    );

    *fast_forward_counter += 1;

    Ok(0)
}

async fn do_harvest(
    sentry_acc: &Account,
    contract: &Contract,
    farm_id_str: &String,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<u128> {
    //Check amount of unclaimed rewards the Strategy has
    let mut unclaimed_amount = 0u128;

    for i in 0..4 {
        let _res = sentry_acc
            .call(worker, contract.id(), "harvest")
            .args_json(serde_json::json!({ "farm_id_str": farm_id_str }))?
            .gas(utils::TOTAL_GAS)
            .transact()
            .await?;
        println!("harvest step {}: {:#?}\n", i + 1, _res);

        if i == 0 {
            unclaimed_amount = _res.json()?;
        }
    }

    Ok(unclaimed_amount)
}

/// Runs the full cycle of auto-compound and fast forward
async fn do_auto_compound_with_fast_forward(
    sentry_acc: &Account,
    contract: &Contract,
    farm_id_str: &String,
    blocks_to_forward: u64,
    fast_forward_counter: &mut u64,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<u128> {
    if blocks_to_forward > 0 {
        fast_forward(blocks_to_forward, fast_forward_counter, worker).await?;
    }

    let unclaimed_amount = do_harvest(sentry_acc, contract, farm_id_str, worker).await?;

    Ok(unclaimed_amount)
}

/// Return the number of shares that the account has in the auto-compound contract
async fn get_user_shares(
    contract: &Contract,
    account: &AccountId,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    // println!("Checking account id {:#?}", account.to_string());
    // println!("Checking seed_id {:#?}", seed_id);
    let res = contract
        .call(worker, "user_share_seed_id")
        .args_json(serde_json::json!({
            "seed_id": seed_id,
            "user": account.to_string(),
        }))?
        .gas(utils::TOTAL_GAS)
        .transact()
        .await?;
    // println!("just retrieved the info {:#?}", res);
    let account_shares: u128 = res.json()?;
    Ok(account_shares)
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
    utils::register_into_contracts(worker, &new_account, vec![exchange.id()]).await?;
    utils::deposit_tokens(
        worker,
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
        .call(worker, exchange.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_id.clone(),
            "amounts": vec![parse_near!("20 N").to_string(), parse_near!("20 N").to_string()],
        }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    // Get account 1 shares
    let initial_shares: String = utils::get_pool_shares(account, exchange, pool_id, worker).await?;

    let token_id: String = format!(":{}", pool_id);

    /* Stake */
    let _res = account
        .call(worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": token_id.clone(),
            "receiver_id": safe_contract.id().to_string(),
            "amount": initial_shares,
            "msg": ""
        }))?
        .gas(utils::TOTAL_GAS)
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
    exchange_id: &AccountId,
    worker: &Worker<Sandbox>,
) -> (Contract, Contract, Contract, Contract, Contract, Contract) {
    let token_1 = (utils::create_custom_ft(owner, worker).await).unwrap();
    let token_2 = (utils::create_custom_ft(owner, worker).await).unwrap();
    let token_reward_1 = (utils::create_custom_ft(owner, worker).await).unwrap();
    let token_reward_2 = (utils::create_custom_ft(owner, worker).await).unwrap();

    let exchange = (utils::deploy_exchange(
        owner,
        exchange_id,
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

    let treasury = (utils::deploy_treasure(owner, &token_1, worker).await).unwrap();
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
async fn simulate_stake_and_withdraw() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let owner = worker.root_account();

    let exchange_id: AccountId = utils::CONTRACT_ID_REF_EXC.parse().unwrap();

    ///////////////////////////////////////////////////////////////////////////
    // Stage 1: Deploy relevant contracts
    ///////////////////////////////////////////////////////////////////////////

    let (token_1, token_2, token_reward_1, token_reward_2, exchange, treasury) =
        deploy_aux_contracts(&owner, &exchange_id, &worker).await;

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
        &worker,
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
        &worker,
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
        &worker,
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
        &worker,
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
    utils::register_into_contracts(&worker, &farmer1, vec![exchange.id()]).await?;

    utils::deposit_tokens(
        &worker,
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

    let (
        pool_token1_token2,
        pool_token1_reward1,
        pool_token2_reward1,
        pool_token1_reward2,
        pool_token2_reward2,
    ) = utils::create_pools(
        &owner,
        &exchange,
        &token_1,
        &token_2,
        &token_reward_1,
        &token_reward_2,
        reward_liquidity,
        base_liquidity,
        &worker,
    )
    .await?;

    let seed_id1: String = format! {"{}@{}", utils::CONTRACT_ID_REF_EXC, pool_token1_token2};

    // Create farms
    let farm = utils::deploy_farm(&owner, &worker).await?;
    println!("farm contract: {}", farm.id());
    let (farm_str0, farm_id0) =
        utils::create_farm(&owner, &farm, &seed_id1, &token_reward_1, true, &worker).await?;

    println!("Created simple farm!");

    ///////////////////////////////////////////////////////////////////////////
    // Stage 3: Deploy Safe contract
    ///////////////////////////////////////////////////////////////////////////

    let safe_contract = utils::deploy_safe_contract(&strat_creator_acc, &treasury, &worker).await?;
    println!("safe contract {}", safe_contract.id());

    utils::create_strategy(
        &strat_creator_acc,
        &safe_contract,
        &token_1,
        &token_2,
        pool_token1_token2,
        &worker,
    )
    .await?;

    utils::add_strategy(
        &safe_contract,
        &token_reward_1,
        pool_token1_reward1,
        pool_token2_reward1,
        pool_token1_token2,
        farm_id0,
        &worker,
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 4: Initialize Safe
    ///////////////////////////////////////////////////////////////////////////

    /* Register into farm contract */
    let _res = safe_contract
        .as_account()
        .call(&worker, farm.id(), "storage_deposit")
        .args_json(serde_json::json!({ "account_id": safe_contract.id() }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    /* Register contract into tokens */
    utils::register_into_contracts(
        &worker,
        safe_contract.as_account(),
        vec![
            &exchange_id,
            token_1.id(),
            token_2.id(),
            token_reward_1.id(),
            token_reward_2.id(),
        ],
    )
    .await?;

    let token_id: String = format!(":{}", pool_token1_token2);

    let res = safe_contract
        .as_account()
        .call(&worker, exchange.id(), "mft_register")
        .args_json(serde_json::json!({
            "token_id": token_id.clone(),
            "account_id": safe_contract.id() }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;
    // println!("mft_register {:#?}", res);

    ///////////////////////////////////////////////////////////////////////////
    // Stage 5: Start interacting with Safe
    ///////////////////////////////////////////////////////////////////////////

    let initial_owner_shares: String =
        utils::get_pool_shares(&owner, &exchange, pool_token1_token2, &worker).await?;

    /* Stake */
    let res = owner
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": token_id,
            "receiver_id": safe_contract.id().to_string(),
            "amount": initial_owner_shares.clone(),
            "msg": ""
        }))?
        .gas(utils::TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    // println!("mft_transfer_call {:#?}\n", res);

    let owner_shares_on_contract =
        get_user_shares(&safe_contract, &owner.id(), &seed_id1, &worker).await?;
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

    let balance_before_sentry = i128::try_from(
        utils::get_balance_of(&sentry_acc, &token_reward_1, true, &worker, None)
            .await?
            .0,
    )
    .unwrap();

    let balance_before_treasury = i128::try_from(
        utils::get_balance_of(
            treasury.as_account(),
            &exchange,
            false,
            &worker,
            Some(token_reward_1.id().to_string()),
        )
        .await?
        .0,
    )
    .unwrap();

    println!("Checking treasury balance");
    let balance_before_strat_creator = i128::try_from(
        utils::get_balance_of(&strat_creator_acc, &token_reward_1, true, &worker, None)
            .await?
            .0,
    )
    .unwrap();

    let amount_claimed = do_auto_compound_with_fast_forward(
        &sentry_acc,
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
        get_user_shares(&safe_contract, &owner.id(), &seed_id1, &worker).await?;
    // Assert the current value is higher than the initial value deposited
    assert!(
        round1_owner_shares > owner_deposited_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK. Expected {} and received {}",
        owner_deposited_shares,
        round1_owner_shares
    );

    let all_fees_amount = Percentage::from(utils::TOTAL_PROTOCOL_FEE).apply_to(amount_claimed);

    println!(
        "Amount claimed: {} | Protocol fees: {}",
        amount_claimed, all_fees_amount
    );

    let sentry_due_fees =
        i128::try_from(Percentage::from(utils::SENTRY_FEES_PERCENT).apply_to(all_fees_amount))
            .unwrap();
    let strat_creator_due_fees = i128::try_from(
        Percentage::from(utils::STRAT_CREATOR_FEES_PERCENT).apply_to(all_fees_amount),
    )
    .unwrap();
    let treasury_due_fees =
        i128::try_from(Percentage::from(utils::TREASURY_FEES_PERCENT).apply_to(all_fees_amount))
            .unwrap();
    let balance_after_sentry = i128::try_from(
        utils::get_balance_of(&sentry_acc, &token_reward_1, true, &worker, None)
            .await?
            .0,
    )
    .unwrap();
    let balance_after_treasury = i128::try_from(
        utils::get_balance_of(
            treasury.as_account(),
            &exchange,
            false,
            &worker,
            Some(token_reward_1.id().to_string()),
        )
        .await?
        .0,
    )
    .unwrap();
    let balance_after_strat_creator = i128::try_from(
        utils::get_balance_of(&strat_creator_acc, &token_reward_1, true, &worker, None)
            .await?
            .0,
    )
    .unwrap();
    assert!(
        (balance_after_sentry - (sentry_due_fees + balance_before_sentry)).abs() < 9,
        "ERR: Sentry did not receive his due fees. there is: {} should be: {}",
        balance_after_sentry,
        (sentry_due_fees + balance_before_sentry)
    );

    assert!(
        (balance_after_strat_creator - (strat_creator_due_fees + balance_before_strat_creator))
            .abs()
            < 9,
        "ERR: Strat Creator did not receive his due fees. there is: {} should be: {}",
        balance_before_strat_creator,
        (strat_creator_due_fees + balance_before_strat_creator)
    );

    assert!(
        (balance_after_treasury - (treasury_due_fees + balance_before_treasury)).abs() < 9,
        "ERR: Treasury did not receive his due fees. there is: {} should be: {}",
        balance_after_treasury,
        (strat_creator_due_fees + balance_before_strat_creator)
    );

    println!("First round of auto-compound succeeded!");

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Stake with another account
    ///////////////////////////////////////////////////////////////////////////

    // add liquidity for account 1
    let res = farmer1
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
        utils::get_pool_shares(&farmer1, &exchange, pool_token1_token2, &worker).await?;

    let pool_id: String = format!(":{}", pool_token1_token2);

    /* Stake */
    let res = farmer1
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": pool_id.clone(),
            "receiver_id": safe_contract.id().to_string(),
            "amount": account1_initial_shares,
            "msg": ""
        }))?
        .gas(utils::TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;
    // println!("mft_transfer_call {:#?}\n", res);//

    let account1_shares_on_contract =
        get_user_shares(&safe_contract, &farmer1.id(), &seed_id1, &worker).await?;

    // assert that contract received the correct number of shares, due to precision issues derived of recurring tithe we must not accept aboslute errors bigger than 9
    // TODO: validates with fuzzing and way more testing to ensure absolute error is not bigger than 9
    let account1_shares_as_int = i128::try_from(account1_shares_on_contract).unwrap();
    assert!(
        (account1_shares_as_int - utils::str_to_i128(&account1_initial_shares)).abs() < 9,
        "ERR: the amount of shares doesn't match there is : {} should be {}",
        account1_shares_on_contract,
        account1_initial_shares
    );

    println!("Stage 7 succeeded!");
    ///////////////////////////////////////////////////////////////////////////
    // Stage 8: Fast forward in the future and auto-compound
    ///////////////////////////////////////////////////////////////////////////

    // utils::log_farm_info(&farm, &seed_id1, &worker).await;

    do_auto_compound_with_fast_forward(
        &sentry_acc,
        &safe_contract,
        &farm_str0,
        900,
        &mut fast_forward_counter,
        &worker,
    )
    .await?;

    println!("Stage 8 succeeded!");

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

    println!("Stage 9 succeeded!");

    ///////////////////////////////////////////////////////////////////////////
    // Stage 10: Withdraw from Safe and assert received shares are correct
    ///////////////////////////////////////////////////////////////////////////

    let _res = owner
        .call(&worker, safe_contract.id(), "unstake")
        .args_json(serde_json::json!({ "token_id": token_id }))?
        .gas(utils::TOTAL_GAS)
        .transact()
        .await?;

    let _res = farmer1
        .call(&worker, safe_contract.id(), "unstake")
        .args_json(serde_json::json!({ "token_id": token_id }))?
        .gas(utils::TOTAL_GAS)
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

    // assert that fft supply is 0
    let _res = owner
        .call(&worker, safe_contract.id(), "seed_total_amount")
        .args_json(serde_json::json!({ "token_id": token_id }))?
        .gas(utils::TOTAL_GAS)
        .transact()
        .await?;
    let seed_total_amount: u128 = _res.json()?;

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
        utils::create_farm(&owner, &farm, &seed_id1, &token_reward_2, false, &worker).await?;

    // common tokens 1
    // create farm with (token1, token2) pair and token1 as reward
    let (farm_str2, farm_id2) =
        utils::create_farm(&owner, &farm, &seed_id1, &token_1, false, &worker).await?;

    // common tokens 2
    // create farm with (token1, token2) pair and token2 as reward
    let (farm_str3, farm_id3) =
        utils::create_farm(&owner, &farm, &seed_id1, &token_2, false, &worker).await?;

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
        pool_token1_reward2,
        pool_token2_reward2,
        pool_token1_token2,
        farm_id1,
        &worker,
    )
    .await?;

    // (token1, token2) -> token1
    utils::add_strategy(
        &safe_contract,
        &token_1,
        utils::POOL_ID_PLACEHOLDER,
        pool_token1_token2,
        pool_token1_token2,
        farm_id2,
        &worker,
    )
    .await?;

    // (token1, token2) -> token2
    utils::add_strategy(
        &safe_contract,
        &token_2,
        pool_token1_token2,
        utils::POOL_ID_PLACEHOLDER,
        pool_token1_token2,
        farm_id3,
        &worker,
    )
    .await?;

    let mut farmers_map: HashMap<AccountId, u128> = HashMap::new();

    let blocks_to_forward = 300;

    println!("Starting harvest test with multiple strategies");
    for i in 0..2u64 {
        println!("Starting harvest test round {}", i);

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

        fast_forward(blocks_to_forward, &mut fast_forward_counter, &worker).await?;

        // auto-compound from seed1, farmX
        for (farm_str, _) in farms.iter() {
            // utils::log_farm_info(&farm, &seed_id1, &worker).await;

            println!("farm_str {}", farm_str);

            do_harvest(&sentry_acc, &safe_contract, farm_str, &worker).await?;

            // checks farmers earnings
            for mut farmer in farmers_map.iter_mut() {
                let farmer_id = farmer.0;
                let current_shares = farmer.1;

                let mut latest_shares: u128 =
                    get_user_shares(&safe_contract, farmer_id, &seed_id1, &worker).await?;

                assert!(
                    latest_shares > *current_shares,
                    "Harvest failed. In loop {} with farm {} from account {} expected {} to be greater than {}",
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

        println!("Harvest test round {} succeed", i);
    }

    Ok(())
}
