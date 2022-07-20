use near_sdk::PromiseError;
use std::collections::HashMap;
use std::collections::HashSet;
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
    BorshStorageKey, Gas, PanicOnDefault, Promise, PromiseResult,PromiseOrValue
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

mod multi_fungible_token;

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    Whitelist,
    AccountTokens { account_id: AccountId },
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

    // Keeps tracks of how much shares the contract gained from the auto-compound
    protocol_shares: u128,

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


    ///It is a map that store the uxu_share and a map of users and their balance. 
    /// illustration: map(uxu_share[i], map(user[i], balance[i])).
    /// TODO: Change HashMap for LookupMap as it is more gas efficient
    users_balance_by_uxu_share: HashMap<String, HashMap<String, u128>>,

    ///Store the auto-compounders of the seeds.
    /// illustration: map( seed[i], vec(user[i]) ).//TODO
    compounders_by_seed_id: HashMap<String, HashSet<String>>,

    ///Store the uxu_share total_supply for each seed_id. 
    /// TODO: Change HashMap for LookupMap as it is more gas efficient
    total_supply_by_uxu_share:  HashMap<String, u128>,
    
    ///Store the uxu_share for each seed_id. 
    /// TODO: Change HashMap for LookupMap as it is more gas efficient
    uxu_share_by_seed_id:  HashMap<String, String>,

    ///Store the uxu_share for each seed_id. 
    seed_id_amount:  HashMap<String, u128>,

    // Contract address of the exchange used
    //TODO: Move it inside the strategy
    exchange_contract_id: AccountId,

    // Contract address of the farm used
    //TODO: Move it inside the strategy
    farm_contract_id: AccountId,

    // Contract address to receive earned shares from fee
    treasure_contract_id: AccountId,

    // Pools used to harvest, in the ":X" format
    //TODO: Move it inside the strategy
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
        fft_shares: Balance,
    );
    fn callback_get_deposits(&self) -> Promise;
    fn callback_get_tokens_return(&self) -> (U128, U128);
    fn callback_get_token_return(&self, common_token: u64, amount_token: U128) -> (U128, U128);
    fn callback_stake(&mut self, #[callback_result] shares_result: Result<U128, PromiseError>);
    fn callback_post_get_pool_shares(
        &mut self,
        #[callback_unwrap] minted_shares_result: U128,
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
    fn get_tokens_return(
        &self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
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
    fn callback_post_mft_transfer(
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        token_id: String,
    );
    fn callback_post_claim_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        token_id: String,
    ) -> Promise;
    fn callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        token_id: String,
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
}

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

        Self {
            data: VersionedContractData::V0001(ContractData {
                owner_id,
                guardians: UnorderedSet::new(StorageKey::Guardian),
                protocol_shares: 0u128,
                accounts: LookupMap::new(StorageKey::Accounts),
                allowed_accounts,
                whitelisted_tokens: UnorderedSet::new(StorageKey::Whitelist),
                state: RunningState::Running,
                users_total_near_deposited: HashMap::new(),
                users_balance_by_uxu_share: HashMap::new(),
                compounders_by_seed_id: HashMap::new(),
                total_supply_by_uxu_share: HashMap::new(),
                uxu_share_by_seed_id: HashMap::new(),
                seed_id_amount: HashMap::new(),
                exchange_contract_id,
                farm_contract_id,
                treasure_contract_id,
                /// List of all the pools.
                token_ids: Vec::new(),
                strategies: HashMap::new(),
            }),
        }
    }
}

impl Contract {
    #[allow(unreachable_patterns)]
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::V0001(data) => data,
            _ => unimplemented!(),
        }
    }
    #[allow(unreachable_patterns)]
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
        self.data().treasure_contract_id.clone()
    }
}
