use fluxus_safe::{self, get_ids_from_farm};
mod utils;

use near_units::parse_near;
use percentage::Percentage;
use workspaces::{
    network::{DevAccountDeployer, Sandbox},
    Account, AccountId, Contract, Network, Worker,
};

const TOTAL_GAS: u64 = 300_000_000_000_000;

const CONTRACT_ID_REF_EXC: &str = "ref-finance-101.testnet";
const TOTAL_PROTOCOL_FEE: u128 = 10;
const SENTRY_FEES_PERCENT: u128 = 10;
const STRAT_FEES_PERCENT: u128 = 10;
const TREASURY_FEES_PERCENT: u128 = 80;
/// Runs the full cycle of auto-compound and fast forward
async fn do_auto_compound_with_fast_forward(
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

    let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

    // TODO: what is it good for?
    //Check amount of unclaimed rewards the Strategy has
    // let unclaimed_amount = utils::get_unclaimed_rewards(contract, &token_id, worker).await?;
    for i in 0..4 {
        let res = sentry_acc
            .call(worker, contract.id(), "harvest")
            .args_json(serde_json::json!({ "farm_id_str": farm_id_str }))?
            .gas(TOTAL_GAS)
            .transact()
            .await?;
        println!("harvest step {}: {:#?}\n", i + 1, res);
    }

    Ok(0)
}

