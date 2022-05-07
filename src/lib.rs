use near_sdk::PromiseError;
use std::collections::HashMap;
use std::convert::Into;
use std::convert::TryInto;
use std::fmt;

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
    owner_id: AccountId,
    user_shares: HashMap<AccountId, u128>,
    protocol_shares: u128,
    accounts: LookupMap<AccountId, VAccount>,
    allowed_accounts: Vec<AccountId>,
    whitelisted_tokens: UnorderedSet<AccountId>,
    state: RunningState,
    last_reward_amount: HashMap<String, u128>,
    users_total_near_deposited: HashMap<AccountId, u128>,
    pool_token1: String,
    pool_token2: String,
    pool_id_token1_wrap: u64,
    pool_id_token2_wrap: u64,
    pool_id_token1_reward: u64,
    pool_id_token2_reward: u64,
    reward_token: String,
    wrap_near_contract_id: String,
    exchange_contract_id: String,
    farm_contract_id: String,
    farm: String,
    pool_id: u64,
    seed_id: String,
    shares: LookupMap<AccountId, Balance>,
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
    fn callback_withdraw_shares(&mut self, account_id: AccountId, amount: Balance);
    fn callback_get_deposits(&self) -> Promise;
    fn callback_stake(&mut self);
    fn callback_to_balance(&mut self);
    fn swap_to_auto(&mut self, farm_id: String);
    fn stake_and_liquidity_auto(&mut self, account_id: AccountId);
    fn balance_update(&mut self, vec: HashMap<AccountId, u128>, shares: String);
}

#[near_bindgen]
impl Contract {
    /// Function that initialize the contract.
    ///
    /// Arguments:
    ///
    /// - `owner_id` - the account id that owns the contract
    /// - `protocol_shares` - the number of shares the contract starts/has
    /// - `pool_token1` - First pool token
    /// - `pool_token2` - Second pool token
    /// - `pool_id_token1_wrap` - Pool_id of a pool that is token1-wnear
    /// - `pool_id_token2_wrap` - Pool_id of a pool that is token2-wnear
    /// - `pool_id_token1_reward` - Pool_id of a pool that is token1-reward
    /// - `pool_id_token2_reward` - Pool_id of a pool that is token2-reward
    /// - `reward_token` - Reward given by the farm
    /// - `exchange` - The exchange that will be used to swap tokens and stake
    /// - `farm_id` - The id of the farm to stake
    /// - `pool_id` - The id of the pool to swap tokens
    #[init]
    pub fn new(
        owner_id: AccountId,
        protocol_shares: u128,
        pool_token1: String,
        pool_token2: String,
        pool_id_token1_wrap: u64,
        pool_id_token2_wrap: u64,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: String,
        exchange_contract_id: String,
        farm_contract_id: String,
        wrap_near_contract_id: String,
        farm_id: u64,
        pool_id: u64,
    ) -> Self {
        let farm: String =
            exchange_contract_id.clone() + "@" + &pool_id.to_string() + "#" + &farm_id.to_string();

        let mut last_reward_amount: HashMap<String, u128> = HashMap::new();
        last_reward_amount.insert(farm.clone(), 0);

        let mut allowed_accounts: Vec<AccountId> = Vec::new();
        allowed_accounts.push(env::current_account_id());

        Self {
            owner_id: owner_id,
            user_shares: HashMap::new(),
            last_reward_amount,
            protocol_shares,
            accounts: LookupMap::new(StorageKey::Accounts),
            allowed_accounts,
            whitelisted_tokens: UnorderedSet::new(StorageKey::Whitelist),
            state: RunningState::Running,
            users_total_near_deposited: HashMap::new(),
            pool_token1: pool_token1,
            pool_token2: pool_token2,
            pool_id_token1_wrap: pool_id_token1_wrap,
            pool_id_token2_wrap: pool_id_token2_wrap,
            pool_id_token1_reward: pool_id_token1_reward,
            pool_id_token2_reward: pool_id_token2_reward,
            reward_token: reward_token,
            exchange_contract_id: exchange_contract_id.clone(),
            farm_contract_id: farm_contract_id.clone(),
            wrap_near_contract_id,
            farm,
            pool_id,
            seed_id: exchange_contract_id + "@" + &pool_id.to_string(),
            shares: LookupMap::new(StorageKey::Shares { pool_id: pool_id }),
        }
    }

