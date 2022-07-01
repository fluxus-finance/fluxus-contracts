use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, require, AccountId, Gas, PanicOnDefault, Promise, PromiseIndex,
};
use near_sdk::{PromiseError, PromiseOrValue};

use std::collections::HashMap;
use std::convert::Into;
use std::fmt;

use percentage::Percentage;

mod external_contracts;
use external_contracts::*;
mod managed_tokens;
mod stakeholders;

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
#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct ContractData {
    // Account address that have authority to update the contract state
    owner_id: AccountId,

    // Keeps track of stakeholders addresses and the percentage they have over the fees
    stakeholders_fees: HashMap<AccountId, u128>,

    stakeholders_amount_available: HashMap<AccountId, u128>,

    // Principal token used to distribute fees
    token_out: AccountId,

    // Maps tokens addresses to the pools with token_out
    token_to_pool: HashMap<AccountId, u64>,

    // Contract address of the exchange used
    exchange_contract_id: AccountId,

    // State is used to update the contract to a Paused/Running state
    state: RunningState,
}
// Functions that we need to call like a callback.
#[ext_contract(ext_self)]
pub trait ExtContract {
    fn callback_register_token(&self, token: AccountId, pool_id: u64) -> String;
    fn get_token_return_and_swap(
        &self,
        #[callback_result] token_balance: Result<U128, PromiseError>,
        token: AccountId,
        pool_id: u64,
    ) -> Promise;
    fn swap(
        &self,
        #[callback_result] amount_out: Result<U128, PromiseError>,
        token_in: AccountId,
        amount_in: U128,
        pool_id: u64,
    ) -> Promise;
    fn callback_post_swap(
        &self,
        #[callback_result] withdraw_result: Result<(), PromiseError>,
    ) -> Promise;
    fn internal_distribute(
        &mut self,
        #[callback_result] withdraw_result: Result<U128, PromiseError>,
        amount: U128,
    ) -> PromiseOrValue<String>;
    fn callback_withdraw(
        &mut self,
        #[callback_result] transfer_result: Result<(), PromiseError>,
        account_id: AccountId,
    ) -> String;
}

