use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{ext_contract, AccountId};

/// Single swap action.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
    /// Token to swap from.
    pub token_in: AccountId,
    /// Amount to exchange.
    /// If amount_in is None, it will take amount_out from previous step.
    /// Will fail if amount_in is None on the first step.
    pub amount_in: Option<U128>,
    /// Token to swap into.
    pub token_out: AccountId,
    /// Required minimum amount of token_out.
    pub min_amount_out: U128,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

//
#[ext_contract(ext_exchange)]
pub trait RefExchange {
    fn get_deposit(&self, account_id: AccountId, token_id: AccountId) -> U128;
    fn get_deposits(&mut self, account_id: AccountId) -> HashMap<AccountId, U128>;
    fn get_return(
        &self,
        pool_id: u64,
        token_in: AccountId,
        amount_in: U128,
        token_out: AccountId,
    ) -> U128;
    fn register_tokens(&mut self, token_ids: Vec<AccountId>);
    fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<AccountId>) -> Balance;
    fn withdraw(&mut self, token_id: String, amount: U128, unregister: Option<bool>) -> Promise;
}

#[ext_contract(ext_input_token)]
pub trait ExtInputToken {
    fn storage_deposit(&self, account_id: AccountId, registration_only: bool);
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: String, msg: String);
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    /// Returns the balance of the account. If the account doesn't exist, `"0"` must be returned.
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}
