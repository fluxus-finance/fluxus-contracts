use near_sdk::json_types::U128;
use near_units::{parse_gas, parse_near};
use std::collections::HashMap;
use tokio::fs;
use workspaces::network::Sandbox;
use workspaces::prelude::*;
use workspaces::{prelude::*, testnet, DevNetwork};
use workspaces::{Account, AccountId, Contract, Network, Worker};
const TOTAL_GAS: u64 = 300_000_000_000_000;

type FarmId = String;
type SeedId = String;
use near_sdk::serde::{Deserialize, Serialize};

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

const CONTRACT_ID_WNEAR_TESTNET: &str = "wrap.testnet";
const CONTRACT_ID_REF_EXC: &str = "exchange.ref-dev.testnet";
const CONTRACT_ID_FARM: &str = "farm.leopollum.testnet";
const FT_CONTRACT_FILEPATH: &str = "./res/fungible_token.wasm";

pub async fn deploy_vault_contract(worker: &Worker<impl DevNetwork>) -> anyhow::Result<Contract> {
    let wasm = fs::read("res/auto_compounder.wasm").await?;
    let contract = worker.dev_deploy(&wasm).await?;

    contract
        .call(&worker, "new_without_pools")
        .args_json(serde_json::json!({
            "owner_id": contract.id().clone(),
        }))?
        .transact()
        .await?;

    Ok(contract)
}

pub async fn deploy_full_vault_contract(
    token1: &Contract,
    token2: &Contract,
    reward_token: &Contract,
    pool_id_token1_wrap: u64,
    pool_id_token2_wrap: u64,
    pool_id_token1_reward: u64,
    pool_id_token2_reward: u64,
    pool_id: u64,
    farm_id: u64,
    worker: &Worker<impl DevNetwork>,
) -> anyhow::Result<Contract> {
    let wasm = fs::read("res/auto_compounder.wasm").await?;
    let contract = worker.dev_deploy(&wasm).await?;
    let wnear_id: AccountId = CONTRACT_ID_WNEAR_TESTNET.parse().unwrap();

    contract
        .call(&worker, "new")
        .args_json(serde_json::json!({
            "owner_id": contract.id().clone(),
            "protocol_shares": 0u128,
            "pool_token1": token1.id().to_string(),
            "pool_token2": token2.id().to_string(),
            "pool_id_token1_wrap": pool_id_token1_wrap,
            "pool_id_token2_wrap": pool_id_token2_wrap,
            "pool_id_token1_reward": pool_id_token1_reward,
            "pool_id_token2_reward": pool_id_token2_reward,
            "reward_token": reward_token.id().to_string(),
            "exchange_contract_id": CONTRACT_ID_REF_EXC,
            "farm_contract_id": CONTRACT_ID_FARM,
            "wrap_near_contract_id": CONTRACT_ID_WNEAR_TESTNET,
            "farm_id": farm_id,
            "pool_id": pool_id,
        }))?
        .transact()
        .await?;

    /* Register tokens into vault */
    contract
        .call(&worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({
            "tokens":
                vec![
                    wnear_id.clone(),
                    token1.id().clone(),
                    token2.id().clone(),
                    reward_token.id().clone(),
                ]
        }))?
        .transact()
        .await?;

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
        .call(&worker, "new")
        .args_json(serde_json::json!({
            "owner_id": ref_finance.id().clone(),
            "exchange_fee": 4,
            "referral_fee": 1,
        }))?
        .transact()
        .await?;
    let wnear_id: AccountId = CONTRACT_ID_WNEAR_TESTNET.parse().unwrap();

    ref_finance
        .call(worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({ "tokens": tokens }))?
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;

    owner
        .call(&worker, ref_finance_id, "storage_deposit")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("20 N"))
        .transact()
        .await?;

    ref_finance
        .as_account()
        .call(&worker, ref_finance_id, "storage_deposit")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("20 N"))
        .transact()
        .await?;

    Ok(ref_finance)
}