#[near_bindgen]
impl Contract {
    /// Function responsible for swapping rewards tokens for the token distributed
    pub fn execute_swaps_and_distribute(&self, token: AccountId) -> Promise {
        // self.assert_contract_running();
        // self.is_owner();

        let token_pool = self.data().token_to_pool.get(&token);
        let mut pool: u64 = 0;

        match token_pool {
            Some(pool_id) => {
                pool = *pool_id;
            }
            _ => env::panic_str("TREASURER::TOKEN_NOT_REGISTERED"),
        };

        ext_exchange::get_deposit(
            env::current_account_id(),
            token.clone(),
            self.exchange_acc(),
            1,
            Gas(9_000_000_000_000),
        )
        .then(ext_self::get_token_return_and_swap(
            token,
            pool,
            env::current_account_id(),
            0,
            Gas(70_000_000_000_000),
        ))
        .then(ext_self::callback_post_swap(
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    /// Get the quotation for amount X of reward_token from exchange
    /// Initiates the swap process
    #[private]
    pub fn get_token_return_and_swap(
        &self,
        #[callback_result] token_balance: Result<U128, PromiseError>,
        token: AccountId,
        pool_id: u64,
    ) -> Promise {
        assert!(token_balance.is_ok(), "TREASURER::COULD_NOT_GET_DEPOSITS");

        let amount_in: U128 = token_balance.unwrap();
        assert_ne!(amount_in, U128(0), "TREASURER::NO_DEPOSIT_AVAILABLE");

        ext_exchange::get_return(
            pool_id,
            token.clone(),
            amount_in,
            self.data().token_out.clone(),
            self.exchange_acc(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(ext_self::swap(
            token,
            amount_in,
            pool_id,
            env::current_account_id(),
            0,
            Gas(40_000_000_000_000),
        ))
    }

    /// Swaps the token received by execute_swaps_and_distribute for token_out
    #[private]
    pub fn swap(
        &self,
        #[callback_result] amount_out: Result<U128, PromiseError>,
        token_in: AccountId,
        amount_in: U128,
        pool_id: u64,
    ) -> Promise {
        assert!(
            amount_out.is_ok(),
            "TREASURER::ERR_COULD_NOT_GET_TOKEN_RETURN"
        );

        let mut min_amount_out: u128;

        if let Ok(s) = amount_out.as_ref() {
            let val: u128 = s.0;
            require!(val > 0u128);
            min_amount_out = val;
        } else {
            env::panic_str("TREASURER::ERR_COULD_NOT_DESERIALIZE_TOKEN")
        }

        ext_exchange::swap(
            vec![SwapAction {
                pool_id: pool_id,
                token_in: token_in,
                token_out: self.data().token_out.clone(),
                amount_in: Some(amount_in),
                min_amount_out: U128(min_amount_out),
            }],
            None,
            self.exchange_acc(),
            1,
            Gas(20_000_000_000_000),
        )
    }

    #[private]
    pub fn callback_post_swap(
        &self,
        #[callback_result] deposit_result: Result<U128, PromiseError>,
    ) -> Promise {
        assert!(deposit_result.is_ok(), "TREASURER::ERR_WITHDRAW_FAILED");

        let amount = deposit_result.unwrap();

        ext_exchange::withdraw(
            self.data().token_out.to_string(),
            amount.clone(),
            Some(false),
            self.exchange_acc(),
            1,
            Gas(60_000_000_000_000),
        )
        .then(ext_self::internal_distribute(
            amount,
            env::current_account_id(),
            0,
            Gas(20_000_000_000_000),
        ))
    }

    #[private]
    pub fn internal_distribute(
        &mut self,
        #[callback_result] withdraw_result: Result<U128, PromiseError>,
        amount: U128,
    ) -> PromiseOrValue<String> {
        assert!(withdraw_result.is_ok(), "TREASURER::ERR_CANNOT_GET_BALANCE");

        let total_amount: u128 = amount.into();

        // keeps the total value distributed to check if it is not greater than total_amount
        let mut total_distributed: u128 = 0;

        // let mut stakeholders_amounts: Vec<Stakeholder> = Vec::new();
        let mut stakeholders_amounts: HashMap<AccountId, u128> = HashMap::new();

        for (account, perc) in self.data().stakeholders_fees.clone() {
            let percent = Percentage::from(perc);
            let amount_received: u128 = percent.apply_to(total_amount);

            assert!(
                amount_received > 0u128,
                "TREASURER::ERR_CANNOT_GET_PERCENTAGE"
            );

            total_distributed += amount_received.clone();

            stakeholders_amounts.insert(account, amount_received);
        }

        // TODO: if this goes wrong, the value was already withdraw and there is no way to distribute again
        assert!(
            total_distributed <= total_amount,
            "TREASURER::ERR_TRIED_TO_DISTRIBUTE_HIGHER_AMOUNT"
        );

        for (acc, amount) in stakeholders_amounts.clone() {
            let prev_amount: &u128 = self
                .data_mut()
                .stakeholders_amount_available
                .get(&acc)
                .unwrap();

            let current_amount: u128 = prev_amount + amount;

            self.data_mut()
                .stakeholders_amount_available
                .insert(acc, current_amount);
        }

        PromiseOrValue::Value("Stakeholders can already withdraw from Treasurer".to_string())
    }

    /// Transfer caller's current available amount from contract to caller
    pub fn withdraw(&self) -> Promise {
        let (caller_id, contract_id) = self.get_predecessor_and_current_account();

        assert!(
            self.data().stakeholders_fees.contains_key(&caller_id),
            "TREASURER::ERR_ACCOUNT_DOES_NOT_EXIST"
        );

        let amount: &u128 = self
            .data()
            .stakeholders_amount_available
            .get(&caller_id)
            .unwrap();

        assert_ne!(*amount, 0u128, "TREASURER::ERR_WITHDRAW_ZERO_AMOUNT");

        ext_input_token::ft_transfer(
            caller_id.clone(),
            U128(*amount),
            Some(String::from("")),
            self.data().token_out.clone(),
            1,
            Gas(100_000_000_000_000),
        )
        .then(ext_self::callback_withdraw(
            caller_id,
            contract_id,
            0,
            Gas(50_000_000_000_000),
        ))
    }

    /// Ensure transfer were successful and set current available amount to zero
    #[private]
    pub fn callback_withdraw(
        &mut self,
        #[callback_result] transfer_result: Result<(), PromiseError>,
        account_id: AccountId,
    ) -> String {
        assert!(
            transfer_result.is_ok(),
            "TREASURER::ERR_WITHDRAW_FROM_CONTRACT_FAILED"
        );

        self.data_mut()
            .stakeholders_amount_available
            .insert(account_id.clone(), 0u128);

        format!("The withdraw from {} was successfully", account_id)
    }

    /// Checks if predecessor_account_id is either the contract or the owner of the contract
    #[private]
    pub fn is_owner(&self) {
        let (caller_acc_id, contract_id) = self.get_predecessor_and_current_account();
        require!(
            caller_acc_id == contract_id || caller_acc_id == self.data().owner_id,
            "TREASURER::ERR_NOT_ALLOWED"
        );
    }

    /// Returns the caller of the execution and the contract address
    #[private]
    pub fn get_predecessor_and_current_account(&self) -> (AccountId, AccountId) {
        (env::predecessor_account_id(), env::current_account_id())
    }

    #[private]
    pub fn assert_contract_running(&self) {
        match self.data().state {
            RunningState::Running => (),
            _ => env::panic_str("TREASURER::CONTRACT_PAUSED"),
        };
    }

    pub fn update_contract_state(&mut self, state: RunningState) -> String {
        self.data_mut().state = state;
        format!("{} is {}", env::current_account_id(), self.data().state)
    }

    pub fn get_contract_state(&self) -> String {
        format!("{} is {}", env::current_account_id(), self.data().state)
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
    /// Function that initialize the contract.
    ///
    /// Arguments:
    ///
    /// - `owner_id` - The account id that owns the contract
    /// - `token_out` - Token address, used to distribute fees between stakeholders
    /// - `exchange_contract_id` - The exchange that will be used to swap tokens
    #[init]
    pub fn new(owner_id: AccountId, token_out: AccountId, exchange_contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let allowed_accounts: Vec<AccountId> = vec![env::current_account_id()];

        Self {
            data: VersionedContractData::V0001(ContractData {
                owner_id,
                stakeholders_fees: HashMap::new(),
                stakeholders_amount_available: HashMap::new(),
                token_out,
                token_to_pool: HashMap::new(),
                state: RunningState::Running,
                exchange_contract_id,
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
}
