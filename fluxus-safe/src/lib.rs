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

use percentage::Percentage;

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

pub mod auto_compounder;
pub use auto_compounder::*;

mod actions_of_compounder;

mod views;

mod fluxus_strat;
use fluxus_strat::*;

mod actions_of_strat;

mod owner;

pub mod admin_fee;
pub use admin_fee::*;

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    Whitelist,
    AccountTokens { account_id: AccountId },
    Shares { pool_id: u64 },
    Guardian,
}

// TODO: update this to newer version, following AutoCompounderState
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
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

// TODO: update to a versionable contract
#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct ContractData {
    // Account address that have authority to update the contract state
    owner_id: AccountId,

    /// Set of guardians.
    guardians: UnorderedSet<AccountId>,

    /// Fees earned by the DAO
    treasury: AccountFee,

    // Keeps tracks of accounts that send coins to this contract
    accounts: LookupMap<AccountId, VAccount>,

    // Keeps track of address that have permission to call auto-compound related methods
    allowed_accounts: Vec<AccountId>,

    // Keeps track of tokens that the contracts can receive
    whitelisted_tokens: UnorderedSet<AccountId>,

    // State is used to update the contract to a Paused/Running state
    state: RunningState,

    // Used by storage_impl and account_deposit to keep track of NEAR deposit in this contract
    users_total_near_deposited: HashMap<AccountId, u128>,

    // Contract address of the exchange used
    exchange_contract_id: AccountId,

    // Contract address of the farm used
    farm_contract_id: AccountId,

    // Pools used to harvest, in the ":X" format
    token_ids: Vec<String>,

    // Keeps track of token_id to strategy used
    strategies: HashMap<String, VersionedStrategy>,
}
// Functions that we need to call like a callback.
#[ext_contract(ext_self)]
pub trait Callbacks {
    fn call_get_pool_shares(&mut self, pool_id: u64, account_id: AccountId) -> String;
    fn call_swap(
        &self,
        pool_id: u64,
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
    fn callback_get_tokens_return(&self) -> (U128, U128);
    fn callback_get_token_return(&self, common_token: u64, amount_token: U128) -> (U128, U128);
    fn callback_stake(
        &mut self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
    );
    fn callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        token_id: String,
    );
    fn callback_stake_result(&mut self, token_id: String, account_id: AccountId, shares: u128);
    fn swap_to_auto(
        &mut self,
        token_id: String,
        amount_in_1: U128,
        amount_in_2: U128,
        common_token: u64,
    );
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
        common_token: u64,
    ) -> Promise;
    fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<U128, PromiseError>,
        token_id: String,
    ) -> Promise;
    fn callback_post_treasury_mft_transfer(
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        token_id: String,
    );
    fn callback_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        token_id: String,
        sentry_id: AccountId,
        amount_earned: u128,
    );
    fn callback_post_claim_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        token_id: String,
    ) -> Promise;
    fn callback_post_first_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        token_id: String,
    );
    fn callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        token_id: String,
        common_token: u64,
    );
    fn callback_post_get_unclaimed_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        token_id: String,
    );
    fn callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
    ) -> Promise;
    fn callback_list_farms_by_seed(
        &self,
        #[callback_result] farms_result: Result<Vec<FarmInfo>, PromiseError>,
        token_id: String,
        farm_id: String,
    ) -> Promise;
    fn callback_post_ft_transfer(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        token_id: String,
    );
}
const F: u128 = 100000000000000000000000000000; // rename this const

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

/// Internal methods that do not rely on blockchain interaction
impl Contract {
    fn assert_contract_running(&self) {
        match self.data().state {
            RunningState::Running => (),
            _ => env::panic_str("E51: contract paused"),
        };
    }
    fn assert_strategy_running(&self, token_id: String) {
        self.assert_contract_running();

        let strat = self.get_strat(&token_id);

        match strat.get().state {
            AutoCompounderState::Running => (),
            _ => env::panic_str("E51: strategy ended"),
        };
    }

    /// wrap token_id into correct format in MFT standard
    pub(crate) fn wrap_mft_token_id(&self, token_id: &String) -> String {
        format!(":{}", token_id)
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    data: VersionedContractData,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    V0001(ContractData),
}

impl VersionedContractData {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        owner_id: AccountId,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        treasure_contract_id: AccountId,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let allowed_accounts: Vec<AccountId> = vec![env::current_account_id()];

        let treasury: AccountFee = AccountFee {
            account_id: treasure_contract_id,
            fee_percentage: 10,
            current_amount: 0u128,
        };

        Self {
            data: VersionedContractData::V0001(ContractData {
                owner_id,
                guardians: UnorderedSet::new(StorageKey::Guardian),
                treasury,
                accounts: LookupMap::new(StorageKey::Accounts),
                allowed_accounts,
                whitelisted_tokens: UnorderedSet::new(StorageKey::Whitelist),
                state: RunningState::Running,
                // TODO: remove this
                users_total_near_deposited: HashMap::new(),
                exchange_contract_id,
                farm_contract_id,
                /// List of all the pools.
                token_ids: Vec::new(),
                strategies: HashMap::new(),
            }),
        }
    }
}

impl Contract {
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::V0001(data) => data,
            _ => unimplemented!(),
        }
    }

    fn data_mut(&mut self) -> &mut ContractData {
        match &mut self.data {
            VersionedContractData::V0001(data) => data,
            _ => unimplemented!(),
        }
    }

    fn exchange_acc(&self) -> AccountId {
        self.data().exchange_contract_id.clone()
    }

    fn farm_acc(&self) -> AccountId {
        self.data().farm_contract_id.clone()
    }

    fn treasure_acc(&self) -> AccountId {
        self.data().treasury.account_id.clone()
    }
}