    #[private]
    pub fn stake(&self, account_id: &AccountId) {
        log!("We are inside stake");
        let pool_id: String = self.wrap_mft_token_id(self.pool_id.to_string());
        self.call_stake(
            self.farm_contract_id.parse().unwrap(),
            pool_id,
            U128(self.shares.get(&account_id).unwrap_or(0)),
            "".to_string(),
        );
    }

    #[private]
    /// wrap token_id into correct format in MFT standard
    pub fn wrap_mft_token_id(&self, token_id: String) -> String {
        format!(":{}", token_id)
    }

    #[private]
    pub fn increment_user_shares(&mut self, account_id: &AccountId, shares: Balance) {
        let user_lps = self.user_shares.get(&account_id);

        let mut prev_shares: Balance = 0;
        if let Some(lps) = user_lps {
            // TODO: improve log
            // log!("");
            prev_shares = *lps;
            self.user_shares
                .insert(account_id.clone(), prev_shares + shares);
        } else {
            // TODO: improve log
            // log!("");
            self.user_shares.insert(account_id.clone(), shares);
        };
    }

    #[private]
    #[payable] // why payable?
    pub fn increment_shares(&mut self, account_id: &AccountId, shares: Balance) {
        //asset that the caller is the vault
        if shares == 0 {
            return;
        }
        //add_to_collection(&mut self.shares, &account_id, shares);
        let prev_value = self.shares.get(account_id).unwrap_or(0);
        log!("Now, {} has {} shares", account_id, prev_value + shares);
        self.shares.insert(account_id, &(prev_value + shares));
    }

    #[private]
    #[payable] // why payable?
    pub fn decrement_shares(&mut self, account_id: &AccountId, shares: Balance) {
        let prev_value = self.shares.get(account_id).unwrap_or(0);
        log!("Now, {} has {} shares", account_id, prev_value - shares);
        self.shares.insert(account_id, &(prev_value - shares));
    }

    #[private]
    pub fn callback_withdraw_shares(&mut self, account_id: AccountId, amount: Balance) {
        // assert!(mft_transfer_result.is_ok());
        let prev_shares = self.user_shares.get(&account_id);
        if let Some(shares) = prev_shares {
            let new_shares = *shares - amount;
            self.user_shares.insert(account_id.clone(), new_shares);
        } else {
            env::panic_str("AutoCompunder:: user shares not found");
        };
    }

    /// Returns the number of shares some accountId has in the contract
    pub fn get_user_shares(&self, account_id: AccountId) -> Option<String> {
        // self.check_caller(account_id.clone());
        let user_shares = self.user_shares.get(&account_id);
        if let Some(account) = user_shares {
            Some(account.to_string())
        } else {
            None
        }
    }

    /// Returns the total amount of near that was deposited
    pub fn user_total_near_deposited(&self, account_id: AccountId) -> Option<String> {
        self.check_caller(account_id.clone());
        let users_total_near_deposited = self.users_total_near_deposited.get(&account_id);
        if let Some(quantity) = users_total_near_deposited {
            Some(quantity.to_string())
        } else {
            None
        }
    }

    /// Function that return the user`s near storage.
    pub fn get_user_storage_state(&self, account_id: AccountId) -> Option<RefStorageState> {
        self.check_caller(account_id.clone());
        let acc = self.internal_get_account(&account_id);
        if let Some(account) = acc {
            Some(RefStorageState {
                deposit: U128(account.near_amount),
                usage: U128(account.storage_usage()),
            })
        } else {
            None
        }
    }

    /// Call the ref get_pool_shares function.
    #[private]
    pub fn call_get_pool_shares(&self, pool_id: u64, account_id: AccountId) -> Promise {
        assert!(self.check_promise(), "Previous tx failed.");
        ext_exchange::get_pool_shares(
            pool_id,
            account_id,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            0,                                          // yocto NEAR to attach
            Gas(10_000_000_000_000),                    // gas to attach
        )
    }

