use crate::*;

#[near_bindgen]
impl Contract {
    /// Returns the number of shares some accountId has in the contract
    pub fn get_user_shares(&self, account_id: AccountId, token_id: String) -> Option<u128> {
        let compounder = self.seeds.get(&token_id).expect("ERR_TOKEN_DOES_NOT_EXIST");
        Some(*compounder.user_shares.get(&account_id).unwrap_or(&0u128))
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

    // pub fn get_seed_min_deposit(&self) -> U128 {
    //     self.seed_min_deposit
    // }

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

    // TODO: remove if compounders wont be used
    // pub fn get_auto_compounders(self) -> Vec<AutoCompounder> {
    //     self.compounders
    // }

    pub fn get_allowed_tokens(self) -> Vec<String> {
        self.token_ids
    }

    pub fn get_compounders(self) -> Vec<AutoCompounderInfo> {
        let mut info: Vec<AutoCompounderInfo> = Vec::new();

        for (token_id, compounder) in self.seeds.clone() {
            info.push(AutoCompounderInfo {
                token_id,
                token1_address: compounder.token1_address,
                token2_address: compounder.token2_address,
                pool_id_token1_reward: compounder.pool_id_token1_reward,
                pool_id_token2_reward: compounder.pool_id_token2_reward,
                reward_token: compounder.reward_token,
                farm: compounder.farm,
                pool_id: compounder.pool_id,
                seed_min_deposit: compounder.seed_min_deposit,
                seed_id: compounder.seed_id,
            })
        }

        info
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AutoCompounderInfo {
    pub token_id: String,
    pub token1_address: AccountId,
    pub token2_address: AccountId,
    pub pool_id_token1_reward: String,
    pub pool_id_token2_reward: String,
    pub reward_token: AccountId,
    pub farm: String,
    pub pool_id: String,
    pub seed_min_deposit: U128,
    pub seed_id: String,
}