/// Return the number of shares that the account has in the auto-compound contract
async fn get_user_shares(
    contract: &Contract,
    account: &Account,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    println!("Checking account id {:#?}", account.id().to_string());
    println!("Checking seed_id {:#?}", seed_id);
    let res = contract
        .call(worker, "user_share_seed_id")
        .args_json(serde_json::json!({
            "seed_id": seed_id,
            "user": account.id().to_string(),
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("just retrieved the info {:#?}", res);
    let account_shares: u128 = res.json()?;
    Ok(account_shares)
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
        &worker,
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

    let exchange_id: AccountId = CONTRACT_ID_REF_EXC.parse().unwrap();

    ///////////////////////////////////////////////////////////////////////////
    // Stage 1: Deploy relevant contracts
    ///////////////////////////////////////////////////////////////////////////
    let (token_1, token_2, token_reward_1, token_reward_2, exchange, treasury) =
        deploy_aux_contracts(&owner, &exchange_id, &worker).await;
    // Transfer tokens from owner to new account
    let account_1 = worker.dev_create_account().await?;
    let account_2 = worker.dev_create_account().await?;

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

    utils::transfer_tokens(
        &owner,
        &account_2,
        maplit::hashmap! {
            token_1.id() => parse_near!("10,000 N"),
            token_2.id() => parse_near!("10,000 N"),
        },
        &worker,
    )
    .await?;

    utils::transfer_tokens(
        &owner,
        treasury.as_account(),
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

    // register accounts into exchange and transfer tokens
    utils::register_into_contracts(&worker, &account_1, vec![exchange.id()]).await?;
    utils::register_into_contracts(&worker, &account_2, vec![exchange.id()]).await?;
    utils::register_into_contracts(&worker, treasury.as_account(), vec![exchange.id()]).await?;

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
    let farm = utils::deploy_farm(&owner, &worker).await?;
    let (farm_id0, farm_0) =
        utils::create_farm(&owner, &farm, &seed_id, &token_reward, &worker).await?;

    ///////////////////////////////////////////////////////////////////////////
    // Stage 3: Deploy Safe contract
    ///////////////////////////////////////////////////////////////////////////

    let contract = utils::deploy_safe_contract(&account_2, &treasury, &worker).await?;

    utils::create_strategy(
        &account_2,
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
    // println!("mft_transfer_call {:#?}\n", res);

    let owner_shares_on_contract = get_user_shares(&contract, &owner, &seed_id, &worker).await?;
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

    do_auto_compound_with_fast_forward(
        &owner,
        &contract,
        &farm_id0,
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
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK. Expected {} and received {}",
        owner_deposited_shares,
        round1_owner_shares
    );

    println!("First round of auto-compound succeeded!");

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
    // println!("mft_transfer_call {:#?}\n", res);//

    let account1_shares_on_contract =
        get_user_shares(&contract, &account_1, &seed_id, &worker).await?;

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

    do_auto_compound_with_fast_forward(
        &owner,
        &contract,
        &farm_id0,
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
        "ERR_AUTO_COMPOUND_DOES_NOT_WORK. Expected {} and received {}",
        round1_owner_shares,
        round2_owner_shares
    );

    // get account 1 shares from auto-compounder contract
    let round2_account1_shares: u128 =
        get_user_shares(&contract, &account_1, &seed_id, &worker).await?;

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

    let res = owner
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({ "token_id": token_id }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // println!("---------------------------------------------------------------------");
    let res = account_1
        .call(&worker, contract.id(), "unstake")
        .args_json(serde_json::json!({ "token_id": token_id }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // println!(" account_1 unstaked sucessfully {:#?}", res);

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

    Ok(())
}

// #[tokio::test]
// async fn simulate_reward_distribution() -> anyhow::Result<()> {
//     let worker = workspaces::sandbox().await?;
//     let owner = worker.root_account();

//     let exchange_id: AccountId = CONTRACT_ID_REF_EXC.parse().unwrap();

//     ///////////////////////////////////////////////////////////////////////////
//     // Stage 1: Deploy relevant contracts
//     ///////////////////////////////////////////////////////////////////////////
//     let (token_1, token_2, token_reward, exchange, treasury) =
//         deploy_aux_contracts(&owner, &exchange_id, &worker).await;

//     // Create needed accounts
//     let account_1 = worker.dev_create_account().await?;
//     let strat_creator_acc = worker.dev_create_account().await?;
//     let sentry_acc = worker.dev_create_account().await?;

//     utils::transfer_tokens(
//         &owner,
//         &account_1,
//         maplit::hashmap! {
//             token_1.id() => parse_near!("10,000 N"),
//             token_2.id() => parse_near!("10,000 N"),
//         },
//         &worker,
//     )
//     .await?;

//     // Register contracts into exchange
//     utils::register_into_contracts(
//         &worker,
//         exchange.as_account(),
//         vec![token_1.id(), token_2.id(), token_reward.id()],
//     )
//     .await?;

//     // register accounts into exchange
//     utils::register_into_contracts(&worker, &account_1, vec![exchange.id()]).await?;
//     utils::register_into_contracts(&worker, treasury.as_account(), vec![exchange.id()]).await?;

//     utils::deposit_tokens(
//         &worker,
//         &account_1,
//         &exchange,
//         maplit::hashmap! {
//             token_1.id() => parse_near!("30 N"),
//             token_2.id() => parse_near!("30 N"),
//         },
//     )
//     .await?;

//     ///////////////////////////////////////////////////////////////////////////
//     // Stage 2: Create pools and farm
//     ///////////////////////////////////////////////////////////////////////////

//     let (pool_token1_token2, pool_token1_reward, pool_token2_reward) = utils::create_pools(
//         &owner,
//         &exchange,
//         &token_1,
//         &token_2,
//         &token_reward,
//         &worker,
//     )
//     .await?;

//     let seed_id: String = format! {"{}@{}", CONTRACT_ID_REF_EXC, pool_token1_token2};

//     // Create farm
//     let farm = utils::deploy_farm(&owner, &worker).await?;
//     let farm_0 = utils::create_farm(&owner, &farm, &seed_id, &token_reward, &worker).await?;

//     ///////////////////////////////////////////////////////////////////////////
//     // Stage 3: Deploy Safe contract
//     ///////////////////////////////////////////////////////////////////////////

//     let contract = utils::deploy_safe_contract(&owner, &treasury, &worker).await?;

//     utils::create_strategy(
//         &strat_creator_acc,
//         &contract,
//         &token_1,
//         &token_2,
//         &token_reward,
//         pool_token1_reward,
//         pool_token2_reward,
//         pool_token1_token2,
//         farm_0,
//         &worker,
//     )
//     .await?;

//     ///////////////////////////////////////////////////////////////////////////
//     // Stage 4: Initialize Safe
//     ///////////////////////////////////////////////////////////////////////////

//     /* Register into farm contract */
//     let res = contract
//         .as_account()
//         .call(&worker, farm.id(), "storage_deposit")
//         .args_json(serde_json::json!({ "account_id": contract.id() }))?
//         .deposit(parse_near!("1 N"))
//         .transact()
//         .await?;

//     /* Register contract into tokens */
//     utils::register_into_contracts(
//         &worker,
//         contract.as_account(),
//         vec![&exchange_id, token_1.id(), token_2.id(), token_reward.id()],
//     )
//     .await?;

//     let pool_id: String = format!(":{}", pool_token1_token2);

//     let res = contract
//         .as_account()
//         .call(&worker, exchange.id(), "mft_register")
//         .args_json(serde_json::json!({
//             "token_id": pool_id.clone(),
//             "account_id": contract.id() }))?
//         .deposit(parse_near!("1 N"))
//         .transact()
//         .await?;
//     // println!("mft_register {:#?}", res);
//     ///////////////////////////////////////////////////////////////////////////
//     // Stage 5: Start interacting with Safe
//     ///////////////////////////////////////////////////////////////////////////

//     let initial_owner_shares: String =
//         utils::get_pool_shares(&owner, &exchange, pool_token1_token2, &worker).await?;

//     let token_id: String = format!(":{}", pool_token1_token2);
//     let seed_id: String = format!("{}@{}", exchange.id(), pool_token1_token2);
//     /* Stake */
//     let res = owner
//         .call(&worker, exchange.id(), "mft_transfer_call")
//         .args_json(serde_json::json!({
//             "token_id": token_id,
//             "receiver_id": contract.id().to_string(),
//             "amount": initial_owner_shares.clone(),
//             "msg": ""
//         }))?
//         .gas(TOTAL_GAS)
//         .deposit(parse_near!("1 yN"))
//         .transact()
//         .await?;
//     // println!("mft_transfer_call {:#?}\n", res);

//     let owner_shares_on_contract = get_user_shares(&contract, &owner, &seed_id, &worker).await?;
//     // assert that contract received the correct number of shares
//     assert_eq!(
//         owner_shares_on_contract,
//         utils::str_to_u128(&initial_owner_shares),
//         "ERR: the amount of shares doesn't match there is : {} should be {}",
//         owner_shares_on_contract,
//         initial_owner_shares
//     );

//     ///////////////////////////////////////////////////////////////////////////
//     // Stage 6: Fast forward in the future and auto-compound
//     ///////////////////////////////////////////////////////////////////////////
//     // register user
//     sentry_acc
//         .call(&worker, token_reward.id(), "storage_deposit")
//         .args_json(serde_json::json!({
//             "account_id": sentry_acc.id()
//         }))?
//         .deposit(parse_near!("0.08 N"))
//         .transact()
//         .await?;

//     // register user
//     strat_creator_acc
//         .call(&worker, token_reward.id(), "storage_deposit")
//         .args_json(serde_json::json!({
//             "account_id": strat_creator_acc.id()
//         }))?
//         .deposit(parse_near!("0.08 N"))
//         .transact()
//         .await?;

//     let mut fast_forward_counter: u64 = 0;
//     let balance_before_sentry = i128::try_from(
//         utils::get_balance_of(&sentry_acc, &token_reward, true, &worker, None)
//             .await?
//             .0,
//     )
//     .unwrap();
//     println!("Checking treasury balance");
//     let balance_before_treasury = i128::try_from(
//         utils::get_balance_of(
//             treasury.as_account(),
//             &exchange,
//             false,
//             &worker,
//             Some(token_reward.id().to_string()),
//         )
//         .await?
//         .0,
//     )
//     .unwrap();
//     let balance_before_strat_creator = i128::try_from(
//         utils::get_balance_of(&strat_creator_acc, &token_reward, true, &worker, None)
//             .await?
//             .0,
//     )
//     .unwrap();
//     // println!("Checking sentry balance before: {:#?}",balance_before_sentry);
//     // println!("Checking treasury balance before: {:#?}",balance_before_treasury);
//     // println!("Checking strat_creator balance before: {:#?}",balance_before_strat_creator);

//     let amount_claimed = do_auto_compound_with_fast_forward(
//         &sentry_acc,
//         &contract,
//         &token_id,
//         700,
//         &mut fast_forward_counter,
//         &worker,
//     )
//     .await?;

//     // Validate claimed amount and percentuals after first compound cycle
//     let unclaimed_amount = utils::get_unclaimed_rewards(&contract, &token_id, &worker).await?;
//     assert!(
//         unclaimed_amount == 0,
//         "ERR: Unclaimend amount should be 0 after compounding round"
//     );
//     // println!("Users claimed amount: {:#?}\n", amount_claimed);
//     let all_fees_amount = Percentage::from(TOTAL_PROTOCOL_FEE).apply_to(amount_claimed);
//     let sentry_due_fees =
//         i128::try_from(Percentage::from(SENTRY_FEES_PERCENT).apply_to(all_fees_amount)).unwrap();
//     let strat_creator_due_fees =
//         i128::try_from(Percentage::from(STRAT_FEES_PERCENT).apply_to(all_fees_amount)).unwrap();
//     let treasury_due_fees =
//         i128::try_from(Percentage::from(TREASURY_FEES_PERCENT).apply_to(all_fees_amount)).unwrap();
//     let balance_after_sentry = i128::try_from(
//         utils::get_balance_of(&sentry_acc, &token_reward, true, &worker, None)
//             .await?
//             .0,
//     )
//     .unwrap();
//     let balance_after_treasury = i128::try_from(
//         utils::get_balance_of(
//             treasury.as_account(),
//             &exchange,
//             false,
//             &worker,
//             Some(token_reward.id().to_string()),
//         )
//         .await?
//         .0,
//     )
//     .unwrap();
//     let balance_after_strat_creator = i128::try_from(
//         utils::get_balance_of(&strat_creator_acc, &token_reward, true, &worker, None)
//             .await?
//             .0,
//     )
//     .unwrap();
//     assert!(
//         (balance_after_sentry - (sentry_due_fees + balance_before_sentry)).abs() < 9,
//         "ERR: Sentry did not receive his due fees. there is: {} should be: {}",
//         balance_after_sentry,
//         (sentry_due_fees + balance_before_sentry)
//     );

//     assert!(
//         (balance_after_strat_creator - (strat_creator_due_fees + balance_before_strat_creator))
//             .abs()
//             < 9,
//         "ERR: Strat Creator did not receive his due fees. there is: {} should be: {}",
//         balance_before_strat_creator,
//         (strat_creator_due_fees + balance_before_strat_creator)
//     );

//     assert!(
//         (balance_after_treasury - (treasury_due_fees + balance_before_treasury)).abs() < 9,
//         "ERR: Treasury did not receive his due fees. there is: {} should be: {}",
//         balance_after_treasury,
//         (strat_creator_due_fees + balance_before_strat_creator)
//     );
//     // println!("Checking sentry balance after: {:#?}",balance_after_sentry);
//     // println!("Checking treasury balance after: {:#?}",balance_after_treasury);
//     // println!("Checking strat_creator balance after: {:#?}",balance_after_strat_creator);
//     let owner_deposited_shares: u128 = utils::str_to_u128(&initial_owner_shares);

//     // Get owner shares from auto-compound contract
//     let round1_owner_shares: u128 = get_user_shares(&contract, &owner, &seed_id, &worker).await?;
//     // Assert the current value is higher than the initial value deposited
//     assert!(
//         round1_owner_shares > owner_deposited_shares,
//         "ERR_AUTO_COMPOUND_DOES_NOT_WORK"
//     );

//     Ok(())
// }