    /// Call the ref user_register function.
    pub fn call_user_register(&self, account_id: AccountId) -> Promise {
        ext_exchange::storage_deposit(
            account_id,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            10000000000000000000000,                    // yocto NEAR to attach
            Gas(3_000_000_000_000),                     // gas to attach
        )
    }

    /// Call the ref get_deposits function.
    fn call_get_deposits(&self, account_id: AccountId) -> Promise {
        ext_exchange::get_deposits(
            account_id,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            1,                                          // yocto NEAR to attach
            Gas(15_000_000_000_000),                    // gas to attach
        )
    }

    /// Update user balances based on the user's percentage in the contract.
    #[private]
    #[payable]
    pub fn balance_update(&mut self, vec: HashMap<AccountId, u128>, shares: String) {
        let new_shares_quantity = shares.parse::<u128>().unwrap();
        log!("new_shares_quantity is equal to {}", new_shares_quantity,);

        let mut total: u128 = 0;
        for (_, val) in vec.clone() {
            total = total + val
        }
        for (account, val) in vec {
            let extra_shares_for_user: u128 =
                // Distribute rewards for users with 10^-10% of the auto_compounder or more 
                (((new_shares_quantity * TEN_TO_THE_POWER_OF_12) as f64 * (val as f64 / total as f64))).floor() as u128 / TEN_TO_THE_POWER_OF_12;
            let new_user_balance = val + extra_shares_for_user;
            self.user_shares.insert(account, new_user_balance);
        }
    }

    /// Ref function to add liquidity in the pool.
    pub fn call_add_liquidity(
        &self,
        pool_id: u64,
        amounts: Vec<U128>,
        min_amounts: Option<Vec<U128>>,
    ) -> Promise {
        ext_exchange::add_liquidity(
            pool_id,
            amounts,
            min_amounts,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            970000000000000000000,                      // yocto NEAR to attach /////////////
            Gas(30_000_000_000_000),                    // gas to attach
        )
    }

    /// Ref function to stake the lps/shares.
    pub fn call_stake(
        &self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        msg: String,
    ) -> Promise {
        ext_exchange::mft_transfer_call(
            receiver_id,
            token_id,
            amount,
            msg,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            1,                                          // yocto NEAR to attach
            Gas(80_000_000_000_000),                    // gas to attach
        )
    }

    /// Ref function to withdraw the rewards to exchange ref contract.
    pub fn call_withdraw_reward(
        &self,
        token_id: String,
        amount: U128,
        unregister: String,
    ) -> Promise {
        ext_farm::withdraw_reward(
            token_id,
            amount,
            unregister,
            self.farm_contract_id.parse().unwrap(), // contract account id
            1,                                      // yocto NEAR to attach
            Gas(180_000_000_000_000),               // gas to attach
        )
    }

