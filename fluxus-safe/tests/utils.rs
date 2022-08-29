use anyhow::Ok;
use near_sdk::json_types::U128;
use near_units::{parse_gas, parse_near};
use std::collections::HashMap;
use tokio::fs;
use workspaces::network::Sandbox;
use workspaces::prelude::*;
use workspaces::{Account, AccountId, Contract, DevNetwork, Network, Worker};

pub const TOTAL_GAS: u64 = 300_000_000_000_000;
pub const MIN_SEED_DEPOSIT: u128 = 1_000_000_000_000;

// pub const CONTRACT_ID_REF_EXC: &str = "ref-finance-101.testnet";
// pub const CONTRACT_ID_FARM: &str = "boostfarm.ref-finance.testnet";
pub const FT_CONTRACT_FILEPATH: &str = "./res/fungible_token.wasm";

pub const TOTAL_PROTOCOL_FEE: u128 = 10;
pub const SENTRY_FEES_PERCENT: u128 = 10;
pub const STRAT_CREATOR_FEES_PERCENT: u128 = 10;
pub const TREASURY_FEES_PERCENT: u128 = 80;
pub const POOL_ID_PLACEHOLDER: u64 = 9999;

type FarmId = String;
type SeedId = String;

use fluxus_safe::AccountFee;
use fluxus_safe::FarmInfoBoost;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FarmInfo {
    pub farm_id: FarmId,
    pub farm_kind: String,
    pub farm_status: String,
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: u32,
    pub reward_per_session: U128,
    pub session_interval: u32,

    pub total_reward: U128,
    pub cur_round: u32,
    pub last_round: u32,
    pub claimed_reward: U128,
    pub unclaimed_reward: U128,
    pub beneficiary_reward: U128,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PoolInfo {
    /// Pool kind.
    pub pool_kind: String,
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<U128>,
    /// Fee charged for swap.
    pub total_fee: u32,
    /// Total number of shares.
    pub shares_total_supply: U128,
    pub amp: u64,
}

pub async fn add_strategy(
    safe_contract: &Contract,
    token_reward: &Contract,
    seed_id: String,
    pool_id_token1_reward: u64,
    pool_id_token2_reward: u64,
    farm_id: u64,
    function_name: &str,
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<()> {
    let res = safe_contract
        .call(worker, function_name)
        .args_json(serde_json::json!({
            "seed_id": seed_id,
            "pool_id_token1_reward": pool_id_token1_reward,
            "pool_id_token2_reward": pool_id_token2_reward,
            "reward_token": token_reward.id().to_string(),
            "farm_id": farm_id.to_string(),
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // assert!(res.is_success());
    println!("add strategy {:#?}", res);

    Ok(())
}
pub async fn create_strategy(
    strat_creator: &Account,
    safe_contract: &Contract,
    exchange_contract_id: &Contract,
    farm_contract_id: &Contract,
    token1: &Contract,
    token2: &Contract,
    pool_id: u64,
    function_name: &str,
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<()> {
    let strat: AccountFee = AccountFee {
        account_id: strat_creator.id().parse().unwrap(),
        fee_percentage: STRAT_CREATOR_FEES_PERCENT,
        current_amount: 0,
    };

    let res = safe_contract
        .call(worker, function_name)
        .args_json(serde_json::json!({
            "_strategy": "".to_string(),
            "strategy_fee": TOTAL_PROTOCOL_FEE,
            "strat_creator": strat,
            "sentry_fee": SENTRY_FEES_PERCENT,
            "exchange_contract_id": exchange_contract_id.id(),
            "farm_contract_id": farm_contract_id.id(),
            "token1_address": token1.id().to_string(),
            "token2_address": token2.id().to_string(),
            "pool_id": pool_id,
            "seed_min_deposit": MIN_SEED_DEPOSIT.to_string()
        }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    // assert!(res.is_success());
    println!("create strategy -> {:#?}", res);

    Ok(())
}

pub async fn deploy_safe_contract(
    owner: &Account,
    treasure: &Contract,
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<Contract> {
    let wasm = fs::read("res/fluxus_safe.wasm").await?;
    let contract = worker.dev_deploy(&wasm).await?;

    let res = contract
        .call(worker, "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "treasure_contract_id": treasure.id()
        }))?
        .transact()
        .await?;

    // println!("deploy safe -> {:#?}", res);

    Ok(contract)
}

pub async fn deploy_treasure(
    owner: &Account,
    token_out: &Contract,
    exchange_contract_id: &Contract,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<Contract> {
    let wasm = fs::read("../fluxus-treasurer/res/fluxus_treasurer.wasm").await?;
    let contract = worker.dev_deploy(&wasm).await?;

    let res = contract
        .call(worker, "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "token_out": token_out.id(),
            "exchange_contract_id": exchange_contract_id.id(),
        }))?
        .transact()
        .await?;

    // println!("deploy treasury -> {:#?}", res);

    Ok(contract)
}

pub async fn deploy_exchange(
    owner: &Account,
    ref_finance_id: &AccountId,
    tokens: Vec<&AccountId>,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<Contract> {
    let testnet = workspaces::testnet().await?;
    // This will pull down the relevant ref-finance contract from testnet. We're going
    // to be overriding the initial balance with 1000N instead of what's on testnet.
    let ref_finance = worker
        .import_contract(&ref_finance_id.clone(), &testnet)
        .transact()
        .await?;

    // NOTE: We are not pulling down the contract's data here, so we'll need ot initialize
    // our own set of metadata. This is because the contract's data is too big for the rpc
    // service to pull down (i.e. greater than 50mb).
    ref_finance
        .call(worker, "new")
        .args_json(serde_json::json!({
            "owner_id": ref_finance.id().clone(),
            "exchange_fee": 4,
            "referral_fee": 1,
        }))?
        .transact()
        .await?;
    // let wnear_id: AccountId = CONTRACT_ID_WNEAR_TESTNET.parse().unwrap();

    ref_finance
        .call(worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({ "tokens": tokens }))?
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;

    owner
        .call(worker, ref_finance_id, "storage_deposit")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("20 N"))
        .transact()
        .await?;

    ref_finance
        .as_account()
        .call(worker, ref_finance_id, "storage_deposit")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("20 N"))
        .transact()
        .await?;

    Ok(ref_finance)
}

pub async fn create_farm(
    owner: &Account,
    farm: &Contract,
    seed_id: &String,
    token_reward: &Contract,
    create_seed: bool,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<(String, u64)> {
    let reward_per_session: String = parse_near!("1000 N").to_string();
    if create_seed {
        let res = owner
            .call(worker, farm.id(), "create_seed")
            .args_json(serde_json::json!({
                "seed_id": seed_id,
                "seed_decimal": 24,
                "min_locking_duration_sec": 0
            }))?
            .deposit(parse_near!("1 yN"))
            .gas(parse_gas!("200 Tgas") as u64)
            .transact()
            .await?;
    }
    let res = owner
        .call(worker, farm.id(), "create_farm")
        .args_json(serde_json::json!({
            "seed_id": seed_id,
            "terms": {
                "reward_token": token_reward.id(),
                "start_at": 0,
                "daily_reward": "48000000000000000000"
            },
            "min_deposit": Some(U128(MIN_SEED_DEPOSIT))
        }))?
        .deposit(parse_near!("1 yN"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("create_farm: {:#?}", res);

    let farm_id: String = res.json()?;

    let res = token_reward
        .call(worker, "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": farm.id(),
        }))?
        .deposit(parse_near!("0.00125 N"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("register farm into reward token -> {:#?}", res);

    let amount: String = parse_near!("100000000 N").to_string();

    let msg = format!("{{\"Reward\":{{\"farm_id\":\"{}\"}} }}", farm_id);
    println!("msg {}", msg);

    let res = owner
        .call(worker, token_reward.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": farm.id(),
            "amount": amount,
            "msg": msg
        }))?
        .deposit(parse_near!("1 yN"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("ft_transfer_call -> {:#?}", res);

    let id = farm_id.chars().last().unwrap().to_digit(10).unwrap() as u64;

    Ok((farm_id, id))
}

pub async fn deploy_farm(owner: &Account, worker: &Worker<Sandbox>) -> anyhow::Result<Contract> {
    let testnet = workspaces::testnet().await?;

    let farm_acc: AccountId = "boostfarm.ref-finance.testnet".parse().unwrap();

    let farm = worker
        .import_contract(&farm_acc, &testnet)
        .transact()
        .await?;

    owner
        .call(worker, farm.id(), "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
        }))?
        .transact()
        .await?;

    // increase reward per session in order to try to swap in the pool for a value that
    // is higher than the pool contains

    // TODO: require farm state is Running

    Ok(farm)
}

pub async fn log_farm_info(
    farm: &Contract,
    seed_id: &String,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<()> {
    let args = serde_json::json!({ "seed_id": seed_id })
        .to_string()
        .into_bytes();

    let res = farm.view(worker, "list_seed_farms", args).await?;
    let info: Vec<FarmInfoBoost> = res.json().unwrap();
    println!("result {:#?}", info);

    Ok(())
}

/// Create a liquidity pool on Ref-Finance, registering the tokens we provide it.
/// Add's the amount in `tokens` we set for liquidity. This will return us the
/// pool_id after the pool has been created.
pub async fn create_pool_with_liquidity(
    owner: &Account,
    exchange: &Contract,
    farming: &Contract,
    tokens: HashMap<&AccountId, u128>,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u64> {
    let (token_ids, token_amounts): (Vec<String>, Vec<String>) = tokens
        .iter()
        .map(|(id, amount)| (id.to_string(), amount.to_string()))
        .unzip();

    let res = exchange
        .call(worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({ "tokens": token_ids }))?
        .deposit(parse_near!("1 yN"))
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    println!("exchange.extend_whitelisted_tokens {:#?}\n", res);

    let pool_id: u64 = exchange
        .call(worker, "add_simple_pool")
        .args_json(serde_json::json!({
            "tokens": token_ids,
            "fee": 25
        }))?
        .deposit(parse_near!("4 mN"))
        .gas(TOTAL_GAS)
        .transact()
        .await?
        .json()?;

    // println!("pool_id == {}\n", pool_id);

    let token_id: String = ":".to_string() + &pool_id.to_string();

    let register = exchange
        .call(worker, "mft_register")
        .args_json(serde_json::json!({
            "token_id": token_id,
            "account_id": farming.id()
        }))?
        .deposit(parse_near!("1 N"))
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    println!("register == {:#?}\n", register);

    let res = owner
        .call(worker, exchange.id(), "register_tokens")
        .args_json(serde_json::json!({
            "token_ids": token_ids,
        }))?
        .deposit(1)
        .gas(TOTAL_GAS)
        .transact()
        .await?;

    println!("register_tokens is {:#?}\n", res);

    deposit_tokens(worker, owner, exchange, tokens).await?;

    let res = owner
        .call(worker, exchange.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_id,
            "amounts": token_amounts,
        }))?
        .deposit(parse_near!("1 N"))
        .gas(TOTAL_GAS)
        .transact()
        .await?;
    println!("added liquidity: {:#?}\n", res);

    // let res = ref_finance
    //     .call(worker, "get_pool")
    //     .args_json(serde_json::json!({ "pool_id": pool_id }))?
    //     .transact()
    //     .await?;

    // println!("get pool {:#?}\n", res);

    Ok(pool_id)
}

pub async fn create_pools(
    owner: &Account,
    exchange: &Contract,
    farming: &Contract,
    token_1: &Contract,
    token_2: &Contract,
    token_reward_1: &Contract,
    token_reward_2: &Contract,
    reward_liquidity: u128,
    base_liquidity: u128,
    worker: &Worker<impl Network>,
) -> anyhow::Result<(u64, u64, u64, u64, u64)> {
    let pool_token1_token2 = create_pool_with_liquidity(
        owner,
        exchange,
        farming,
        maplit::hashmap! {
            token_1.id() => base_liquidity,
            token_2.id() => base_liquidity,
        },
        worker,
    )
    .await?;

    let pool_token1_reward1 = create_pool_with_liquidity(
        owner,
        exchange,
        farming,
        maplit::hashmap! {
            token_1.id() => base_liquidity,
            token_reward_1.id() => reward_liquidity ,
        },
        worker,
    )
    .await?;

    let pool_token2_reward1 = create_pool_with_liquidity(
        owner,
        exchange,
        farming,
        maplit::hashmap! {
            token_2.id() => base_liquidity,
            token_reward_1.id() => reward_liquidity ,
        },
        worker,
    )
    .await?;

    let pool_token1_reward2 = create_pool_with_liquidity(
        owner,
        exchange,
        farming,
        maplit::hashmap! {
            token_1.id() => base_liquidity,
            token_reward_2.id() => reward_liquidity ,
        },
        worker,
    )
    .await?;

    let pool_token2_reward2 = create_pool_with_liquidity(
        owner,
        exchange,
        farming,
        maplit::hashmap! {
            token_2.id() => base_liquidity,
            token_reward_2.id() => reward_liquidity ,
        },
        worker,
    )
    .await?;

    Ok((
        pool_token1_token2,
        pool_token1_reward1,
        pool_token2_reward1,
        pool_token1_reward2,
        pool_token2_reward2,
    ))
}

/// Create our own custom Fungible Token contract and setup the initial state.
pub async fn create_custom_ft(
    owner: &Account,
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<Contract> {
    let ft: Contract = worker
        .dev_deploy(&std::fs::read(FT_CONTRACT_FILEPATH)?)
        .await?;

    // Initialize our FT contract with owner metadata and total supply available
    // to be traded and transferred into other contracts such as Ref-Finance
    ft.call(worker, "new_default_meta")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("1,000,000,000 N").to_string(),
        }))?
        .transact()
        .await?;

    // println!("deployed custom ft: {:?}", ft.id());

    Ok(ft)
}

/// Deposit tokens into Ref-Finance
pub async fn deposit_tokens(
    worker: &Worker<impl Network>,
    owner: &Account,
    exchange: &Contract,
    tokens: HashMap<&AccountId, u128>,
) -> anyhow::Result<()> {
    for (contract_id, amount) in tokens {
        let res = owner
            .call(worker, contract_id, "ft_transfer_call")
            .args_json(serde_json::json!({
                "receiver_id": exchange.id(),
                "amount": amount.to_string(),
                "msg": "",
            }))?
            .gas(parse_gas!("200 Tgas") as u64)
            .deposit(1)
            .transact()
            .await?;
    }

    Ok(())
}

pub async fn register_into_contracts(
    worker: &Worker<impl Network>,
    account: &Account,
    contracts_id: Vec<&AccountId>,
) -> anyhow::Result<()> {
    for contract_id in contracts_id {
        let res = account
            .call(worker, &contract_id, "storage_deposit")
            .args_json(serde_json::json!({
                "registration_only": false,
            }))?
            .deposit(parse_near!("1 N"))
            .transact()
            .await?;

        // println!("{:#?}", res);
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn get_pool_info(
    worker: &Worker<impl Network>,
    ref_finance: &Contract,
    pool_id: u64,
) -> anyhow::Result<()> {
    let args = serde_json::json!({ "pool_id": pool_id })
        .to_string()
        .into_bytes();

    let res = ref_finance.view(worker, "get_pool", args).await?;
    let pool_info: PoolInfo = res.json()?;
    println!("get pool {:#?}\n", pool_info);

    Ok(())
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SeedInfo {
    seed_id: String,
    seed_decimal: u64,
    next_index: u64,
    total_seed_amount: U128,
    total_seed_power: U128,
    min_deposit: U128,
    slash_rate: u64,
    min_locking_duration_sec: u64,
}

#[allow(dead_code)]
pub async fn get_farms_min_deposit(
    farm: &Contract,
    worker: &Worker<impl Network>,
) -> anyhow::Result<HashMap<String, U128>> {
    let args = serde_json::json!({ "from_index": 0u64, "limit": 300u64 })
        .to_string()
        .into_bytes();

    let seeds = farm.view(worker, "list_seeds_info", args).await?;

    let farms_info: Vec<SeedInfo> = seeds.json()?;

    let mut map: HashMap<String, U128> = HashMap::new();

    for info in farms_info {
        map.insert(info.seed_id, info.min_deposit);
    }

    Ok(map)
}

pub async fn transfer_tokens(
    from: &Account,
    to: Vec<&Account>,
    tokens: HashMap<&AccountId, u128>,
    worker: &Worker<impl Network>,
) -> anyhow::Result<()> {
    for (token, amount) in tokens.iter() {
        for receiver in to.iter() {
            let res = receiver
                .call(worker, token, "storage_deposit")
                .args_json(serde_json::json!({
                    "registration_only": false,
                }))?
                .gas(TOTAL_GAS)
                .deposit(parse_near!("1 N"))
                .transact()
                .await?;
            // println!("storage_deposit {:#?}\n", res);

            let res = from
                .call(worker, token, "ft_transfer")
                .args_json(serde_json::json!({
                    "receiver_id": receiver.id(),
                    "amount":  amount.to_string(),
                    "msg": Some(""),
                }))?
                .gas(TOTAL_GAS)
                .deposit(parse_near!("1 yN"))
                .transact()
                .await?;
            // println!("ft_transfer {:#?}\n", res);
        }
    }

    Ok(())
}

pub fn str_to_u128(amount: &str) -> u128 {
    amount.parse::<u128>().unwrap()
}
pub fn str_to_i128(amount: &str) -> i128 {
    amount.parse::<i128>().unwrap()
}

pub async fn get_pool_shares(
    account: &Account,
    exchange: &Contract,
    pool_id: u64,
    worker: &Worker<impl Network>,
) -> anyhow::Result<String> {
    let args = serde_json::json!({ "pool_id": pool_id, "account_id": account.id().to_string()  })
        .to_string()
        .into_bytes();

    let res = exchange.view(worker, "get_pool_shares", args).await?;
    let shares: String = res.json()?;

    println!("debug pool shares: {:#?}", shares);

    Ok(shares)
}

pub async fn get_balance_of(
    account: &Account,
    contract: &Contract,
    is_ft: bool,
    worker: &Worker<impl Network>,
    mft_id: Option<String>,
) -> anyhow::Result<U128> {
    let (function_str, args) = if is_ft {
        (
            "ft_balance_of",
            serde_json::json!({"account_id": account.id()})
                .to_string()
                .into_bytes(),
        )
    } else {
        (
            "mft_balance_of",
            serde_json::json!({"token_id": mft_id.unwrap(), "account_id": account.id()})
                .to_string()
                .into_bytes(),
        )
    };

    let res: U128 = contract.view(worker, function_str, args).await?.json()?;
    Ok(res)
}

#[allow(dead_code)]
pub async fn get_unclaimed_rewards(
    contract: &Contract,
    farm_id_str: &String,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<u128> {
    let unclaimed_amount: U128 = contract
        .call(worker, "get_unclaimed_rewards")
        .args_json(serde_json::json!({ "farm_id_str": farm_id_str }))?
        .gas(TOTAL_GAS)
        .transact()
        .await?
        .json()?;
    Ok(unclaimed_amount.0)
}

#[allow(dead_code)]
pub async fn create_account_and_add_liquidity(
    owner: &Account,
    contract: &Contract,
    exchange: &Contract,
    pool_id: u64,
    token_1: &Contract,
    token_2: &Contract,
    token_id: &String,
    base_liquidity: u128,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<u128> {
    let new_account = worker.dev_create_account().await?;

    register_into_contracts(worker, &new_account, vec![exchange.id()]).await?;

    // Transfer from owner to new account
    transfer_tokens(
        owner,
        vec![&new_account],
        maplit::hashmap! {
            token_1.id() => parse_near!("10,000 N"),
            token_2.id() => parse_near!("10,000 N"),
        },
        worker,
    )
    .await?;

    let res = owner
        .call(worker, exchange.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_id,
            "amounts": maplit::hashmap! {
                token_1.id() => base_liquidity,
                token_2.id() => base_liquidity,
            },
        }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;
    // println!("added liquidity: {:#?}\n", res);
    Ok(0)
}

pub async fn get_user_fft(
    contract: &Contract,
    account: &Account,
    fft_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    let args = serde_json::json!({ "fft_share": fft_id, "account_id": account.id().to_string(), })
        .to_string()
        .into_bytes();
    let res = contract
        .view(worker, "users_fft_share_amount", args)
        .await?;

    let account_shares: u128 = res.json()?;
    Ok(account_shares)
}

pub async fn get_fft_token_by_seed(
    safe_contract: &Contract,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<String> {
    let args = serde_json::json!({ "seed_id": seed_id })
        .to_string()
        .into_bytes();

    let fft_token: String = safe_contract
        .view(worker, "fft_token_seed_id", args)
        .await?
        .json()?;

    Ok(fft_token)
}

pub async fn get_seed_total_amount(
    safe_contract: &Contract,
    seed_id: &String,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u128> {
    let args = serde_json::json!({ "seed_id": seed_id })
        .to_string()
        .into_bytes();

    let seed_before_withdraw = safe_contract
        .view(worker, "seed_total_amount", args)
        .await?
        .json()?;

    Ok(seed_before_withdraw)
}

///////////////////// Jumbo

pub async fn deploy_proxy_contract(
    owner: &Account,
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<Contract> {
    let wasm = fs::read("res/proxy_contract_local.wasm").await?;
    let contract = worker.dev_deploy(&wasm).await?;

    let res = contract
        .call(worker, "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
        }))?
        .transact()
        .await?;

    println!("deploy proxy contract -> {:#?}", res);

    Ok(contract)
}

pub async fn jumbo_deploy_exchange(
    owner: &Account,
    proxy_contract: &Contract,
    tokens: Vec<&AccountId>,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<Contract> {
    println!("here");

    let wasm = fs::read("res/jumbo_exchange_local.wasm").await?;
    let ref_finance = worker.dev_deploy(&wasm).await?;

    println!("here");

    // NOTE: We are not pulling down the contract's data here, so we'll need ot initialize
    // our own set of metadata. This is because the contract's data is too big for the rpc
    // service to pull down (i.e. greater than 50mb).
    let _res = ref_finance
        .call(worker, "new")
        .args_json(serde_json::json!({
            "owner_id": ref_finance.id().clone(),
            "exchange_fee": 4,
            "referral_fee": 1,
            "aml_account_id": proxy_contract.id(),
            "accepted_risk_score": 1u8
        }))?
        .transact()
        .await?;

    println!("debug exchange new: {:#?}", _res);

    let _res = ref_finance
        .call(worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({ "tokens": tokens }))?
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;

    println!("debug exchange extend: {:#?}", _res);
    let _res = owner
        .call(worker, ref_finance.id(), "storage_deposit")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("20 N"))
        .transact()
        .await?;

    println!("debug exchange storage deposit: {:#?}", _res);

    Ok(ref_finance)
}

pub async fn jumbo_create_farm(
    owner: &Account,
    farm: &Contract,
    seed_id: &String,
    token_reward: &Contract,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<(String, u64)> {
    let reward_per_session: String = parse_near!("1000 N").to_string();
    let res = owner
        .call(worker, farm.id(), "create_simple_farm")
        .args_json(serde_json::json!({
            "terms": {
                "seed_id": seed_id,
                "reward_token": token_reward.id(),
                "start_at": 0,
                "reward_per_session": reward_per_session,
                "session_interval": 10
            },
            "min_deposit": Some(U128(MIN_SEED_DEPOSIT))
        }))?
        .deposit(parse_near!("0.1 N"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("create simple farm{:#?}", res);

    let farm_id: String = res.json()?;

    let res = token_reward
        .call(worker, "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": farm.id(),
        }))?
        .deposit(parse_near!("0.00125 N"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("register farm into reward token -> {:#?}", res);

    let amount: String = parse_near!("100000000 N").to_string();

    let res = owner
        .call(worker, token_reward.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": farm.id(),
            "amount": amount,
            "msg": farm_id
        }))?
        .deposit(parse_near!("1 yN"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("ft_transfer_call -> {:#?}", res);

    let id = farm_id.chars().last().unwrap().to_digit(10).unwrap() as u64;

    Ok((farm_id, id))
}

pub async fn jumbo_deploy_farm(
    owner: &Account,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<Contract> {
    let wasm = fs::read("res/jumbo_farming_local.wasm").await?;
    let farm = worker.dev_deploy(&wasm).await?;

    owner
        .call(worker, farm.id(), "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
        }))?
        .transact()
        .await?;

    // TODO: remove if not necessary
    let _res = farm
        .call(worker, "get_metadata")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("0.1 N"))
        .transact()
        .await?;

    // increase reward per session in order to try to swap in the pool for a value that
    // is higher than the pool contains

    // TODO: require farm state is Running

    Ok(farm)
}
