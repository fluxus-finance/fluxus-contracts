use crate::*;

#[near_bindgen]
impl Contract {
    /// Returns the number of shares some accountId has in the contract
    /// Panics if token_id does not exist
    pub fn get_user_shares(&self, account_id: AccountId, token_id: String) -> SharesBalance {
        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.clone().get();

        compounder
            .user_shares
            .get(&account_id)
            .unwrap_or(&SharesBalance {
                deposited: 0u128,
                total: 0u128,
            })
            .clone()
    }

    /// Function that return the user`s near storage.
    /// WARN: DEPRECATED
    pub fn get_user_storage_state(&self, account_id: AccountId) -> Option<RefStorageState> {
        self.is_caller(account_id.clone());
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
    /// WARN: DEPRECATED
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

    /// Returns the state of the contract, such as Running, Paused
    pub fn get_contract_state(&self) -> String {
        format!("{} is {}", env::current_account_id(), self.state)
    }

    /// Return the whitelisted tokens.
    /// WARN: DEPRECATED
    pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
        self.whitelisted_tokens.to_vec()
    }

    /// Returns the minimum value accepted for given token_id
    pub fn get_seed_min_deposit(self, token_id: String) -> U128 {
        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();
        compounder.seed_min_deposit
    }

    /// Returns the total amount of near that was deposited
    /// WARN: DEPRECATED
    pub fn user_total_near_deposited(&self, account_id: AccountId) -> Option<String> {
        let users_total_near_deposited = self.users_total_near_deposited.get(&account_id);
        if let Some(quantity) = users_total_near_deposited {
            Some(quantity.to_string())
        } else {
            None
        }
    }

    /// Returns all token ids filtering by running strategies
    pub fn get_allowed_tokens(self) -> Vec<String> {
        self.token_ids
    }

    /// Return all Strategies filtering by running
    pub fn get_strats(self) -> Vec<AutoCompounderInfo> {
        let mut info: Vec<AutoCompounderInfo> = Vec::new();

        for (token_id, strat) in self.strategies.clone() {
            let compounder = strat.get();

            info.push(AutoCompounderInfo {
                state: compounder.state,
                token_id,
                token1_address: compounder.token1_address,
                token2_address: compounder.token2_address,
                pool_id_token1_reward: compounder.pool_id_token1_reward,
                pool_id_token2_reward: compounder.pool_id_token2_reward,
                reward_token: compounder.reward_token,
                farm_id: compounder.farm_id,
                pool_id: compounder.pool_id,
                seed_min_deposit: compounder.seed_min_deposit,
                seed_id: compounder.seed_id,
            })
        }

        info
    }

    pub fn get_strat_state(self, token_id: String) -> AutoCompounderState {
        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();
        compounder.state
    }

    /// Returns exchange and farm contracts
    pub fn get_contract_info(self) -> SafeInfo {
        SafeInfo {
            exchange_address: self.exchange_contract_id,
            farm_address: self.farm_contract_id,
        }
    }

    /// Only get guardians info
    pub fn get_guardians(&self) -> Vec<AccountId> {
        self.guardians.to_vec()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AutoCompounderInfo {
    pub state: AutoCompounderState,
    pub token_id: String,
    pub token1_address: AccountId,
    pub token2_address: AccountId,
    pub pool_id_token1_reward: u64,
    pub pool_id_token2_reward: u64,
    pub reward_token: AccountId,
    pub farm_id: String,
    pub pool_id: u64,
    pub seed_min_deposit: U128,
    pub seed_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SafeInfo {
    pub exchange_address: AccountId,
    pub farm_address: AccountId,
}
