//! Account deposit is information per user about their balances in the exchange.

use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::collections::UnorderedMap;
use std::collections::HashMap;

use crate::token_receiver::ext_self;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, Balance, Gas, PromiseResult, StorageUsage,
};

use crate::*;

// [AUDIT_01]
// const MAX_ACCOUNT_LENGTH: u128 = 64;
// const MAX_ACCOUNT_BYTES: u128 = MAX_ACCOUNT_LENGTH + 4;
// const MIN_ACCOUNT_DEPOSIT_LENGTH: u128 = 1 + MAX_ACCOUNT_BYTES + 16 + 4;
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(20_000_000_000_000);
pub const GAS_FOR_FT_TRANSFER: Gas = Gas(20_000_000_000_000);

const U128_STORAGE: StorageUsage = 16;
const U64_STORAGE: StorageUsage = 8;
const U32_STORAGE: StorageUsage = 4;
/// max length of account id is 64 bytes. We charge per byte.
const ACC_ID_STORAGE: StorageUsage = 64;
/// As a key, 4 bytes length would be added to the head
const ACC_ID_AS_KEY_STORAGE: StorageUsage = ACC_ID_STORAGE + 4;
const KEY_PREFIX_ACC: StorageUsage = 64;
/// As a near_sdk::collection key, 1 byte for prefix
const ACC_ID_AS_CLT_KEY_STORAGE: StorageUsage = ACC_ID_AS_KEY_STORAGE + 1;

// ACC_ID: the Contract accounts map key length
// + VAccount enum: 1 byte
// + U128_STORAGE: near_amount storage
// + U32_STORAGE: legacy_tokens HashMap length
// + U32_STORAGE: tokens HashMap length
// + U64_STORAGE: storage_used
pub const INIT_ACCOUNT_STORAGE: StorageUsage =
    ACC_ID_AS_CLT_KEY_STORAGE + 1 + U128_STORAGE + U32_STORAGE + U32_STORAGE + U64_STORAGE;

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VAccount {
    //V1(AccountV1),
    Current(Account),
}

impl VAccount {
    /// Upgrades from other versions to the currently used version.
    pub fn into_current(self) -> Account {
        match self {
            VAccount::Current(account) => account,
            //VAccount::V1(account) => account.into_current(account_id),
        }
    }
}

impl From<Account> for VAccount {
    fn from(account: Account) -> Self {
        VAccount::Current(account)
    }
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Account {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub legacy_tokens: HashMap<AccountId, Balance>,
    pub tokens: UnorderedMap<AccountId, Balance>,
    pub storage_used: StorageUsage,
}

impl Account {

    /// Create a new account structure.
    /// # Parameters example: 
    ///   account_id: account.testnet
    pub fn new(account_id: &AccountId) -> Self {
        Account {
            near_amount: 0,
            legacy_tokens: HashMap::new(),
            tokens: UnorderedMap::new(StorageKey::AccountTokens {
                account_id: account_id.clone(),
            }),
            storage_used: 0,
        }
    }

    /// Get the balance of a specific token for an account.
    /// # Parameters example: 
    ///   token_id: token1.testnet
    pub fn get_balance(&self, token_id: &AccountId) -> Option<Balance> {
        if let Some(token_balance) = self.tokens.get(token_id) {
            Some(token_balance)
        } else if let Some(legacy_token_balance) = self.legacy_tokens.get(token_id) {
            Some(*legacy_token_balance)
        } else {
            None
        }
    }

    /// Return a vector with all tokens.
    pub fn get_tokens(&self) -> Vec<AccountId> {
        let mut a: Vec<AccountId> = self.tokens.keys().collect();
        let b: Vec<AccountId> = self.legacy_tokens.keys().cloned().collect();
        a.extend(b);
        a
    }