/// Pull down the WNear contract from mainnet and initialize it with our own metadata.
pub async fn deploy_wnear(owner: &Account, worker: &Worker<Sandbox>) -> anyhow::Result<Contract> {
    let testnet = workspaces::testnet().await?;

    let wnear_id: AccountId = CONTRACT_ID_WNEAR_TESTNET.parse().unwrap();
    let wnear = worker
        .import_contract(&wnear_id, &testnet)
        .transact()
        .await?;

    owner
        .call(&worker, wnear.id(), "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("1,000,000,000 N"),
        }))?
        .transact()
        .await?;

    owner
        .call(&worker, wnear.id(), "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": owner.id().clone()
        }))?
        .deposit(parse_near!("0.00125 N"))
        .transact()
        .await?;

    owner
        .call(&worker, wnear.id(), "near_deposit")
        .deposit(parse_near!("50 N"))
        .transact()
        .await?;

    Ok(wnear)
}

pub async fn deploy_farm(
    owner: &Account,
    seed_id: &String,
    token_reward: &Contract,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<Contract> {
    let testnet = workspaces::testnet().await?;

    let farm_acc: AccountId = CONTRACT_ID_FARM.parse().unwrap();

    let farm = worker
        .import_contract(&farm_acc, &testnet)
        .transact()
        .await?;

    owner
        .call(&worker, farm.id(), "new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
        }))?
        .transact()
        .await?;

    // TODO: remove if not necessary
    let _res = farm
        .call(&worker, "get_metadata")
        .args_json(serde_json::json!({}))?
        .deposit(parse_near!("0.1 N"))
        .transact()
        .await?;

    let reward_per_session: String = parse_near!("0.1 N").to_string();
    owner
        .call(&worker, farm.id(), "create_simple_farm")
        .args_json(serde_json::json!({
            "terms": {
                "seed_id": seed_id,
                "reward_token": token_reward.id(),
                "start_at": 0,
                "reward_per_session": reward_per_session,
                "session_interval": 60
            }
        }))?
        .deposit(parse_near!("0.1 N"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;

    let res = token_reward
        .call(&worker, "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": farm.id(),
        }))?
        .deposit(parse_near!("0.00125 N"))
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    println!("register farm into reward token -> {:#?}", res);

    let farm_id = format!("{}#0", seed_id);
    let amount: String = parse_near!("20 N").to_string();
    let res = owner
        .call(&worker, token_reward.id(), "ft_transfer_call")
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

    // TODO: require farm state is Running

    Ok(farm)
}

