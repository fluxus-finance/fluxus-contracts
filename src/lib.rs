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

mod errors;
use crate::errors::*;

mod auto_compounder;
use auto_compounder::*;

mod actions_of_compounder;

mod views;

mod fluxus_strat;
use fluxus_strat::*;

mod actions_of_strat;

mod owner;

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    Whitelist,
    AccountTokens { account_id: AccountId },
    Shares { pool_id: u64 },
}

// TODO: update this to newer version, following AutoCompounderState
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

    // TODO: do we still need this?
    // Used by storage_impl and account_deposit to keep track of NEAR deposit in this contract
    users_total_near_deposited: HashMap<AccountId, u128>,

    // Contract address of the exchange used
    exchange_contract_id: AccountId,

    // Contract address of the farm used
    farm_contract_id: AccountId,

    // Pools used to harvest, in the ":X" format
    token_ids: Vec<String>,

    // Keeps track of token_id to strategy used
    strategies: HashMap<String, Strategy>,
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
    fn callback_withdraw_rewards(
        &mut self,
        #[callback_result] reward_result: Result<U128, PromiseError>,
        reward_token: String,
        token_id: String,
    ) -> String;
    fn callback_withdraw_shares(
        &mut self,
        token_id: String,
        account_id: AccountId,
        amount: Balance,
    );
    fn callback_get_deposits(&self) -> Promise;
    fn callback_get_return(&self) -> (U128, U128);
    fn callback_stake(&mut self, #[callback_result] shares_result: Result<U128, PromiseError>);
    fn callback_post_get_pool_shares(
        &mut self,
        #[callback_unwrap] minted_shares_result: U128,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        token_id: String,
    );
    fn callback_stake_result(&mut self, token_id: String, account_id: AccountId, shares: u128);
    fn swap_to_auto(&mut self, token_id: String, amount_in_1: U128, amount_in_2: U128);
    fn stake_and_liquidity_auto(
        &mut self,
        #[callback_result] deposits_result: Result<HashMap<AccountId, U128>, PromiseError>,
        token_id: String,
        account_id: AccountId,
    );
    fn balance_update(&mut self, vec: HashMap<AccountId, u128>, shares: String);
    fn get_tokens_return(
        &self,
        token_id: String,
        amount_token_1: U128,
        amount_token_2: U128,
    ) -> Promise;
    fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<(), PromiseError>,
        token_id: String,
        amount: U128,
    ) -> U128;
    fn callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
    ) -> Promise;
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
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
    ) -> Self {
        let mut allowed_accounts: Vec<AccountId> = Vec::new();
        allowed_accounts.push(env::current_account_id());

        Self {
            owner_id: owner_id,
            accounts: LookupMap::new(StorageKey::Accounts),
            allowed_accounts,
            whitelisted_tokens: UnorderedSet::new(StorageKey::Whitelist),
            state: RunningState::Running,
            // TODO: remove this
            users_total_near_deposited: HashMap::new(),
            exchange_contract_id: exchange_contract_id.clone(),
            farm_contract_id: farm_contract_id.clone(),
            /// List of all the pools.
            token_ids: Vec::new(),
            strategies: HashMap::new(),
        }
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