    /// Deposit amount to the balance of given token,
    /// if given token not register and not enough storage, deposit fails
    /// # Parameters example: 
    ///   token: token1.testnet
    ///   amount: 100000000
    pub(crate) fn deposit_with_storage_check(
        &mut self,
        token: &AccountId,
        amount: Balance,
    ) -> bool {
        if let Some(balance) = self.tokens.get(token) {
            // token has been registered, just add without storage check,
            let new_balance = balance + amount;
            self.tokens.insert(token, &new_balance);
            true
        } else if let Some(x) = self.legacy_tokens.get_mut(token) {
            // token has been registered, just add without storage check
            *x += amount;
            true
        } else {
            // check storage after insert, if fail should unregister the token
            self.tokens.insert(token, &(amount));
            if self.storage_usage() <= self.near_amount {
                true
            } else {
                self.tokens.remove(token);
                false
            }
        }
    }

    /// Deposit amount to the balance of given token.
    /// # Parameters example: 
    ///   token: token1.testnet
    ///   amount: 100000000
    pub(crate) fn deposit(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.legacy_tokens.remove(token) {
            // need convert to tokens
            self.tokens.insert(token, &(amount + x));
        } else if let Some(x) = self.tokens.get(token) {
            self.tokens.insert(token, &(amount + x));
        } else {
            self.tokens.insert(token, &amount);
        }
    }

    /// Withdraw amount of `token` from the internal balance.
    /// Panics if `amount` is bigger than the current balance.
    /// # Parameters example: 
    ///   token: token1.testnet
    ///   amount: 100000000
    pub(crate) fn withdraw(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.legacy_tokens.remove(token) {
            // need convert to
            assert!(x >= amount, "{}", "E22: not enough tokens in deposit");
            self.tokens.insert(token, &(x - amount));
        } else if let Some(x) = self.tokens.get(token) {
            assert!(x >= amount, "{}", "E22: not enough tokens in deposit");
            self.tokens.insert(token, &(x - amount));
        } else {
            env::panic_str("E21: token not registered");
        }
    }

    // [AUDIT_01]
    /// Returns amount of $NEAR necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (INIT_ACCOUNT_STORAGE
            + self.legacy_tokens.len() as u64 * (ACC_ID_AS_KEY_STORAGE + U128_STORAGE)
            + self.tokens.len() as u64 * (KEY_PREFIX_ACC + ACC_ID_AS_KEY_STORAGE + U128_STORAGE))
            as u128
            * env::storage_byte_cost()
    }

    /// Returns how much NEAR is available for storage.
    pub fn storage_available(&self) -> Balance {
        // [AUDIT_01] avoid math overflow
        let locked = self.storage_usage();
        if self.near_amount > locked {
            self.near_amount - locked
        } else {
            0
        }
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    pub fn assert_storage_usage(&self) {
        assert!(
            self.storage_usage() <= self.near_amount,
            "{}",
            "E11: insufficient $NEAR storage deposit"
        );
    }

    /// Returns minimal account deposit storage usage possible.
    pub fn min_storage_usage() -> Balance {
        INIT_ACCOUNT_STORAGE as Balance * env::storage_byte_cost()
    }

    /// Registers given token and set balance to 0.
    /// # Parameters example: 
    ///   token_ids: [token1.testnet, token2.testnet]
    pub(crate) fn register(&mut self, token_ids: &Vec<AccountId>) {
        for token_id in token_ids {
            let t = token_id;
            if self.get_balance(t).is_none() {
                self.tokens.insert(t, &0);
            }
        }
    }

    /// Unregister `token_id` from this account balance.
    /// Panics if the `token_id` balance is not 0.
    /// # Parameters example: 
    ///   token_id: token1.testnet 
    pub(crate) fn unregister(&mut self, token_id: &AccountId) {
        let amount = self.legacy_tokens.remove(token_id).unwrap_or_default();
        assert_eq!(amount, 0, "{}", "E24: non-zero token balance");
        let amount = self.tokens.remove(token_id).unwrap_or_default();
        assert_eq!(amount, 0, "{}", "E24: non-zero token balance");
    }
}

