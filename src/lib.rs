use near_sdk::PromiseError;
use std::collections::HashMap;
use std::convert::Into;
use std::convert::TryInto;
use std::fmt;
use uint::construct_uint;

use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, log, near_bindgen, require, AccountId, Balance,
    BorshStorageKey, Gas, PanicOnDefault, Promise, PromiseResult,
};

use crate::account_deposit::{Account, VAccount};
mod account_deposit;
mod auto_compound;
mod storage_impl;
mod token_receiver;

mod external_contracts;
use external_contracts::*;

mod utils;

mod auto_compounder;
use auto_compounder::*;

mod actions_of_compounder;

mod views;

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    Whitelist,
    AccountTokens { account_id: AccountId },
    Shares { pool_id: u64 },
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub enum RunningState {
    Running,
    Paused,
}

impl fmt::Display for RunningState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RunningState::Running => write!(f, "Running"),
            RunningState::Paused => write!(f, "Paused"),
        }
    }
}

const TEN_TO_THE_POWER_OF_12: u128 = 1000000000000;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    // Account address that have authority to update the contract state
    owner_id: AccountId,

    // // Struct that maps addresses to its currents shares added plus the received
    // // from the auto-compound strategy
    // user_shares: HashMap<AccountId, u128>,

    // // Keeps tracks of how much shares the contract gained from the auto-compound
    // protocol_shares: u128,

    // Keeps tracks of accounts that send coins to this contract
    accounts: LookupMap<AccountId, VAccount>,

    // Keeps track of address that have permission to call auto-compound related methods
    allowed_accounts: Vec<AccountId>,

    // Keeps track of tokens that the contracts can receive
    whitelisted_tokens: UnorderedSet<AccountId>,

    // State is used to update the contract to a Paused/Running state
    state: RunningState,

    // Used to keep track of the rewards received from the farm during auto-compound cycle
    // last_reward_amount: u128,

    // TODO: do we still need this?
    // Used by storage_impl and account_deposit to keep track of NEAR deposit in this contract
    users_total_near_deposited: HashMap<AccountId, u128>,

    // Address of the first token used by pool
    // token1_address: String,

    // Address of the token used by the pool
    // token2_address: String,

    // Pool used to swap the reward received by the token used to add liquidity
    // pool_id_token1_reward: u64,

    // Pool used to swap the reward received by the token used to add liquidity
    // pool_id_token2_reward: u64,

    // Address of the reward token given by the farm
    // reward_token: String,

    // Contract address of the exchange used
    exchange_contract_id: String,

    // Contract address of the farm used
    farm_contract_id: String,
    // Farm used to auto-compound
    // farm: String,

    // Pool used to add liquidity and farming
    // pool_id: u64,

    // Format expected by the farm to claim and withdraw rewards
    // seed_id: String,

    // Min LP amount accepted by the farm for stake
    // seed_min_deposit: U128,
    compounders: Vec<AutoCompounder>,
    token_ids: Vec<String>,
    seeds: HashMap<String, AutoCompounder>,
}
// Functions that we need to call like a callback.
#[ext_contract(ext_self)]
pub trait Callbacks {
    fn call_get_pool_shares(&mut self, pool_id: u64, account_id: AccountId) -> String;
    fn call_swap(
        &self,
        pool_id_to_swap: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    );
    fn callback_update_user_balance(&mut self, account_id: AccountId) -> String;
    fn callback_withdraw_rewards(&mut self, token_id: String) -> String;
    fn callback_withdraw_shares(
        &mut self,
        token_id: String,
        account_id: AccountId,
        amount: Balance,
        shares_available: Balance,
    );
    fn callback_get_deposits(&self) -> Promise;
    fn callback_get_return(&self) -> (U128, U128);
    fn callback_stake(&mut self);
    fn callback_to_balance(&mut self);
    fn callback_stake_result(&mut self, token_id: String, account_id: AccountId, shares: u128);
    fn swap_to_auto(&mut self, amount_in_1: U128, amount_in_2: U128);
    fn stake_and_liquidity_auto(&mut self, account_id: AccountId);
    fn balance_update(&mut self, vec: HashMap<AccountId, u128>, shares: String);
    fn get_tokens_return(&self, amount_token_1: U128, amount_token_2: U128) -> Promise;