pub async fn log_farm_info(
    farm: &Contract,
    seed_id: &String,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<()> {
    let farm_id = format!("{}#{}", seed_id, 0);
    let res = farm
        .call(&worker, "get_farm")
        .args_json(serde_json::json!({ "farm_id": farm_id }))?
        .transact()
        .await?;
    // TODO: require tx success

    let info: FarmInfo = res.json().unwrap();
    println!("result {:#?}", info);

    Ok(())
}

/// Create a liquidity pool on Ref-Finance, registering the tokens we provide it.
/// Add's the amount in `tokens` we set for liquidity. This will return us the
/// pool_id after the pool has been created.
pub async fn create_pool_with_liquidity(
    owner: &Account,
    ref_finance: &Contract,
    tokens: HashMap<&AccountId, u128>,
    worker: &Worker<impl Network>,
) -> anyhow::Result<u64> {
    let (token_ids, token_amounts): (Vec<String>, Vec<String>) = tokens
        .iter()
        .map(|(id, amount)| (id.to_string(), amount.to_string()))
        .unzip();

    let res = ref_finance
        .call(worker, "extend_whitelisted_tokens")
        .args_json(serde_json::json!({ "tokens": token_ids }))?
        .deposit(parse_near!("1 yN"))
        .transact()
        .await?;

    // println!("exchange.extend_whitelisted_tokens {:#?}\n", res);

    let pool_id: u64 = ref_finance
        .call(worker, "add_simple_pool")
        .args_json(serde_json::json!({
            "tokens": token_ids,
            "fee": 25
        }))?
        .deposit(parse_near!("4 mN"))
        .transact()
        .await?
        .json()?;

    // println!("pool_id == {}\n", pool_id);

    let token_id: String = ":".to_string() + &pool_id.to_string();
    let farm_acc: AccountId = CONTRACT_ID_FARM.parse().unwrap();

    let register = ref_finance
        .call(worker, "mft_register")
        .args_json(serde_json::json!({
            "token_id": token_id,
            "account_id": farm_acc
        }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    // println!("register == {:#?}\n", register);

    let res = owner
        .call(&worker, ref_finance.id(), "register_tokens")
        .args_json(serde_json::json!({
            "token_ids": token_ids,
        }))?
        .deposit(1)
        .transact()
        .await?;

    // println!("register_tokens is {:#?}\n", res);

    deposit_tokens(worker, owner, &ref_finance, tokens).await?;

    let res = owner
        .call(&worker, ref_finance.id(), "add_liquidity")
        .args_json(serde_json::json!({
            "pool_id": pool_id,
            "amounts": token_amounts,
        }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;
    // println!("added liquidity: {:#?}\n", res);

    let res = ref_finance
        .call(&worker, "get_pool")
        .args_json(serde_json::json!({ "pool_id": pool_id }))?
        .transact()
        .await?;

    // println!("get pool {:#?}\n", res);

    Ok(pool_id)
}

pub async fn create_pools(
    owner: &Account,
    exchange: &Contract,
    token_1: &Contract,
    token_2: &Contract,
    token_reward: &Contract,
    wnear: &Contract,
    worker: &Worker<impl Network>,
) -> anyhow::Result<((u64, u64, u64, u64, u64))> {
    let pool_token1_token2 = create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_1.id() => parse_near!("10 N"),
            token_2.id() => parse_near!("10 N"),
        },
        &worker,
    )
    .await?;

    let pool_token1_wnear = create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_1.id() => parse_near!("10 N"),
            wnear.id() => parse_near!("10 N"),
        },
        &worker,
    )
    .await?;

    let pool_token2_wnear = create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_2.id() => parse_near!("10 N"),
            wnear.id() => parse_near!("10 N"),
        },
        &worker,
    )
    .await?;

    let pool_token1_reward = create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_1.id() => parse_near!("10 N"),
            token_reward.id() => parse_near!("10 N"),
        },
        &worker,
    )
    .await?;

    let pool_token2_reward = create_pool_with_liquidity(
        &owner,
        &exchange,
        maplit::hashmap! {
            token_2.id() => parse_near!("10 N"),
            token_reward.id() => parse_near!("10 N"),
        },
        &worker,
    )
    .await?;
    Ok((
        pool_token1_token2,
        pool_token1_wnear,
        pool_token2_wnear,
        pool_token1_reward,
        pool_token2_reward,
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
    ft.call(&worker, "new_default_meta")
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
    ref_finance: &Contract,
    tokens: HashMap<&AccountId, u128>,
) -> anyhow::Result<()> {
    for (contract_id, amount) in tokens {
        let res = owner
            .call(&worker, contract_id, "ft_transfer_call")
            .args_json(serde_json::json!({
                "receiver_id": ref_finance.id(),
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
        account
            .call(&worker, &contract_id, "storage_deposit")
            .args_json(serde_json::json!({
                "registration_only": true,
            }))?
            .deposit(parse_near!("0.00125 N"))
            .transact()
            .await?;
    }

    Ok(())
}

pub async fn get_pool_info(
    worker: &Worker<impl Network>,
    ref_finance: &Contract,
    pool_id: u64,
) -> anyhow::Result<()> {
    let res = ref_finance
        .call(&worker, "get_pool")
        .args_json(serde_json::json!({ "pool_id": pool_id }))?
        .transact()
        .await?;

    println!("get pool {:#?}\n", res);

    Ok(())
}