#[near_bindgen]
impl Contract {
    /// Registers given token in the user's account deposit.
    /// Fails if not enough balance on this account to cover storage.
    /// # Parameters example: 
    ///   token_ids: [token1.testnet, token2.testnet]
    #[payable]
    pub fn register_tokens(&mut self, token_ids: Vec<AccountId>) {
        assert_one_yocto();
        self.assert_contract_running();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        account.register(&token_ids);
        self.internal_save_account(&sender_id, account);
    }

    /// Unregister given token from user's account deposit.
    /// Panics if the balance of any given token is non 0.
    /// # Parameters example: 
    ///   token_ids: [token1.testnet, token2.testnet]
    #[payable]
    pub fn unregister_tokens(&mut self, token_ids: Vec<AccountId>) {
        assert_one_yocto();
        self.assert_contract_running();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        for token_id in token_ids {
            account.unregister(&token_id);
        }
        self.internal_save_account(&sender_id, account);
    }

    /// Withdraws given token from the deposits of given user.
    /// Optional unregister will try to remove record of this token from AccountDeposit for given user.
    /// Unregister will fail if the left over balance is non 0.
    /// # Parameters example: 
    ///   token_id: token1.testnet
    ///   amount: U128(100000000)
    ///   unregister: True or None
    #[payable]
    pub fn withdraw(
        &mut self,
        token_id: AccountId,
        amount: U128,
        unregister: Option<bool>,
    ) -> Promise {
        assert_one_yocto();
        self.assert_contract_running();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        assert!(amount > 0, "{}", "E29: Illegal withdraw amount");
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        // Note: subtraction and deregistration will be reverted if the promise fails.
        account.withdraw(&token_id, amount);
        if unregister == Some(true) {
            account.unregister(&token_id);
        }
        self.internal_save_account(&sender_id, account);
        self.internal_send_tokens(&sender_id, &token_id, amount)
    }

    /// Check if the transfer succeeded.
    /// # Parameters example: 
    ///   token_id: token1.testnet
    ///   sender_id: sender.testnet
    ///   amount: U128(100000000)
    #[private]
    pub fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    ) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            "E25: expected 1 promise result from withdraw"
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                // This reverts the changes from withdraw function.
                // If account doesn't exit, deposits to the owner's account as lostfound.
                let mut failed = false;
                if let Some(mut account) = self.internal_get_account(&sender_id) {
                    if account.deposit_with_storage_check(&token_id, amount.0) {
                        // cause storage already checked, here can directly save
                        self.data_mut().accounts.insert(&sender_id, &account.into());
                    } else {
                        // we can ensure that internal_get_account here would NOT cause a version upgrade,
                        // cause it is callback, the account must be the current version or non-exist,
                        // so, here we can just leave it without insert, won't cause storage collection inconsistency.
                        env::log_str(
                            format!(
                                "Account {} has not enough storage. Depositing to owner.",
                                sender_id
                            )
                            .as_ref(),
                        );
                        failed = true;
                    }
                } else {
                    env::log_str(
                        format!(
                            "Account {} is not registered. Depositing to owner.",
                            sender_id
                        )
                        .as_ref(),
                    );
                    failed = true;
                }
                if failed {
                    self.internal_lostfound(&token_id, amount.0);
                }
            }
        };
    }
}

