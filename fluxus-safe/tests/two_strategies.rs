mod utils;

use near_units::parse_near;
use workspaces::{
    network::{DevAccountDeployer, Sandbox},
    Account, AccountId, Contract, Network, Worker,
};

const TOTAL_GAS: u64 = 300_000_000_000_000;
pub const MIN_SEED_DEPOSIT: u128 = 1_000_000_000_000;

const CONTRACT_ID_REF_EXC: &str = "ref-finance-101.testnet";

/// Runs the full cycle of auto-compound and fast forward
async fn do_auto_compound_with_fast_forward(
    sentry_acc: &Account,
    contract: &Contract,
    token_id: &String,
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

    //Check amount of unclaimed rewards the Strategy has
    let unclaimed_amount = utils::get_unclaimed_rewards(contract,token_id,worker).await?;
    for _ in 0..4 {
        let res = sentry_acc
            .call(worker, contract.id(), "harvest")
            .args_json(serde_json::json!({ "token_id": token_id }))?
            .gas(TOTAL_GAS)
            .transact()
            .await?;
        // println!("{:#?}\n", res);
    }

    Ok(unclaimed_amount)
}

use fluxus_safe::{self, SharesBalance};

/// Return the number of shares that the account has in the auto-compound contract
async fn get_user_shares(
    contract: &Contract,
    account: &Account,
    token_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    println!("Checking account id {:#?}", account.id().to_string());
    println!("Checking token_id {:#?}", token_id);
    let res = contract
        .call(worker, "user_share_seed_id")
        .args_json(serde_json::json!({
            "seed_id": token_id,
            "user": account.id().to_string(),
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("just retrieved the info {:#?}", res);
    let account_shares: u128 = res.json()?;
    Ok(account_shares)
}

async fn strategies_amount(
    contract: &Contract,
    account: &Account,
    token_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    println!("We are in the strategies_amount");
    let res = contract
        .call(worker, "number_of_strategies")
        .args_json(serde_json::json!({
            "seed_id": token_id
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    let result: u128 = res.json()?;
    Ok(result)
}

#[tokio::test]
async fn simulate_two_strategies_same_seed_id() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let owner = worker.root_account();

    let exchange_id: AccountId = CONTRACT_ID_REF_EXC.parse().unwrap();

    ///////////////////////////////////////////////////////////////////////////
    // Stage 1: Deploy relevant contracts
    ///////////////////////////////////////////////////////////////////////////

    let token_1 = utils::create_custom_ft(&owner, &worker).await?;
    let token_2 = utils::create_custom_ft(&owner, &worker).await?;
    let token_reward = utils::create_custom_ft(&owner, &worker).await?;
    let token_reward2 = utils::create_custom_ft(&owner, &worker).await?;
    let treasury = (utils::deploy_treasure(&owner, &token_1, &worker).await).unwrap();

    let exchange = utils::deploy_exchange(
        &owner,
        &exchange_id,
        vec![token_1.id(), token_2.id(), token_reward.id(),token_reward2.id()],
        &worker,
    )
    .await?;

    // Transfer tokens from owner to new account
    let account_1 = worker.dev_create_account().await?;
    let strat_creator = worker.dev_create_account().await?;
    // register strat_creator to token rewards
    strat_creator.call(&worker, token_reward.id(), "storage_deposit")
    .args_json(serde_json::json!({
        "account_id": strat_creator.id()
    }))?
    .deposit(parse_near!("0.08 N"))
    .transact()
    .await?;
    strat_creator.call(&worker, token_reward2.id(), "storage_deposit")
    .args_json(serde_json::json!({
        "account_id": strat_creator.id()
    }))?
    .deposit(parse_near!("0.08 N"))
    .transact()
    .await?;

    utils::transfer_tokens(
        &owner,
        &account_1,
        maplit::hashmap! {
            token_1.id() => parse_near!("20,000 N"),
            token_2.id() => parse_near!("20,000 N"),
        },
        &worker,
    )
    .await?;

    // Register contracts into exchange
    utils::register_into_contracts(
        &worker,
        exchange.as_account(),
        vec![token_1.id(), token_2.id(), token_reward.id(), token_reward2.id(), exchange.id()],
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

    let pool_token1_reward2 = utils::create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_1.id() => parse_near!("1 N"),
            token_reward2.id() => parse_near!("0.000000000000001 N") ,
        },
        &worker,
    )
    .await?;
    let pool_token2_reward2 = utils::create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_2.id() => parse_near!("1 N"),
            token_reward2.id() => parse_near!("0.000000000000001 N") ,
        },
        &worker,
    )
    .await?;
 

    let seed_id: String = format! {"{}@{}", CONTRACT_ID_REF_EXC, pool_token1_token2};

    // Create two farms with same seed_id
    let farm = utils::deploy_farm(&owner, &worker).await?;
    let farm_0 = utils::create_farm(&owner, &farm, &seed_id, &token_reward, &worker).await?;
    let farm_1 = utils::create_farm(&owner, &farm, &seed_id, &token_reward2, &worker).await?;

    utils::register_into_contracts(
        &worker,
        exchange.as_account(),
        vec![farm.id()],
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 3: Deploy Safe contract
    ///////////////////////////////////////////////////////////////////////////
    let contract = utils::deploy_safe_contract(&owner, &treasury, &worker).await?;
    //Create first strategy for seed_id
    utils::create_strategy(
        &strat_creator,
        &contract,
        &token_1,
        &token_2,
        &token_reward,
        pool_token1_reward,
        pool_token2_reward,
        pool_token1_token2,
        farm_0,
        &worker,
    )
    .await?;

    //Checking that we have one strategies now.   
    let number_of_strats = strategies_amount(&contract, &owner, &seed_id, &worker).await?;
    
    assert_eq!(
        number_of_strats,
        1_u128,
        "ERR: we don't have the correct number of strategies stored in the code. It is supposed to be {} and it is {}", 1_u128,number_of_strats
    );   
     
        //Create second strategy for seed_id
        utils::create_strategy(
            &strat_creator,
            &contract,
            &token_1,
            &token_2,
            &token_reward2,
            pool_token1_reward2,
            pool_token2_reward2,
            pool_token1_token2,
            farm_1,
            &worker,
        )
        .await?;
    
    

        
    //Checking that we have two strategies now.
    let number_of_strats = strategies_amount(&contract, &owner, &seed_id, &worker).await?;
   
    assert_eq!(
        number_of_strats,
        2_u128,
        "ERR: we don't have the correct number of strategies stored in the code. It is supposed to be {}", 2_u128
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 4: Initialize Safe
    ///////////////////////////////////////////////////////////////////////////

    /* Register into farm contract */
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
        vec![&exchange_id, token_1.id(), token_2.id(), token_reward.id(), token_reward2.id()],
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

    ///////////////////////////////////////////////////////////////////////////
    // Stage 5: Start interacting with Safe
    ///////////////////////////////////////////////////////////////////////////

    let initial_owner_shares: String =
        utils::get_pool_shares(&owner, &exchange, pool_token1_token2, &worker).await?;

    let token_id: String = format!(":{}", pool_token1_token2);
    let seed_id: String = format!("{}@{}", exchange.id(), pool_token1_token2);
    /* Stake */
    let res = owner
        .call(&worker, exchange.id(), "mft_transfer_call")
        .args_json(serde_json::json!({
            "token_id": token_id,
            "receiver_id": contract.id().to_string(),
            "amount": initial_owner_shares.clone(),
            "msg": ""
        }))?
        .gas(TOTAL_GAS)
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;

    let owner_shares_on_contract = get_user_shares(&contract, &owner, &seed_id, &worker).await?;
    // assert that contract received the correct number of shares
    assert_eq!(
        owner_shares_on_contract,
        utils::str_to_u128(&initial_owner_shares),
        "ERR: the amount of shares doesn't match there is : {} should be {}", owner_shares_on_contract,initial_owner_shares
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 6: Fast forward in the future and auto-compound
    ///////////////////////////////////////////////////////////////////////////

    let mut fast_forward_counter: u64 = 0;

    do_auto_compound_with_fast_forward(
        &owner,
        &contract,
        &token_id,
        700,
        &mut fast_forward_counter,
        &worker,
    )
    .await?;


    let owner_deposited_shares: u128 = utils::str_to_u128(&initial_owner_shares);

    // Get owner shares from auto-compound contract
    let round1_owner_shares: u128 = get_user_shares(&contract, &owner, &seed_id, &worker).await?;
    // Assert the current value is higher than the initial value deposited
    assert!(
        round1_owner_shares > owner_deposited_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    ///////////////////////////////////////////////////////////////////////////
    // Stage 7: Stake with another account
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
        utils::get_pool_shares(&account_1, &exchange, pool_token1_token2, &worker).await?;

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

    let account1_shares_on_contract =
        get_user_shares(&contract, &account_1, &seed_id, &worker).await?;

    // assert that contract received the correct number of shares, due to precision issues derived of recurring tithe we must not accept aboslute errors bigger than 9
    // TODO: validates with fuzzing and way more testing to ensure absolute error is not bigger than 9
    let account1_shares_as_int = i128::try_from(account1_shares_on_contract).unwrap();
    assert!(
        (account1_shares_as_int - utils::str_to_i128(&account1_initial_shares)).abs() < 9,
        "ERR: the amount of shares doesn't match there is : {} should be {}", account1_shares_on_contract,account1_initial_shares
    );
    ///////////////////////////////////////////////////////////////////////////
    // Stage 8: Fast forward in the future and auto-compound
    ///////////////////////////////////////////////////////////////////////////

    do_auto_compound_with_fast_forward(
        &owner,
        &contract,
        &token_id,
        900,
        &mut fast_forward_counter,
        &worker,
    )
    .await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 9: Assert owner and account_1 earned shares from auto-compounder strategy
    ///////////////////////////////////////////////////////////////////////////

    // owner shares
    let round2_owner_shares: u128 = get_user_shares(&contract, &owner, &seed_id, &worker).await?;

    assert!(
        round2_owner_shares > round1_owner_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    // get account 1 shares from auto-compounder contract
    let round2_account1_shares: u128 =
        get_user_shares(&contract, &account_1, &seed_id, &worker).await?;

    // parse String to u128
    let account1_initial_shares: u128 = utils::str_to_u128(&account1_initial_shares);

    assert!(
        round2_account1_shares > account1_initial_shares,
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
    );

    Ok(())
}