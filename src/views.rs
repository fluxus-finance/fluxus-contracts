use crate::*;

#[near_bindgen]
impl Contract {
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

    pub fn get_contract_state(&self) -> String {
        format!("{} is {}", env::current_account_id(), self.state)
    }

    /// Returns allowed_accounts
    pub fn get_allowed_accounts(&self) -> Vec<AccountId> {
        self.check_permission();
        self.allowed_accounts.clone()
    }

    /// Return the whitelisted tokens.
    pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
        self.whitelisted_tokens.to_vec()
    }

    pub fn get_seed_min_deposit(&self) -> U128 {
        self.seed_min_deposit
    }

    /// Returns the total amount of near that was deposited
    /// TODO: remove this if not necessary
    pub fn user_total_near_deposited(&self, account_id: AccountId) -> Option<String> {
        let users_total_near_deposited = self.users_total_near_deposited.get(&account_id);
        if let Some(quantity) = users_total_near_deposited {
            Some(quantity.to_string())
        } else {
            None
        }
    }
}