impl Contract {
    /// Checks that account has enough storage to be stored and saves it into collection.
    /// This should be only place to directly use `self.accounts`.
    /// # Parameters example:
    ///   account_id: account.testnet,
    ///   account: {near_amount: 100000000, legacy_tokens: {token_id.testnet: 100000000}, tokens: {token_id2.testnet: 100000000}, storage_used: 1000000}
    pub(crate) fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
        account.assert_storage_usage();
        self.data_mut()
            .accounts
            .insert(&account_id, &account.into());
    }

    /// save token to owner account as lostfound, no need to care about storage
    /// only global whitelisted token can be stored in lost-found
    /// # Parameters example: 
    ///   token_id: token1.testnet
    ///   amount: 100000000
    pub(crate) fn internal_lostfound(&mut self, token_id: &AccountId, amount: u128) {
        
        if self.data().whitelisted_tokens.contains(token_id) {
            let mut lostfound = self.internal_unwrap_or_default_account(&self.data().owner_id);
            lostfound.deposit(token_id, amount);
            let owner_id = self.data().owner_id.clone();
            self.data_mut()
                .accounts
                .insert(&owner_id, &lostfound.into());
        } else {
            env::panic_str("ERR: non-whitelisted token can NOT deposit into lost-found.");
        }
    }

    /// Registers account in deposited amounts with given amount of $NEAR.
    /// If account already exists, adds amount to it.
    /// This should be used when it's known that storage is prepaid.
    /// # Parameters example: 
    ///   token_id: token1.testnet
    ///   amount: 100000000
    pub(crate) fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_or_default_account(&account_id);
        log!(
            "account.near_amount = {} + amount = {} == {}",
            account.near_amount,
            amount,
            account.near_amount + amount
        );
        account.near_amount += amount;
        self.internal_save_account(&account_id, account);
    }

    // pub(crate) fn internal_register_account_sub(
    //     &mut self,
    //     account_id: &AccountId,
    //     amount: Balance,
    // ) {
    //     let mut account = self.internal_unwrap_or_default_account(&account_id);
    //     log!(
    //         "account.near_amount is = {} and amount = {}",
    //         account.near_amount,
    //         amount
    //     );
    //     account.near_amount -= amount;
    //     log!(
    //         "the new balance after subtracting = {}",
    //         account.near_amount
    //     );
    //     self.internal_save_account(&account_id, account);
    // }

    /// Decrement the available amount of an account.
    /// # Parameters example: 
    ///   account_id: account.testnet
    ///   amount: 100000000
    pub(crate) fn internal_storage_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: Balance,
    ) -> u128 {
        let mut account = self.internal_unwrap_account(&account_id);
        let available = account.storage_available();
        assert!(available > 0, "ERR_NO_STORAGE_CAN_WITHDRAW");
        let mut withdraw_amount = amount;
        if amount == 0 {
            withdraw_amount = available;
        }
        assert!(
            withdraw_amount <= available,
            "ERR_STORAGE_WITHDRAW_TOO_MUCH"
        );
        account.near_amount -= withdraw_amount;
        self.internal_save_account(&account_id, account);
        withdraw_amount
    }

    /// Record deposit of some number of tokens to this contract.
    /// Fails if account is not registered or if token isn't whitelisted.
    /// # Parameters example: 
    ///   sender_id: account.testnet
    ///   token_id: token1.testnet
    ///   amount: 100000000
    pub(crate) fn internal_deposit(
        &mut self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) {
        let mut account = self.internal_unwrap_account(sender_id);
        assert!(
            self.data().whitelisted_tokens.contains(token_id)
                || account.get_balance(token_id).is_some(),
            "{}",
            "E12: token not whitelisted"
        );
        account.deposit(token_id, amount);
        self.internal_save_account(&sender_id, account);
    }

    /// Return a Option<Account> based on an account_id
    /// # Parameters example: 
    ///   account_id: account.testnet
    pub fn internal_get_account(&self, account_id: &AccountId) -> Option<Account> {
        self.data()
            .accounts
            .get(account_id)
            .map(|va| va.into_current())
    }

    /// Return an Account based on an account_id
    /// # Parameters example: 
    ///   account_id: account.testnet
    pub fn internal_unwrap_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .expect("E27: attach 1yN to swap tokens not in whitelist")
    }

    /// Return an Account based on an account_id or create a new one.
    /// # Parameters example: 
    ///   account_id: account.testnet
    pub fn internal_unwrap_or_default_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .unwrap_or_else(|| Account::new(account_id))
    }

    /*
    /// Returns current balance of given token for given user. If there is nothing recorded, returns 0.
    pub(crate) fn internal_get_deposit(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        self.internal_get_account(sender_id)
            .and_then(|x| x.get_balance(token_id))
            .unwrap_or(0)
    }*/
    /// Sends given amount to given user and if it fails, returns it back to user's balance.
    /// Tokens must already be subtracted from internal balance.
    /// # Parameters example: 
    ///   sender_id: account.testnet
    ///   token_id: token.testnet
    ///   amount: 10000000
    pub(crate) fn internal_send_tokens(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) -> Promise {
        ext_fungible_token::ft_transfer(
            sender_id.clone(),
            U128(amount),
            None,
            token_id.clone(),
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::exchange_callback_post_withdraw(
            token_id.clone(),
            sender_id.clone(),
            U128(amount),
            env::current_account_id(),
            0,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(to_account_id("auto_compounder.near"))
            .signer_account_id(to_account_id("auto_compounder.near"))
            .predecessor_account_id(to_account_id("auto_compounder.near"));
        builder
    }

    pub fn to_account_id(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn create_account() -> Account {
        let account_struct = Account::new(&to_account_id("fluxus.near"));
        account_struct
    }

    #[test]
    fn test_account_initialization() {
        let context = get_context();
        testing_env!(context.build());

        let mut account = create_account();

        // Initialized account has none token registered
        assert_eq!(account.get_tokens(), vec![]);
        account.register(&vec![to_account_id("usn.near")]);
        assert_eq!(account.get_tokens(), vec![to_account_id("usn.near")]);

        // Register token and check current balance, defaults to 0
        let usn_balance: Option<Balance> = account.get_balance(&to_account_id("usn.near"));
        assert_eq!(usn_balance.unwrap_or(1u128), 0u128);
    }

    #[test]
    fn test_account_deposit_and_withdraw() {
        let context = get_context();
        testing_env!(context.build());

        let mut account = create_account();

        // Initialized account has none token registered
        assert_eq!(account.get_tokens(), vec![]);
        account.register(&vec![to_account_id("usn.near")]);
        assert_eq!(account.get_tokens(), vec![to_account_id("usn.near")]);

        // Deposit balance to token
        account.deposit(&to_account_id("usn.near"), 10u128);
        let usn_balance: Option<Balance> = account.get_balance(&to_account_id("usn.near"));
        assert_eq!(usn_balance.unwrap_or(1u128), 10u128);

        // deposit_with_storage_check withdraw
        account.withdraw(&to_account_id("usn.near"), 10u128);
        let usn_balance: Option<Balance> = account.get_balance(&to_account_id("usn.near"));
        assert_eq!(usn_balance.unwrap_or(1u128), 0u128);
    }

    #[test]
    fn test_account_register_unregister() {
        let context = get_context();
        testing_env!(context.build());

        let mut account = create_account();

        // Register and unregister token without modifying its balance
        account.register(&vec![to_account_id("usn.near")]);
        account.register(&vec![to_account_id("wnear.near")]);
        assert_eq!(
            account.get_tokens(),
            vec![to_account_id("usn.near"), to_account_id("wnear.near")]
        );
        let wnear_balance: Option<Balance> = account.get_balance(&to_account_id("wnear.near"));
        assert_eq!(wnear_balance.unwrap_or(1u128), 0u128);
        account.unregister(&to_account_id("wnear.near"));
        assert_eq!(account.get_tokens(), vec![to_account_id("usn.near")]);
    }

    #[test]
    fn test_account_deposit_with_balance() {
        let context = get_context();
        testing_env!(context.build());

        let mut account = create_account();

        // assert no storage is available until near is added
        assert_eq!(account.storage_available(), 0u128);
        account.deposit_with_storage_check(&to_account_id("uxu.near"), 10u128);
        let fft_balance: Option<Balance> = account.get_balance(&to_account_id("uxu.near"));
        assert_eq!(fft_balance.unwrap_or(0u128), 0u128);

        account.near_amount = 100000000000000000000000u128;
        assert_ne!(account.storage_available(), 0u128);

        // deposit token with balance
        account.deposit_with_storage_check(&to_account_id("uxu.near"), 10u128);
        let fft_balance: Option<Balance> = account.get_balance(&to_account_id("uxu.near"));
        assert_eq!(fft_balance.unwrap_or(1u128), 10u128);
    }
}