    /// Function to return the user's deposit in the auto_compounder contract.
    pub fn get_deposits(&self, account_id: AccountId) -> HashMap<AccountId, U128> {
        let wrapped_account = self.internal_get_account(&account_id);
        if let Some(account) = wrapped_account {
            account
                .get_tokens()
                .iter()
                .map(|token| (token.clone(), U128(account.get_balance(token).unwrap())))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Receives shares from auto-compound and stake it
    #[private]
    pub fn callback_stake(&mut self) {
        assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
        let shares = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(tokens) => {
                if let Ok(shares) = near_sdk::serde_json::from_slice::<String>(&tokens) {
                    shares
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };

        //Concatenate ":" with pool id because ref's testnet contract need an argument like this. Ex -> :193
        //For Mainnet, probability it is not necessary concatenate the ":"
        let pool_id: String = ":".to_string() + &self.pool_id.to_string();

        self.call_stake(
            self.farm_contract_id.parse().unwrap(),
            pool_id,
            U128(shares.parse::<u128>().unwrap()),
            "".to_string(),
        );
    }

    /// Change the user_balance and the auto_compounder balance of lps/shares
    #[private]
    pub fn callback_update_user_balance(
        &mut self,
        account_id: AccountId,
        #[callback_result] shares: Result<String, PromiseError>,
    ) -> String {
        require!(shares.is_ok());

        let protocol_shares_on_pool: u128 = match shares {
            Ok(shares_) => shares_.parse::<u128>().unwrap(),
            _ => env::panic_str("Unknown error occurred"),
        };

        let shares_added_to_pool = protocol_shares_on_pool - self.protocol_shares;
        let user_shares = self.get_user_shares(account_id.clone());

        if user_shares == None {
            self.user_shares.insert(account_id.clone(), 0);
        }

        let mut new_user_balance: u128 = 0;

        if protocol_shares_on_pool > self.protocol_shares {
            if let Some(x) = self.get_user_shares(account_id.clone()) {
                Some(new_user_balance = x.parse::<u128>().unwrap() + shares_added_to_pool)
            } else {
                None
            };
            self.user_shares.insert(account_id, new_user_balance);
            log!("User_shares = {}", new_user_balance);
        };
        self.protocol_shares = protocol_shares_on_pool;

        protocol_shares_on_pool.to_string()
    }

    /// Call the swap function in the exchange. It can be used by itself or as a callback.
    #[private]
    #[payable]
    pub fn call_swap(
        &mut self,
        pool_id_to_swap: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    ) -> Promise {
        assert!(self.check_promise(), "Previous tx failed.");
        ext_exchange::swap(
            vec![SwapAction {
                pool_id: pool_id_to_swap,
                token_in: token_in,
                token_out: token_out,
                amount_in: amount_in,
                min_amount_out: min_amount_out,
            }],
            None,
            self.exchange_contract_id.parse().unwrap(),
            1,
            Gas(20_000_000_000_000),
        )
    }

    #[private]
    pub fn callback_get_deposits(&self) -> Promise {
        assert!(self.check_promise(), "Previous tx failed.");

        let (_, contract_id) = self.get_predecessor_and_current_account();
        ext_exchange::get_deposits(
            contract_id,
            self.exchange_contract_id.parse().unwrap(),
            1,                       // yocto NEAR to attach
            Gas(12_000_000_000_000), // gas to attach
        )
    }

    /// Withdraw user lps and send it to the contract.
    pub fn unstake(&mut self, amount_withdrawal:Option<U128>) -> Promise {
        let (caller_id, contract_id) = self.get_predecessor_and_current_account();
        // TODO
        // require!(ACCOUNT_EXIST)
        let user_lps = self.user_shares.get(&caller_id);

        let mut shares_available: u128 = 0;
        if let Some(lps) = user_lps {
            Some(shares_available = *lps)
        } else {
            None
        };

        assert!(
            shares_available != 0,
            "User does not have enough lps to withdraw"
        );
        let amount = amount_withdrawal.unwrap_or(U128(shares_available));
        log!("Unstake amount = {}", amount.0);
        assert!(
            amount.0 != 0,
            "User is trying to withdraw 0 shares"
        );

        assert!(
            shares_available >= amount.0,
            "User is trying to withdrawal {} and only has {}", amount.0, shares_available
        );

        // Unstake shares/lps
        ext_farm::withdraw_seed(
            self.seed_id.clone(),
            amount.clone(),
            "".to_string(),
            self.farm_contract_id.parse().unwrap(), // contract account id
            1,                                      // yocto NEAR to attach
            Gas(180_000_000_000_000),               // gas to attach 108 -> 180_000_000_000_000
        )
        .then(ext_exchange::mft_transfer(
            self.wrap_mft_token_id(self.pool_id.to_string()),
            caller_id.clone(),
            amount,
            Some("".to_string()),
            self.exchange_contract_id.parse().unwrap(),
            1,
            Gas(50_000_000_000_000),
        ))
        .then(ext_self::callback_withdraw_shares(
            caller_id,
            amount.clone().0,
            contract_id,
            0,
            Gas(20_000_000_000_000),
        ))
    }
}

/// Internal methods implementation.
impl Contract {
    fn assert_contract_running(&self) {
        match self.state {
            RunningState::Running => (),
            _ => env::panic_str("E51: contract paused"),
        };
    }
}