    // fn stake(&self, token_id: String, account_id: AccountId, shares: u128) -> Promise;
}
const F: u128 = 100000000000000000000000000000; // rename this const

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[near_bindgen]
impl Contract {
    /// Function that initialize the contract.
    ///
    /// Arguments:
    ///
    /// - `owner_id` - the account id that owns the contract
    /// - `protocol_shares` - the number of shares the contract starts/has
    /// - `token1_address` - First pool token
    /// - `token2_address` - Second pool token
    /// - `pool_id_token1_reward` - Pool_id of a pool that is token1-reward
    /// - `pool_id_token2_reward` - Pool_id of a pool that is token2-reward
    /// - `reward_token` - Reward given by the farm
    /// - `exchange` - The exchange that will be used to swap tokens and stake
    /// - `farm_id` - The id of the farm to stake
    /// - `pool_id` - The id of the pool to swap tokens
    #[init]
    pub fn new(
        owner_id: AccountId,
        exchange_contract_id: String,
        farm_contract_id: String,
    ) -> Self {
        let mut allowed_accounts: Vec<AccountId> = Vec::new();
        allowed_accounts.push(env::current_account_id());

        Self {
            owner_id: owner_id,
            // user_shares: HashMap::new(),
            // last_reward_amount: 0u128,
            // protocol_shares,
            accounts: LookupMap::new(StorageKey::Accounts),
            allowed_accounts,
            whitelisted_tokens: UnorderedSet::new(StorageKey::Whitelist),
            state: RunningState::Running,
            // TODO: remove this
            users_total_near_deposited: HashMap::new(),
            // token1_address: token1_address,
            // token2_address: token2_address,
            // pool_id_token1_reward: pool_id_token1_reward,
            // pool_id_token2_reward: pool_id_token2_reward,
            // reward_token: reward_token,
            exchange_contract_id: exchange_contract_id.clone(),
            farm_contract_id: farm_contract_id.clone(),
            // farm,
            // pool_id,
            // seed_id: exchange_contract_id + "@" + &pool_id.to_string(),
            // seed_min_deposit,
            /// List of all the pools.
            compounders: Vec::new(),
            token_ids: Vec::new(),
            seeds: HashMap::new(),
        }
    }
}

/// Internal methods implementation.
#[near_bindgen]
impl Contract {
    pub fn update_contract_state(&mut self, state: RunningState) -> String {
        self.state = state;
        format!("{} is {}", env::current_account_id(), self.state)
    }
}

/// Internal methods that do not rely on blockchain interaction
impl Contract {
    fn assert_contract_running(&self) {
        match self.state {
            RunningState::Running => (),
            _ => env::panic_str("E51: contract paused"),
        };
    }

    /// wrap token_id into correct format in MFT standard
    pub(crate) fn wrap_mft_token_id(&self, token_id: &String) -> String {
        format!(":{}", token_id)
    }
}

// #[cfg(all(test, not(target_arch = "wasm32")))]
// mod tests {
//     use std::hash::Hash;

//     use super::*;
//     use near_sdk::test_utils::VMContextBuilder;
//     use near_sdk::testing_env;

//     fn get_context() -> VMContextBuilder {
//         let mut builder = VMContextBuilder::new();
//         builder
//             .current_account_id(to_account_id("auto_compounder.near"))
//             .signer_account_id(to_account_id("auto_compounder.near"))
//             .predecessor_account_id(to_account_id("auto_compounder.near"));
//         builder
//     }

//     pub fn to_account_id(value: &str) -> AccountId {
//         value.parse().unwrap()
//     }

//     fn create_contract() -> Contract {
//         let contract = Contract::new(
//             to_account_id("auto_compounder.near"),
//             0u128,
//             String::from("eth.near"),
//             String::from("dai.near"),
//             0,
//             1,
//             String::from("usn.near"),
//             String::from(""),
//             String::from(""),
//             0,
//             0,
//             U128(100),
//         );

//         contract
//     }

//     #[test]
//     fn test_balance_update() {
//         let context = get_context();
//         testing_env!(context.build());

//         let mut contract = create_contract();

//         let near: u128 = 1_000_000_000_000_000_000_000_000; // 1 N

//         let acc1 = to_account_id("alice.near");
//         let shares1 = near.clone();

//         let acc2 = to_account_id("bob.near");
//         let shares2 = near.clone() * 3;

//         // add initial balance for accounts
//         contract.user_shares.insert(acc1.clone(), shares1);
//         contract.user_shares.insert(acc2.clone(), shares2);

//         let total_shares: u128 = shares1 + shares2;

//         // distribute shares between accounts
//         contract.balance_update(total_shares, near.to_string());

//         // assert account 1 earned 25% from reward shares
//         let acc1_updated_shares = contract.user_shares.get(&acc1).unwrap();
//         assert_eq!(
//             *acc1_updated_shares, 1250000000000000000000000u128,
//             "ERR_BALANCE_UPDATE"
//         );

//         // assert account 2 earned 75% from reward shares
//         let acc2_updated_shares = contract.user_shares.get(&acc2).unwrap();
//         assert_eq!(
//             *acc2_updated_shares, 3750000000000000000000000u128,
//             "ERR_BALANCE_UPDATE"
//         );
//     }
// }
