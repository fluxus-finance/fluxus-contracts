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
    BorshStorageKey, Gas, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
};

use percentage::Percentage;

use crate::account_deposit::{Account, VAccount};
mod account_deposit;
mod storage_impl;
mod token_receiver;

mod external_contracts;
pub use external_contracts::*;

pub mod utils;
use utils::*;

mod errors;
use crate::errors::*;

pub mod auto_compounder;
pub use auto_compounder::*;
mod auto_compound;

pub mod stable_auto_compounder;
pub use stable_auto_compounder::*;
mod stable_auto_compound;

pub mod jumbo_auto_compounder;
pub use jumbo_auto_compounder::*;
mod jumbo_auto_compound;

mod actions_of_compounder;

mod views;

mod fluxus_strat;
use fluxus_strat::*;

mod actions_of_strat;

mod owner;

pub mod admin_fee;
pub use admin_fee::*;

pub mod callback;
use callback::*;

mod multi_fungible_token;

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    Whitelist,
    AccountTokens { account_id: AccountId },
    Guardian,
    NearDeposited,
    UsersBalanceByShare,
    TotalSupplyByShare,
    SeedIdAmount,
    SeedRegister { fft_share: String },
    Strategy { fft_share_id: String },
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
    users_total_near_deposited: LookupMap<AccountId, u128>,

    ///It is a map that store the fft_share and a map of users and their balance.
    /// illustration: map(fft_share[i], map(user[i], balance[i])).
    users_balance_by_fft_share: LookupMap<String, LookupMap<String, u128>>,

    ///Store the fft_share total_supply for each seed_id.
    total_supply_by_fft_share: LookupMap<String, u128>,

    ///Store the fft_share for each seed_id.
    /// TODO: Change HashMap for LookupMap as it is more gas efficient
    fft_share_by_seed_id: HashMap<String, String>,

    ///Store the fft_share for each seed_id.
    seed_id_amount: LookupMap<String, u128>,

    // Keeps track of token_id to strategy used
    strategies: HashMap<String, VersionedStrategy>,
}

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
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
    pub fn new(owner_id: AccountId, treasure_contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let allowed_accounts: Vec<AccountId> = vec![env::current_account_id()];

        let treasury: AccountFee = AccountFee {
            account_id: treasure_contract_id,
            fee_percentage: 10, //TODO: the treasury fee_percentage can be removed from here as the treasury contract will receive all the fees amount that won't be sent to strat_creator or sentry
            // The breakdown of amount for Stakers, operations and treasury will be dealt with inside the treasury contract
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
                users_total_near_deposited: LookupMap::new(StorageKey::NearDeposited),
                users_balance_by_fft_share: LookupMap::new(StorageKey::UsersBalanceByShare),
                total_supply_by_fft_share: LookupMap::new(StorageKey::TotalSupplyByShare),
                fft_share_by_seed_id: HashMap::new(),
                seed_id_amount: LookupMap::new(StorageKey::SeedIdAmount),
                /// List of all the pools.
                /// TODO: with more exchanges, this should not exist
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

    fn assert_contract_running(&self) {
        match self.data().state {
            RunningState::Running => (),
            _ => env::panic_str("E51: contract paused"),
        };
    }

    /// Ensures that at least one strategy is running for given token_id
    fn assert_strategy_is_running(&self, seed_id: &str) {
        let strat = self.get_strat(seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(_) => {
                let compounder = strat.get_compounder_ref();

                for farm in compounder.farms.iter() {
                    if farm.state == AutoCompounderState::Running {
                        return;
                    }
                }
            }
            VersionedStrategy::StableAutoCompounder(_) => {
                let compounder = strat.get_stable_compounder_ref();

                for farm in compounder.farms.iter() {
                    if farm.state == AutoCompounderState::Running {
                        return;
                    }
                }
            }
            VersionedStrategy::JumboAutoCompounder(_) => {
                let compounder = strat.get_jumbo();

                for farm in compounder.farms.iter() {
                    if farm.state == JumboAutoCompounderState::Running {
                        return;
                    }
                }
            }
        }

        panic!("There is no running strategy for this pool")
    }
}
